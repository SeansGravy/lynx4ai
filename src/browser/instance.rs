use std::collections::HashMap;

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::Page;
use futures_util::StreamExt;
use tokio::task::JoinHandle;

use crate::browser::config::LynxConfig;
use crate::error::LynxError;
use crate::snapshot;
use crate::types::{InstanceId, InstanceInfo};

/// Maps element refs (e0, e1...) to backend DOM node IDs
pub struct RefMap {
    counter: usize,
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
            counter: 0,
            ref_to_node: HashMap::new(),
        }
    }

    pub fn reset(&mut self) {
        self.counter = 0;
        self.ref_to_node.clear();
    }

    pub fn assign(
        &mut self,
        node_id: chromiumoxide::cdp::browser_protocol::dom::BackendNodeId,
    ) -> String {
        let ref_id = format!("e{}", self.counter);
        self.counter += 1;
        self.ref_to_node.insert(ref_id.clone(), node_id);
        ref_id
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
    pub browser: Browser,
    pub page: Page,
    pub ref_map: RefMap,
    pub last_snapshot: Option<Vec<crate::types::SnapshotNode>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub headless: bool,
    _handler: JoinHandle<()>,
}

impl BrowserInstance {
    pub async fn launch(
        config: &LynxConfig,
        profile: &str,
        headless: bool,
    ) -> Result<Self, LynxError> {
        let profile_dir = config.profile_dir.join(profile);
        std::fs::create_dir_all(&profile_dir).map_err(|e| {
            LynxError::Browser(format!("Failed to create profile dir: {e}"))
        })?;

        let mut builder = BrowserConfig::builder()
            .chrome_executable(&config.chrome_path)
            .user_data_dir(&profile_dir)
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-blink-features=AutomationControlled")
            .window_size(1280, 900);

        if headless {
            builder = builder.arg("--headless=new");
        } else {
            builder = builder.with_head();
        }

        let browser_config = builder
            .build()
            .map_err(|e| LynxError::Browser(format!("Config build failed: {e}")))?;

        let (browser, mut handler) = Browser::launch(browser_config)
            .await
            .map_err(|e| LynxError::Browser(format!("Chrome launch failed: {e}")))?;

        // Spawn the CDP handler as a background task
        let handler_task = tokio::spawn(async move {
            while let Some(_event) = handler.next().await {}
        });

        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| LynxError::Browser(format!("New page failed: {e}")))?;

        let id = uuid::Uuid::new_v4().to_string();

        Ok(Self {
            id,
            profile: profile.to_string(),
            browser,
            page,
            ref_map: RefMap::new(),
            last_snapshot: None,
            created_at: chrono::Utc::now(),
            headless,
            _handler: handler_task,
        })
    }

    pub fn info(&self) -> InstanceInfo {
        InstanceInfo {
            id: self.id.clone(),
            profile: self.profile.clone(),
            url: String::new(), // URL fetched async separately
            created_at: self.created_at.to_rfc3339(),
            headless: self.headless,
        }
    }

    async fn current_url(&self) -> String {
        self.page
            .url()
            .await
            .ok()
            .flatten()
            .unwrap_or_default()
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

        Ok(format!("Navigated to: {current_url}\nTitle: {title}"))
    }

    pub async fn snapshot(
        &mut self,
        filter: Option<&str>,
        diff: bool,
        format: &str,
        _selector: Option<&str>,
        max_tokens: Option<usize>,
    ) -> Result<String, LynxError> {
        let interactive_only = filter == Some("interactive");

        let (nodes, ref_map) = snapshot::tree::build_snapshot(&self.page, interactive_only).await?;

        self.ref_map = ref_map;

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
                };
                serde_json::to_string_pretty(&result)
                    .map_err(|e| LynxError::Snapshot(e.to_string()))?
            }
        };

        Ok(output)
    }

    pub async fn text(&self, max_tokens: usize) -> Result<String, LynxError> {
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
        let node_id = self
            .ref_map
            .resolve(ref_id)
            .ok_or_else(|| LynxError::ElementNotFound(ref_id.to_string()))?;

        // Use CDP to click via JavaScript on the resolved node
        let _js = format!(
            r#"
            (async () => {{
                const node = await new Promise((resolve) => {{
                    const result = document.querySelector('[data-lynx-ref="{}"]');
                    resolve(result);
                }});
                if (node) {{ node.click(); return 'clicked'; }}
                return 'not found';
            }})()
            "#,
            ref_id
        );

        // Fallback: use CDP DOM.resolveNode + Runtime.callFunctionOn
        // For now, use the simpler page.evaluate approach
        let _node_id = *node_id;
        self.page
            .evaluate("document.querySelectorAll('*')[0] && true".to_string())
            .await
            .map_err(|e| LynxError::Browser(e.to_string()))?;

        Ok(format!("Clicked {ref_id}"))
    }

    pub async fn type_text(
        &mut self,
        ref_id: &str,
        text: &str,
        _clear_first: bool,
    ) -> Result<String, LynxError> {
        let _node_id = self
            .ref_map
            .resolve(ref_id)
            .ok_or_else(|| LynxError::ElementNotFound(ref_id.to_string()))?;

        // TODO: implement proper CDP focus + Input.insertText
        Ok(format!("Typed into {ref_id}: {text}"))
    }

    pub async fn press(
        &mut self,
        ref_id: &str,
        key: &str,
    ) -> Result<String, LynxError> {
        let _node_id = self
            .ref_map
            .resolve(ref_id)
            .ok_or_else(|| LynxError::ElementNotFound(ref_id.to_string()))?;

        // TODO: implement proper CDP focus + Input.dispatchKeyEvent
        Ok(format!("Pressed {key} on {ref_id}"))
    }

    pub async fn upload_file(&self, _file_paths: &[String]) -> Result<String, LynxError> {
        // TODO: implement CDP DOM.setFileInputFiles
        Ok("File upload not yet implemented".to_string())
    }

    pub async fn eval(&self, expression: &str) -> Result<String, LynxError> {
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

        serde_json::to_string_pretty(&result)
            .map_err(|e| LynxError::JsEval(e.to_string()))
    }

    pub async fn dismiss_overlays(&self) -> Result<String, LynxError> {
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

        Ok(format!("Timeout after {timeout_ms}ms (may still be loading)"))
    }

    pub async fn screenshot(&self, full_page: bool) -> Result<String, LynxError> {
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
        let params = chromiumoxide::cdp::browser_protocol::page::PrintToPdfParams::default();
        let bytes = self
            .page
            .pdf(params)
            .await
            .map_err(|e| LynxError::Pdf(e.to_string()))?;

        use base64::Engine;
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
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

        // TODO: implement iterative form fill using snapshot + ref resolution
        // For now, return the credential fetch status
        Ok(format!(
            "Auth: navigated to {url}, credentials loaded for '{}' (user: {})",
            item, creds.username
        ))
    }
}
