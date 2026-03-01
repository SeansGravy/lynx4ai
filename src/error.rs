use thiserror::Error;

#[derive(Error, Debug)]
pub enum LynxError {
    #[error("Browser error: {0}")]
    Browser(String),

    #[error("No active instance: {0}")]
    NoInstance(String),

    #[error("Instance not found: {0}")]
    InstanceNotFound(String),

    #[error("Navigation failed: {url}: {reason}")]
    Navigation { url: String, reason: String },

    #[error("Element not found: ref={0}")]
    ElementNotFound(String),

    #[error("Snapshot failed: {0}")]
    Snapshot(String),

    #[error("JavaScript evaluation error: {0}")]
    JsEval(String),

    #[error("Auth provider error: {0}")]
    AuthProvider(String),

    #[error("Auth failed: {0}")]
    Auth(String),

    #[error("Screenshot failed: {0}")]
    Screenshot(String),

    #[error("PDF export failed: {0}")]
    Pdf(String),

    #[error("Chrome not found at {0}")]
    ChromeNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
