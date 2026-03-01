use std::process::Command;

use crate::auth::Credentials;
use crate::error::LynxError;

/// Get credentials from 1Password CLI (`op`)
pub fn get_credentials(item: &str, vault: Option<&str>) -> Result<Credentials, LynxError> {
    let mut cmd = Command::new("op");
    cmd.args([
        "item",
        "get",
        item,
        "--fields",
        "label=username,label=password",
        "--format",
        "json",
    ]);

    if let Some(v) = vault {
        cmd.args(["--vault", v]);
    }

    let output = cmd
        .output()
        .map_err(|e| LynxError::AuthProvider(format!("op CLI failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LynxError::AuthProvider(format!("op error: {stderr}")));
    }

    let fields: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)
        .map_err(|e| LynxError::AuthProvider(format!("op JSON parse error: {e}")))?;

    let mut username = String::new();
    let mut password = String::new();

    for field in &fields {
        let label = field
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        let value = field
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if label == "username" {
            username = value;
        } else if label == "password" {
            password = value;
        }
    }

    // Try to get TOTP
    let totp = get_totp(item, vault).ok();

    Ok(Credentials {
        username,
        password,
        totp,
    })
}

/// Get TOTP code from 1Password
fn get_totp(item: &str, vault: Option<&str>) -> Result<String, LynxError> {
    let mut cmd = Command::new("op");
    cmd.args(["item", "get", item, "--otp"]);

    if let Some(v) = vault {
        cmd.args(["--vault", v]);
    }

    let output = cmd
        .output()
        .map_err(|e| LynxError::AuthProvider(format!("op OTP failed: {e}")))?;

    if !output.status.success() {
        return Err(LynxError::AuthProvider("No TOTP configured".into()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
