use std::collections::HashMap;

use chromiumoxide::Page;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchMouseEventParams, DispatchMouseEventType, MouseButton,
};
use futures_util::StreamExt;
use tokio::task::JoinHandle;

use crate::browser::config::LynxConfig;
use crate::error::LynxError;
use crate::snapshot;
use crate::types::{InstanceId, InstanceInfo};

/// Maps element refs (e0, e1...) to backend DOM node IDs.
/// Ref indices are assigned by the JS snapshot walker (data-lynx-ref stamps)
/// and inserted here for validation. Actions use querySelector, not BackendNodeId.
pub struct RefMap {
    ref_to_node: HashMap<String, chromiumoxide::cdp::browser_protocol::dom::BackendNodeId>,
}

impl Default for RefMap {
    fn default() -> Self {
        Self::new()
    }
}

impl RefMap {
    pub fn new() -> Self {
        Self {
            ref_to_node: HashMap::new(),
        }
    }

    /// Insert a ref_id assigned by the JS snapshot walker.
    pub fn insert(
        &mut self,
        ref_id: String,
        node_id: chromiumoxide::cdp::browser_protocol::dom::BackendNodeId,
    ) {
        self.ref_to_node.insert(ref_id, node_id);
    }

    pub fn resolve(
        &self,
        ref_id: &str,
    ) -> Option<&chromiumoxide::cdp::browser_protocol::dom::BackendNodeId> {
        self.ref_to_node.get(ref_id)
    }
}

pub struct BrowserInstance {
    pub id: InstanceId,
    pub profile: String,
    #[allow(dead_code)]
    pub browser: Browser,
    pub page: Page,
    pub ref_map: RefMap,
    pub last_snapshot: Option<Vec<crate::types::SnapshotNode>>,
    pub snapshot_version: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub headless: bool,
    pub block_images: bool,
    handler: JoinHandle<()>,
}

impl BrowserInstance {
    pub async fn launch(
        config: &LynxConfig,
        profile: &str,
        headless: bool,
        block_images: bool,
    ) -> Result<Self, LynxError> {
        let profile_dir = config.profile_dir.join(profile);
        std::fs::create_dir_all(&profile_dir)
            .map_err(|e| LynxError::Browser(format!("Failed to create profile dir: {e}")))?;

        // Clean up stale lock files from unclean Chrome exits
        for lock_file in &["SingletonLock", "SingletonCookie", "SingletonSocket"] {
            let lock_path = profile_dir.join(lock_file);
            if lock_path.exists() {
                tracing::info!("Removing stale {lock_file} from profile '{profile}'");
                let _ = std::fs::remove_file(&lock_path);
            }
        }

        let mut builder = BrowserConfig::builder()
            .chrome_executable(&config.chrome_path)
            .user_data_dir(&profile_dir)
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-blink-features=AutomationControlled")
            .arg("--force-renderer-accessibility");

        if headless {
            builder = builder.arg("--headless=new").window_size(1920, 1080);
        } else {
            // Use a large window_size — Chrome clamps to the available screen.
            // 3840x2160 ensures the viewport fills any Mac display at native scaling.
            builder = builder.with_head().window_size(3840, 2160);
        }

        if block_images {
            builder = builder.arg("--blink-settings=imagesEnabled=false");
        }

        let browser_config = builder
            .build()
            .map_err(|e| LynxError::Browser(format!("Config build failed: {e}")))?;

        let (browser, mut handler) = Browser::launch(browser_config)
            .await
            .map_err(|e| LynxError::Browser(format!("Chrome launch failed: {e}")))?;

        // Spawn the CDP handler as a background task
        let handler_task =
            tokio::spawn(async move { while let Some(_event) = handler.next().await {} });

        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| LynxError::Browser(format!("New page failed: {e}")))?;

        // chromiumoxide defaults the CDP viewport to 800x600 regardless of window_size.
        // Use Emulation.setDeviceMetricsOverride to make the viewport fill the window.
        if !headless {
            // Query actual screen dimensions (CSS logical pixels)
            let screen_dims: String = page
                .evaluate(r#"JSON.stringify({w: screen.availWidth, h: screen.availHeight})"#)
                .await
                .ok()
                .and_then(|v| v.into_value().ok())
                .unwrap_or_else(|| r#"{"w":1920,"h":1080}"#.to_string());

            let v: serde_json::Value = serde_json::from_str(&screen_dims)
                .unwrap_or(serde_json::json!({"w":1920,"h":1080}));
            let w = v["w"].as_i64().unwrap_or(1920);
            // Subtract ~80px for Chrome UI (tabs + address bar + info bar)
            let h = v["h"].as_i64().unwrap_or(1080) - 80;

            let _ = page
                .execute(SetDeviceMetricsOverrideParams::new(w, h, 0, false))
                .await;
        }

        let id = uuid::Uuid::new_v4().to_string();

        Ok(Self {
            id,
            profile: profile.to_string(),
            browser,
            page,
            ref_map: RefMap::new(),
            last_snapshot: None,
            snapshot_version: 0,
            created_at: chrono::Utc::now(),
            headless,
            block_images,
            handler: handler_task,
        })
    }

    /// Check if the browser instance is still alive.
    /// Returns false if the CDP handler has finished (Chrome disconnected/crashed).
    pub fn is_alive(&self) -> bool {
        !self.handler.is_finished()
    }

    pub fn info(&self) -> InstanceInfo {
        InstanceInfo {
            id: self.id.clone(),
            profile: self.profile.clone(),
            url: String::new(), // URL fetched async separately
            created_at: self.created_at.to_rfc3339(),
            headless: self.headless,
            status: if self.is_alive() {
                "alive".to_string()
            } else {
                "dead".to_string()
            },
        }
    }

    /// Ensure the instance is alive before performing operations.
    /// Returns a clear error instead of cryptic "send failed because receiver is gone".
    fn check_alive(&self) -> Result<(), LynxError> {
        if self.is_alive() {
            Ok(())
        } else {
            Err(LynxError::Browser(format!(
                "Instance '{}' is dead (Chrome process exited). Destroy and recreate it.",
                self.id
            )))
        }
    }

    async fn current_url(&self) -> String {
        self.page.url().await.ok().flatten().unwrap_or_default()
    }

    async fn current_title(&self) -> String {
        self.page
            .get_title()
            .await
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    pub async fn navigate(&mut self, url: &str, wait_ms: u64) -> Result<String, LynxError> {
        self.check_alive()?;
        self.page
            .goto(url)
            .await
            .map_err(|e| LynxError::Navigation {
                url: url.to_string(),
                reason: e.to_string(),
            })?;

        // Wait for accessibility tree to populate
        tokio::time::sleep(tokio::time::Duration::from_millis(wait_ms)).await;

        let title = self.current_title().await;
        let current_url = self.current_url().await;

        // Auto-detect if user intervention is needed (login forms, CAPTCHAs)
        if let Some(reason) = self.detect_intervention_needed().await {
            tracing::info!("Intervention needed on {current_url}: {reason}");
            self.show_system_notification(&reason);
            self.inject_intervention_banner(&reason).await;
            return Ok(format!(
                "Navigated to: {current_url}\nTitle: {title}\n⚠️ Intervention required: {reason}"
            ));
        }

        Ok(format!("Navigated to: {current_url}\nTitle: {title}"))
    }

    /// Detect if the current page requires user intervention (login, CAPTCHA, etc.)
    async fn detect_intervention_needed(&self) -> Option<String> {
        let js = r#"(function() {
            var signals = [];

            // Check for visible password input
            var pwInputs = document.querySelectorAll('input[type="password"]');
            for (var i = 0; i < pwInputs.length; i++) {
                if (pwInputs[i].offsetParent !== null) {
                    signals.push('password_field');
                    break;
                }
            }

            // Check for login-like username/email fields
            var textInputs = document.querySelectorAll('input[type="text"], input[type="email"], input:not([type])');
            for (var i = 0; i < textInputs.length; i++) {
                var el = textInputs[i];
                var name = ((el.name || '') + (el.id || '') + (el.placeholder || '') + (el.getAttribute('aria-label') || '')).toLowerCase();
                if (/username|user.?name|email|login|sign.?in|account/i.test(name) && el.offsetParent !== null) {
                    signals.push('login_field');
                    break;
                }
            }

            // Check for CAPTCHA
            if (document.querySelector('.g-recaptcha, .h-captcha, iframe[src*="recaptcha"], iframe[src*="hcaptcha"], [class*="captcha" i], [id*="captcha" i]')) {
                signals.push('captcha');
            }

            // Check for verification code / MFA prompts
            var bodyText = (document.body.innerText || '').substring(0, 3000).toLowerCase();
            if (/verification code|verify your identity|enter.{0,20}code|two.?factor|2fa|multi.?factor|mfa|one.?time.?pass|sent.{0,30}(text|sms|email|phone)/i.test(bodyText)) {
                // Confirm there's an input for the code
                var codeInputs = document.querySelectorAll('input[type="text"], input[type="tel"], input[type="number"], input:not([type])');
                for (var i = 0; i < codeInputs.length; i++) {
                    var ci = codeInputs[i];
                    var ciName = ((ci.name || '') + (ci.id || '') + (ci.placeholder || '') + (ci.getAttribute('aria-label') || '')).toLowerCase();
                    if (/code|token|otp|verify|pin/i.test(ciName) && ci.offsetParent !== null) {
                        signals.push('verification_code');
                        break;
                    }
                }
                // Also flag if the page text strongly suggests verification even without a named input
                if (!signals.includes('verification_code') && /enter.{0,10}(the |your )?(code|pin)/i.test(bodyText)) {
                    signals.push('verification_code');
                }
            }

            // Check page title/URL for login indicators
            var titleUrl = (document.title + ' ' + location.href).toLowerCase();
            if (/login|log.in|sign.in|signin|authenticate|verification/i.test(titleUrl)) {
                signals.push('login_page');
            }

            if (signals.length === 0) return null;

            // Build reason (most specific first)
            if (signals.includes('captcha')) return 'CAPTCHA detected — please solve it manually.';
            if (signals.includes('verification_code')) return 'Verification code required — please check your phone/email and enter the code.';
            if (signals.includes('password_field') || (signals.includes('login_field') && signals.includes('login_page'))) {
                return 'Login form detected — please sign in manually.';
            }
            if (signals.includes('login_page') && signals.includes('login_field')) {
                return 'Login page detected — please sign in manually.';
            }
            return null;
        })()"#;

        let result: Option<String> = self
            .page
            .evaluate(js)
            .await
            .ok()
            .and_then(|v| v.into_value().ok());

        result
    }

    /// Fire a native macOS notification via osascript
    fn show_system_notification(&self, message: &str) {
        let escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            r#"display notification "{escaped}" with title "Lynx MCP" subtitle "User Intervention Required" sound name "Ping""#
        );
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .spawn();
    }

    /// Inject the bottom intervention banner (without polling — fire-and-forget)
    async fn inject_intervention_banner(&self, message: &str) {
        let escaped_msg = message.replace('\\', "\\\\").replace('\'', "\\'");
        let inject_js = format!(
            r#"(function() {{
                var old = document.getElementById('__lynx_intervention');
                if (old) old.remove();
                var banner = document.createElement('div');
                banner.id = '__lynx_intervention';
                banner.style.cssText = 'position:fixed;bottom:0;left:0;right:0;z-index:999999;background:#e94560;color:white;padding:12px 20px;font-family:-apple-system,BlinkMacSystemFont,sans-serif;font-size:15px;display:flex;align-items:center;justify-content:space-between;box-shadow:0 -4px 12px rgba(0,0,0,0.3);';
                banner.innerHTML = '<span>\u{{1f511}} <strong>User intervention required</strong> \u{{2014}} {escaped_msg}</span><button id="__lynx_dismiss" style="background:white;color:#e94560;border:none;padding:8px 20px;border-radius:6px;font-size:14px;cursor:pointer;font-weight:600;margin-left:16px;white-space:nowrap;">Done \u{{2713}}</button>';
                document.body.appendChild(banner);
                document.getElementById('__lynx_dismiss').addEventListener('click', function() {{
                    banner.remove();
                    window.__lynx_intervention_dismissed = true;
                }});
                window.__lynx_intervention_dismissed = false;
            }})()"#
        );
        let _ = self.page.evaluate(inject_js).await;
    }

    pub async fn snapshot(
        &mut self,
        filter: Option<&str>,
        diff: bool,
        format: &str,
        selector: Option<&str>,
        max_tokens: Option<usize>,
    ) -> Result<String, LynxError> {
        self.check_alive()?;
        let interactive_only = filter == Some("interactive");

        let (nodes, ref_map, next_ref) =
            snapshot::tree::build_snapshot(&self.page, interactive_only, selector).await?;

        self.ref_map = ref_map;
        self.snapshot_version = next_ref;

        // Handle diff
        let diff_summary = if diff {
            if let Some(ref prev) = self.last_snapshot {
                Some(snapshot::diff::compute_diff(prev, &nodes))
            } else {
                Some("First snapshot — no diff available".to_string())
            }
        } else {
            None
        };

        self.last_snapshot = Some(nodes.clone());

        let output = match format {
            "compact" => {
                let mut lines = Vec::new();
                snapshot::compact::render_compact(&nodes, &mut lines);
                let mut text = lines.join("\n");
                if let Some(max) = max_tokens {
                    let char_limit = max * 4; // ~4 chars per token
                    if text.len() > char_limit {
                        text.truncate(char_limit);
                        text.push_str("\n... (truncated)");
                    }
                }
                if let Some(ref diff) = diff_summary {
                    format!("{diff}\n---\n{text}")
                } else {
                    text
                }
            }
            _ => {
                // Full JSON
                let result = crate::types::SnapshotResult {
                    url: self.current_url().await,
                    title: self.current_title().await,
                    interactive_refs: nodes.iter().filter(|n| n.interactive).count(),
                    total_refs: nodes.len(),
                    nodes,
                    diff_summary,
                    snapshot_version: self.snapshot_version,
                };
                serde_json::to_string_pretty(&result)
                    .map_err(|e| LynxError::Snapshot(e.to_string()))?
            }
        };

        Ok(output)
    }

    pub async fn text(&self, max_tokens: usize) -> Result<String, LynxError> {
        self.check_alive()?;
        let text: String = self
            .page
            .evaluate("document.body.innerText")
            .await
            .map_err(|e| LynxError::JsEval(e.to_string()))?
            .into_value()
            .map_err(|e| LynxError::JsEval(format!("{e:?}")))?;

        let char_limit = max_tokens * 4;
        if text.len() > char_limit {
            Ok(format!("{}... (truncated)", &text[..char_limit]))
        } else {
            Ok(text)
        }
    }

    pub async fn click(&mut self, ref_id: &str) -> Result<String, LynxError> {
        self.check_alive()?;
        self.ref_map
            .resolve(ref_id)
            .ok_or_else(|| LynxError::ElementNotFound(ref_id.to_string()))?;

        // First attempt
        match self.click_inner(ref_id).await {
            Ok(msg) => return Ok(msg),
            Err(LynxError::ElementNotFound(_)) => {
                tracing::info!("click {ref_id} missed — dismissing overlays and retrying");
            }
            Err(e) => return Err(e),
        }

        // Dismiss overlays that may be blocking the element
        let _ = self.dismiss_overlays().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Retry once
        self.click_inner(ref_id).await
    }

    /// Single click attempt — locate element by ref, CDP mouse click at center.
    async fn click_inner(&self, ref_id: &str) -> Result<String, LynxError> {
        // Find element by data-lynx-ref attribute (stamped during snapshot)
        let locate_js = format!(
            r#"(function() {{
                var el = document.querySelector('[data-lynx-ref="{ref_id}"]');
                if (!el) return 'not_found';
                // Walk up to nearest interactive ancestor if this is an inner element
                var target = el;
                var btn = el.closest('button, a, [role="button"], [role="link"], [role="menuitem"]');
                if (btn && btn !== el && btn.contains(el)) {{
                    target = btn;
                }}
                target.scrollIntoView({{block: 'center'}});
                var rect = target.getBoundingClientRect();
                return JSON.stringify({{
                    x: rect.left + rect.width / 2,
                    y: rect.top + rect.height / 2,
                    tag: target.tagName,
                    text: (target.textContent || '').substring(0, 40).trim()
                }});
            }})()"#
        );

        let locate_result: String = self
            .page
            .evaluate(locate_js)
            .await
            .map_err(|e| LynxError::Browser(format!("click locate failed: {e}")))?
            .into_value()
            .map_err(|e| LynxError::Browser(format!("click locate parse failed: {e:?}")))?;

        if locate_result == "not_found" {
            return Err(LynxError::ElementNotFound(format!(
                "{ref_id} not found in DOM"
            )));
        }

        let coords: serde_json::Value = serde_json::from_str(&locate_result)
            .map_err(|e| LynxError::Browser(format!("click coords parse failed: {e}")))?;
        let x = coords["x"].as_f64().unwrap_or(0.0);
        let y = coords["y"].as_f64().unwrap_or(0.0);
        let tag = coords["tag"].as_str().unwrap_or("?");
        let text = coords["text"].as_str().unwrap_or("");

        // CDP trusted mouse click at element center
        // Produces isTrusted:true events that React/Angular respect
        let mut mouse_down =
            DispatchMouseEventParams::new(DispatchMouseEventType::MousePressed, x, y);
        mouse_down.button = Some(MouseButton::Left);
        mouse_down.click_count = Some(1);

        self.page
            .execute(mouse_down)
            .await
            .map_err(|e| LynxError::Browser(format!("click mousedown failed: {e}")))?;

        let mut mouse_up =
            DispatchMouseEventParams::new(DispatchMouseEventType::MouseReleased, x, y);
        mouse_up.button = Some(MouseButton::Left);
        mouse_up.click_count = Some(1);

        self.page
            .execute(mouse_up)
            .await
            .map_err(|e| LynxError::Browser(format!("click mouseup failed: {e}")))?;

        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        Ok(format!("Clicked {ref_id} ({tag}:{text})"))
    }

    pub async fn type_text(
        &mut self,
        ref_id: &str,
        text: &str,
        clear_first: bool,
    ) -> Result<String, LynxError> {
        self.check_alive()?;
        let _backend_id = *self
            .ref_map
            .resolve(ref_id)
            .ok_or_else(|| LynxError::ElementNotFound(ref_id.to_string()))?;

        let escaped_text = text
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n");
        let clear_flag = if clear_first { "true" } else { "false" };

        // Find element by data-lynx-ref (stamped during snapshot), focus, type
        // Single async eval so focus isn't lost between calls
        let type_js = format!(
            r#"(async function() {{
                var el = document.querySelector('[data-lynx-ref="{ref_id}"]');
                if (!el) return 'not_found';
                var tag = el.tagName;

                el.scrollIntoView({{block: 'center'}});
                el.focus();
                el.click();

                if ({clear_flag}) {{
                    if (tag === 'INPUT' || tag === 'TEXTAREA') {{
                        el.select();
                    }} else {{
                        var sel = window.getSelection();
                        var range = document.createRange();
                        range.selectNodeContents(el);
                        sel.removeAllRanges();
                        sel.addRange(range);
                    }}
                    document.execCommand('delete', false);
                }}

                // Yield to let React/framework process focus event
                await new Promise(function(r) {{ setTimeout(r, 50); }});

                // Strategy 1: execCommand insertText (React-compatible)
                var ok = document.execCommand('insertText', false, '{escaped_text}');
                var v = el.value || el.textContent || '';
                if (ok && v.indexOf('{escaped_text}') >= 0) {{
                    return 'typed:execCommand';
                }}

                // Strategy 2: nativeInputValueSetter (React onChange trigger)
                if (tag === 'INPUT' || tag === 'TEXTAREA') {{
                    var proto = tag === 'INPUT'
                        ? window.HTMLInputElement.prototype
                        : window.HTMLTextAreaElement.prototype;
                    var nativeSet = Object.getOwnPropertyDescriptor(proto, 'value');
                    if (nativeSet && nativeSet.set) {{
                        nativeSet.set.call(el, '{escaped_text}');
                        el.dispatchEvent(new Event('input', {{bubbles: true}}));
                        el.dispatchEvent(new Event('change', {{bubbles: true}}));
                        return 'typed:nativeSet';
                    }}
                    el.value = '{escaped_text}';
                    el.dispatchEvent(new Event('input', {{bubbles: true}}));
                    return 'typed:valueSet';
                }}

                // Strategy 3: textContent for contenteditable
                if (el.isContentEditable) {{
                    el.textContent = '{escaped_text}';
                    el.dispatchEvent(new Event('input', {{bubbles: true}}));
                    return 'typed:textContent';
                }}

                return 'typed:execCommand';
            }})()"#
        );

        let result: String = self
            .page
            .evaluate(type_js)
            .await
            .map_err(|e| LynxError::Browser(format!("type_text failed: {e}")))?
            .into_value()
            .map_err(|e| LynxError::Browser(format!("type_text parse failed: {e:?}")))?;

        if result == "not_found" {
            return Err(LynxError::ElementNotFound(format!(
                "{ref_id} not found in DOM walk"
            )));
        }

        let method = result.strip_prefix("typed:").unwrap_or("unknown");
        Ok(format!("Typed into {ref_id} via {method}: {text}"))
    }

    pub async fn press(&mut self, ref_id: &str, key: &str) -> Result<String, LynxError> {
        self.check_alive()?;
        let _backend_id = *self
            .ref_map
            .resolve(ref_id)
            .ok_or_else(|| LynxError::ElementNotFound(ref_id.to_string()))?;

        // Map key names to KeyboardEvent key/code values
        let (key_value, code_value) = match key {
            "Enter" => ("Enter", "Enter"),
            "Tab" => ("Tab", "Tab"),
            "Escape" => ("Escape", "Escape"),
            "Backspace" => ("Backspace", "Backspace"),
            "Delete" => ("Delete", "Delete"),
            "ArrowUp" => ("ArrowUp", "ArrowUp"),
            "ArrowDown" => ("ArrowDown", "ArrowDown"),
            "ArrowLeft" => ("ArrowLeft", "ArrowLeft"),
            "ArrowRight" => ("ArrowRight", "ArrowRight"),
            "Home" => ("Home", "Home"),
            "End" => ("End", "End"),
            "PageUp" => ("PageUp", "PageUp"),
            "PageDown" => ("PageDown", "PageDown"),
            "Space" | " " => (" ", "Space"),
            other => (other, other),
        };

        // Find element by data-lynx-ref (stamped during snapshot)
        let press_js = format!(
            r#"(function() {{
                var el = document.querySelector('[data-lynx-ref="{ref_id}"]');
                if (!el) return 'not_found';
                el.focus();
                var opts = {{key: '{key_value}', code: '{code_value}', bubbles: true, cancelable: true}};
                el.dispatchEvent(new KeyboardEvent('keydown', opts));
                el.dispatchEvent(new KeyboardEvent('keypress', opts));
                el.dispatchEvent(new KeyboardEvent('keyup', opts));

                // For Enter, also submit the closest form
                if ('{key_value}' === 'Enter') {{
                    var form = el.closest('form');
                    if (form) form.requestSubmit ? form.requestSubmit() : form.submit();
                }}

                return 'pressed';
            }})()"#
        );

        let result: String = self
            .page
            .evaluate(press_js)
            .await
            .map_err(|e| LynxError::Browser(format!("press eval failed: {e}")))?
            .into_value()
            .map_err(|e| LynxError::Browser(format!("press result parse failed: {e:?}")))?;

        if result == "pressed" {
            Ok(format!("Pressed {key} on {ref_id}"))
        } else {
            Err(LynxError::ElementNotFound(format!(
                "{ref_id} not found in DOM walk"
            )))
        }
    }

    pub async fn upload_file(
        &self,
        ref_id: Option<&str>,
        file_paths: &[String],
    ) -> Result<String, LynxError> {
        self.check_alive()?;

        // Validate file paths exist locally
        for path in file_paths {
            if !std::path::Path::new(path).exists() {
                return Err(LynxError::Browser(format!("File not found: {path}")));
            }
        }

        // Find the file input element — by ref or first input[type=file]
        let selector = if let Some(rid) = ref_id {
            self.ref_map
                .resolve(rid)
                .ok_or_else(|| LynxError::ElementNotFound(rid.to_string()))?;
            format!("[data-lynx-ref=\"{rid}\"]")
        } else {
            "input[type=file]".to_string()
        };

        // Get a RemoteObjectId for the file input via JS evaluation
        use chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams;
        use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;

        let eval_js = format!(
            r#"(function() {{
                var el = document.querySelector('{selector}');
                if (!el) return null;
                if (el.tagName !== 'INPUT' || el.type !== 'file') {{
                    var nested = el.querySelector('input[type=file]');
                    if (nested) return nested;
                    return null;
                }}
                return el;
            }})()"#
        );

        let mut eval_params = EvaluateParams::new(eval_js);
        eval_params.return_by_value = Some(false);

        let eval_result = self
            .page
            .execute(eval_params)
            .await
            .map_err(|e| LynxError::Browser(format!("upload locate failed: {e}")))?;

        let remote_object = &eval_result.result.result;
        let object_id = remote_object
            .object_id
            .as_ref()
            .ok_or_else(|| {
                LynxError::ElementNotFound("No file input element found on page".to_string())
            })?
            .clone();

        // Set files via CDP DOM.setFileInputFiles
        let mut params = SetFileInputFilesParams::new(file_paths.to_vec());
        params.object_id = Some(object_id);

        self.page
            .execute(params)
            .await
            .map_err(|e| LynxError::Browser(format!("setFileInputFiles failed: {e}")))?;

        let file_names: Vec<&str> = file_paths
            .iter()
            .filter_map(|p| std::path::Path::new(p).file_name().and_then(|n| n.to_str()))
            .collect();

        Ok(format!(
            "Uploaded {} file(s): {}",
            file_paths.len(),
            file_names.join(", ")
        ))
    }

    pub async fn eval(&self, expression: &str) -> Result<String, LynxError> {
        self.check_alive()?;
        let enabled = std::env::var("LYNX_EVAL_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        if !enabled {
            return Err(LynxError::JsEval(
                "JavaScript evaluation is disabled (LYNX_EVAL_ENABLED=false)".into(),
            ));
        }

        let result: serde_json::Value = self
            .page
            .evaluate(expression)
            .await
            .map_err(|e| LynxError::JsEval(e.to_string()))?
            .into_value()
            .map_err(|e| LynxError::JsEval(format!("{e:?}")))?;

        serde_json::to_string_pretty(&result).map_err(|e| LynxError::JsEval(e.to_string()))
    }

    pub async fn dismiss_overlays(&self) -> Result<String, LynxError> {
        self.check_alive()?;
        let js = r#"
        (function() {
            const selectors = [
                '[aria-label*="close" i]',
                '[aria-label*="dismiss" i]',
                '[aria-label*="accept" i]',
                'button[class*="cookie" i]',
                'button[class*="consent" i]',
                '[id*="cookie" i] button',
                '[class*="modal" i] [aria-label*="close" i]',
                '[class*="overlay" i] button:first-of-type',
            ];
            let dismissed = 0;
            for (const sel of selectors) {
                const els = document.querySelectorAll(sel);
                for (const el of els) {
                    if (el.offsetParent !== null) {
                        el.click();
                        dismissed++;
                    }
                }
            }
            return dismissed;
        })()
        "#;

        let count: i64 = self
            .page
            .evaluate(js)
            .await
            .map_err(|e| LynxError::Browser(e.to_string()))?
            .into_value()
            .unwrap_or(0);

        Ok(format!("Dismissed {count} overlay(s)"))
    }

    pub async fn wait_for_stable(&self, timeout_ms: u64) -> Result<String, LynxError> {
        self.check_alive()?;
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);
        let mut last_text = String::new();
        let mut stable_rounds = 0;

        while start.elapsed() < timeout {
            let text: String = self
                .page
                .evaluate("document.body.innerText")
                .await
                .map_err(|e| LynxError::Browser(e.to_string()))?
                .into_value()
                .unwrap_or_default();

            if text == last_text && !text.is_empty() {
                stable_rounds += 1;
                if stable_rounds >= 3 {
                    return Ok(format!(
                        "Page stable after {}ms",
                        start.elapsed().as_millis()
                    ));
                }
            } else {
                stable_rounds = 0;
                last_text = text;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        Ok(format!(
            "Timeout after {timeout_ms}ms (may still be loading)"
        ))
    }

    pub async fn screenshot(&self, full_page: bool) -> Result<String, LynxError> {
        self.check_alive()?;
        let params = chromiumoxide::page::ScreenshotParams::builder()
            .format(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png)
            .full_page(full_page)
            .build();

        let bytes = self
            .page
            .screenshot(params)
            .await
            .map_err(|e| LynxError::Screenshot(e.to_string()))?;

        use base64::Engine;
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
    }

    pub async fn pdf(&self) -> Result<String, LynxError> {
        self.check_alive()?;
        let params = chromiumoxide::cdp::browser_protocol::page::PrintToPdfParams::default();
        let bytes = self
            .page
            .pdf(params)
            .await
            .map_err(|e| LynxError::Pdf(e.to_string()))?;

        use base64::Engine;
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
    }

    /// Show a non-blocking intervention banner at the bottom of the page.
    /// Fires a native macOS notification, then polls until the user clicks "Done" or timeout expires.
    pub async fn request_intervention(
        &self,
        message: &str,
        timeout_ms: u64,
    ) -> Result<String, LynxError> {
        self.check_alive()?;

        // System notification + browser banner
        self.show_system_notification(message);
        self.inject_intervention_banner(message).await;

        // Poll for dismiss
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        while start.elapsed() < timeout {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let dismissed: bool = self
                .page
                .evaluate("window.__lynx_intervention_dismissed === true")
                .await
                .map_err(|e| LynxError::Browser(format!("intervention poll failed: {e}")))?
                .into_value()
                .unwrap_or(false);

            if dismissed {
                return Ok(format!(
                    "User completed intervention after {}ms",
                    start.elapsed().as_millis()
                ));
            }
        }

        // Timeout — clean up banner
        let _ = self
            .page
            .evaluate(
                "(function(){ var b = document.getElementById('__lynx_intervention'); if (b) b.remove(); })()",
            )
            .await;

        Ok(format!(
            "Intervention timed out after {timeout_ms}ms (user did not click Done)"
        ))
    }

    pub async fn auth_login(
        &mut self,
        item: &str,
        url: &str,
        vault: Option<&str>,
    ) -> Result<String, LynxError> {
        // Get credentials from password manager
        let creds = crate::auth::op_cli::get_credentials(item, vault)?;

        // Navigate to login page
        self.navigate(url, 3000).await?;

        // Run iterative form fill
        let fill_result = crate::auth::form_fill::fill_login_form(self, &creds).await?;

        let current_url = self.current_url().await;
        Ok(format!(
            "Auth login for '{}' (user: {})\nURL: {current_url}\n{fill_result}",
            item, creds.username
        ))
    }
}
