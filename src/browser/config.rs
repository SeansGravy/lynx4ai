use std::path::PathBuf;

pub struct LynxConfig {
    pub chrome_path: PathBuf,
    pub profile_dir: PathBuf,
    pub headless: bool,
}

impl LynxConfig {
    pub fn from_env() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            chrome_path: std::env::var("LYNX_CHROME_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| detect_chrome()),
            profile_dir: std::env::var("LYNX_PROFILE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join(".lynx4ai").join("profiles")),
            headless: std::env::var("LYNX_HEADLESS")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true),
        }
    }
}

/// Cross-platform Chrome binary detection
fn detect_chrome() -> PathBuf {
    let candidates = if cfg!(target_os = "macos") {
        vec![
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
            "/snap/bin/chromium",
        ]
    } else {
        // Windows or other
        vec![
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ]
    };

    for path in candidates {
        let p = PathBuf::from(path);
        if p.exists() {
            return p;
        }
    }

    // Fallback — hope it's on PATH
    PathBuf::from("google-chrome")
}
