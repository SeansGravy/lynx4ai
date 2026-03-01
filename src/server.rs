use std::sync::Arc;
use tokio::sync::RwLock;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::browser::BrowserManager;

// --- Parameter structs for each tool ---

#[derive(Deserialize, JsonSchema)]
pub struct InstanceCreateParams {
    /// Profile name for persistent session storage
    pub profile: Option<String>,
    /// Run in headless mode (default: true). Set false for visible Chrome.
    pub headless: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct InstanceDestroyParams {
    /// Instance ID to destroy
    pub instance_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct NavigateParams {
    /// URL to navigate to
    pub url: String,
    /// Instance ID (uses default if omitted)
    pub instance_id: Option<String>,
    /// Block image loading for speed
    pub block_images: Option<bool>,
    /// Extra wait in ms after navigation (default: 2000)
    pub wait_ms: Option<u64>,
}

#[derive(Deserialize, JsonSchema)]
pub struct SnapshotParams {
    /// Instance ID (uses default if omitted)
    pub instance_id: Option<String>,
    /// Filter: "interactive" for only clickable/typeable elements
    pub filter: Option<String>,
    /// Return only changes since last snapshot
    pub diff: Option<bool>,
    /// Output format: "compact" or "full" (default)
    pub format: Option<String>,
    /// CSS selector to scope snapshot
    pub selector: Option<String>,
    /// Approximate max tokens in output
    pub max_tokens: Option<u64>,
}

#[derive(Deserialize, JsonSchema)]
pub struct TextParams {
    /// Instance ID (uses default if omitted)
    pub instance_id: Option<String>,
    /// Approximate max tokens (default: 800)
    pub max_tokens: Option<u64>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ClickParams {
    /// Element ref from snapshot (e.g., "e5")
    pub ref_id: String,
    /// Instance ID (uses default if omitted)
    pub instance_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct TypeTextParams {
    /// Element ref from snapshot
    pub ref_id: String,
    /// Text to type
    pub text: String,
    /// Clear existing content first
    pub clear_first: Option<bool>,
    /// Instance ID (uses default if omitted)
    pub instance_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct PressParams {
    /// Element ref from snapshot
    pub ref_id: String,
    /// Key name: Enter, Tab, Escape, ArrowDown, etc.
    pub key: String,
    /// Instance ID (uses default if omitted)
    pub instance_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct UploadFileParams {
    /// Local file path(s), comma-separated for multiple
    pub file_paths: String,
    /// Instance ID (uses default if omitted)
    pub instance_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct EvalParams {
    /// JavaScript expression to evaluate
    pub expression: String,
    /// Instance ID (uses default if omitted)
    pub instance_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct InstanceIdParam {
    /// Instance ID (uses default if omitted)
    pub instance_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct WaitForStableParams {
    /// Instance ID (uses default if omitted)
    pub instance_id: Option<String>,
    /// Max wait time in ms (default: 10000)
    pub timeout_ms: Option<u64>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ScreenshotParams {
    /// Instance ID (uses default if omitted)
    pub instance_id: Option<String>,
    /// Capture full scrollable page
    pub full_page: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct AuthLoginParams {
    /// Password manager item name/ID
    pub item: String,
    /// Login page URL
    pub url: String,
    /// Password manager vault name
    pub vault: Option<String>,
    /// Instance ID (uses default if omitted)
    pub instance_id: Option<String>,
}

// --- Server ---

#[derive(Clone)]
pub struct LynxServer {
    manager: Arc<RwLock<BrowserManager>>,
    tool_router: ToolRouter<Self>,
}

impl Default for LynxServer {
    fn default() -> Self {
        Self::new()
    }
}

impl LynxServer {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(BrowserManager::new())),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router(router = tool_router)]
impl LynxServer {
    #[tool(description = "Create a new browser instance with persistent Chrome profile. Returns instance ID.")]
    async fn instance_create(&self, Parameters(p): Parameters<InstanceCreateParams>) -> String {
        let profile = p.profile.unwrap_or_else(|| "default".to_string());
        let headless = p.headless.unwrap_or(true);
        let mut mgr = self.manager.write().await;
        match mgr.create_instance(&profile, headless).await {
            Ok(id) => format!("Instance created: {id}\nProfile: {profile}\nHeadless: {headless}"),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all running browser instances with their IDs, profiles, and current URLs.")]
    async fn instance_list(&self) -> String {
        let mgr = self.manager.read().await;
        let instances = mgr.list_instances();
        if instances.is_empty() {
            return "No running instances".to_string();
        }
        serde_json::to_string_pretty(&instances).unwrap_or_else(|_| "Error serializing".into())
    }

    #[tool(description = "Destroy a browser instance by its ID.")]
    async fn instance_destroy(&self, Parameters(p): Parameters<InstanceDestroyParams>) -> String {
        let mut mgr = self.manager.write().await;
        match mgr.destroy_instance(&p.instance_id).await {
            Ok(()) => format!("Instance {} destroyed", p.instance_id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Navigate to a URL. Waits for page load + accessibility tree. Returns page title and URL.")]
    async fn navigate(&self, Parameters(p): Parameters<NavigateParams>) -> String {
        let mut mgr = self.manager.write().await;
        match mgr.navigate(&p.instance_id, &p.url, p.block_images.unwrap_or(false), p.wait_ms.unwrap_or(2000)).await {
            Ok(info) => info,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Get accessibility tree snapshot with stable element refs (e0, e1, e2...). Use filter='interactive' for only clickable/typeable elements. Use diff=true for changes since last snapshot. Use format='compact' for fewer tokens.")]
    async fn snapshot(&self, Parameters(p): Parameters<SnapshotParams>) -> String {
        let mut mgr = self.manager.write().await;
        match mgr.snapshot(&p.instance_id, p.filter.as_deref(), p.diff.unwrap_or(false), p.format.as_deref().unwrap_or("full"), p.selector.as_deref(), p.max_tokens.map(|v| v as usize)).await {
            Ok(result) => result,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Extract readable text from the page (~800 tokens by default).")]
    async fn text(&self, Parameters(p): Parameters<TextParams>) -> String {
        let mgr = self.manager.read().await;
        match mgr.text(&p.instance_id, p.max_tokens.unwrap_or(800) as usize).await {
            Ok(text) => text,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Click an element by its ref ID from snapshot (e.g., 'e5'). Auto-dismisses overlays and retries.")]
    async fn click(&self, Parameters(p): Parameters<ClickParams>) -> String {
        let mut mgr = self.manager.write().await;
        match mgr.click(&p.instance_id, &p.ref_id).await {
            Ok(msg) => msg,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Type text into an element by its ref ID.")]
    async fn type_text(&self, Parameters(p): Parameters<TypeTextParams>) -> String {
        let mut mgr = self.manager.write().await;
        match mgr.type_text(&p.instance_id, &p.ref_id, &p.text, p.clear_first.unwrap_or(false)).await {
            Ok(msg) => msg,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Press a keyboard key on an element (Enter, Tab, Escape, ArrowDown, etc.).")]
    async fn press(&self, Parameters(p): Parameters<PressParams>) -> String {
        let mut mgr = self.manager.write().await;
        match mgr.press(&p.instance_id, &p.ref_id, &p.key).await {
            Ok(msg) => msg,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Upload file(s) via a file input element on the page. Comma-separate multiple paths.")]
    async fn upload_file(&self, Parameters(p): Parameters<UploadFileParams>) -> String {
        let paths: Vec<String> = p.file_paths.split(',').map(|s| s.trim().to_string()).collect();
        let mgr = self.manager.read().await;
        match mgr.upload_file(&p.instance_id, &paths).await {
            Ok(msg) => msg,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Execute JavaScript in the page context and return the result as JSON.")]
    async fn eval(&self, Parameters(p): Parameters<EvalParams>) -> String {
        let mgr = self.manager.read().await;
        match mgr.eval(&p.instance_id, &p.expression).await {
            Ok(result) => result,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Dismiss cookie banners, modals, and popups blocking page interaction.")]
    async fn dismiss_overlays(&self, Parameters(p): Parameters<InstanceIdParam>) -> String {
        let mgr = self.manager.read().await;
        match mgr.dismiss_overlays(&p.instance_id).await {
            Ok(msg) => msg,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Wait until page content stabilizes (text stops changing, no loading spinners).")]
    async fn wait_for_stable(&self, Parameters(p): Parameters<WaitForStableParams>) -> String {
        let mgr = self.manager.read().await;
        match mgr.wait_for_stable(&p.instance_id, p.timeout_ms.unwrap_or(10000)).await {
            Ok(msg) => msg,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Take a page screenshot. Returns base64-encoded PNG.")]
    async fn screenshot(&self, Parameters(p): Parameters<ScreenshotParams>) -> String {
        let mgr = self.manager.read().await;
        match mgr.screenshot(&p.instance_id, p.full_page.unwrap_or(false)).await {
            Ok(b64) => format!("data:image/png;base64,{b64}"),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Export the current page as PDF. Returns base64-encoded PDF.")]
    async fn pdf(&self, Parameters(p): Parameters<InstanceIdParam>) -> String {
        let mgr = self.manager.read().await;
        match mgr.pdf(&p.instance_id).await {
            Ok(b64) => format!("data:application/pdf;base64,{b64}"),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Log into a website using credentials from a password manager (1Password by default). Handles multi-page login flows with username, password, and optional TOTP.")]
    async fn auth_login(&self, Parameters(p): Parameters<AuthLoginParams>) -> String {
        let mut mgr = self.manager.write().await;
        match mgr.auth_login(&p.instance_id, &p.item, &p.url, p.vault.as_deref()).await {
            Ok(msg) => msg,
            Err(e) => format!("Error: {e}"),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LynxServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "lynx4ai: AI browser automation via Chrome accessibility tree. \
                 Create a browser instance, navigate to pages, snapshot the accessibility tree \
                 with stable element refs, interact via click/type/press, and authenticate \
                 via password manager. Inspired by the original Lynx text browser (1992)."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
