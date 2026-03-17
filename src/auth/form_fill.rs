use crate::auth::Credentials;
use crate::browser::instance::BrowserInstance;
use crate::error::LynxError;
use crate::types::SnapshotNode;

const MAX_LOGIN_ITERATIONS: usize = 5;

/// Patterns to match username-like fields (case-insensitive against name)
const USERNAME_PATTERNS: &[&str] = &["user", "email", "login", "account", "identifier"];
/// Patterns to match password-like fields
const PASSWORD_PATTERNS: &[&str] = &["pass", "secret", "pwd"];
/// Patterns to match TOTP/MFA fields
const TOTP_PATTERNS: &[&str] = &[
    "totp",
    "otp",
    "mfa",
    "verification",
    "code",
    "2fa",
    "authenticator",
];
/// Patterns to match submit buttons
const SUBMIT_PATTERNS: &[&str] = &["sign in", "log in", "login", "submit", "continue", "next"];

/// Find a textbox node whose name matches any of the given patterns
fn find_field<'a>(nodes: &'a [SnapshotNode], patterns: &[&str]) -> Option<&'a SnapshotNode> {
    nodes.iter().find(|n| {
        (n.role == "textbox" || n.role == "searchbox") && n.interactive && {
            let name_lower = n.name.to_lowercase();
            patterns.iter().any(|p| name_lower.contains(p))
        }
    })
}

/// Find a submit-like button
fn find_submit_button(nodes: &[SnapshotNode]) -> Option<&SnapshotNode> {
    // First try: button with submit-like name
    if let Some(btn) = nodes.iter().find(|n| {
        n.role == "button" && {
            let name_lower = n.name.to_lowercase();
            SUBMIT_PATTERNS.iter().any(|p| name_lower.contains(p))
        }
    }) {
        return Some(btn);
    }
    // Fallback: any interactive button
    nodes.iter().find(|n| n.role == "button" && n.interactive)
}

/// Iterative login form fill.
///
/// Takes snapshots, finds username/password/TOTP fields by name pattern,
/// fills them with credentials, clicks submit, and repeats up to 5 times
/// to handle multi-page login flows (username → password → TOTP).
pub async fn fill_login_form(
    instance: &mut BrowserInstance,
    creds: &Credentials,
) -> Result<String, LynxError> {
    let mut status_log = Vec::new();

    for iteration in 0..MAX_LOGIN_ITERATIONS {
        // Take interactive-only snapshot
        instance
            .snapshot(Some("interactive"), false, "compact", None, None)
            .await?;

        let nodes = match instance.last_snapshot {
            Some(ref snap) => snap.clone(),
            None => {
                status_log.push(format!("iter {iteration}: no snapshot available"));
                break;
            }
        };

        let mut acted = false;

        // Try to find and fill username field
        if let Some(user_field) = find_field(&nodes, USERNAME_PATTERNS) {
            instance
                .type_text(&user_field.ref_id, &creds.username, true)
                .await?;
            status_log.push(format!(
                "iter {iteration}: typed username into {}",
                user_field.ref_id
            ));
            acted = true;
        }

        // Try to find and fill password field
        if let Some(pass_field) = find_field(&nodes, PASSWORD_PATTERNS) {
            instance
                .type_text(&pass_field.ref_id, &creds.password, true)
                .await?;
            status_log.push(format!(
                "iter {iteration}: typed password into {}",
                pass_field.ref_id
            ));
            acted = true;
        }

        // Try to find and fill TOTP field
        if let Some(totp) = &creds.totp
            && let Some(totp_field) = find_field(&nodes, TOTP_PATTERNS)
        {
            instance.type_text(&totp_field.ref_id, totp, true).await?;
            status_log.push(format!(
                "iter {iteration}: typed TOTP into {}",
                totp_field.ref_id
            ));
            acted = true;
        }

        if !acted {
            status_log.push(format!(
                "iter {iteration}: no fillable fields found — login may be complete"
            ));
            break;
        }

        // Click submit button or press Enter on last filled field
        if let Some(submit_btn) = find_submit_button(&nodes) {
            instance.click(&submit_btn.ref_id).await?;
            status_log.push(format!(
                "iter {iteration}: clicked submit {}",
                submit_btn.ref_id
            ));
        } else {
            // Press Enter on the last field we filled
            let last_field = find_field(&nodes, PASSWORD_PATTERNS)
                .or_else(|| find_field(&nodes, TOTP_PATTERNS))
                .or_else(|| find_field(&nodes, USERNAME_PATTERNS));
            if let Some(field) = last_field {
                instance.press(&field.ref_id, "Enter").await?;
                status_log.push(format!(
                    "iter {iteration}: pressed Enter on {}",
                    field.ref_id
                ));
            }
        }

        // Wait for page transition after submit
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    }

    Ok(status_log.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_textbox(ref_id: &str, name: &str) -> SnapshotNode {
        SnapshotNode {
            ref_id: ref_id.to_string(),
            role: "textbox".to_string(),
            name: name.to_string(),
            description: None,
            value: None,
            interactive: true,
            children: Vec::new(),
        }
    }

    fn make_button(ref_id: &str, name: &str) -> SnapshotNode {
        SnapshotNode {
            ref_id: ref_id.to_string(),
            role: "button".to_string(),
            name: name.to_string(),
            description: None,
            value: None,
            interactive: true,
            children: Vec::new(),
        }
    }

    #[test]
    fn find_username_by_user() {
        let nodes = vec![
            make_textbox("e0", "Username"),
            make_textbox("e1", "Password"),
        ];
        let found = find_field(&nodes, USERNAME_PATTERNS);
        assert_eq!(found.unwrap().ref_id, "e0");
    }

    #[test]
    fn find_username_by_email() {
        let nodes = vec![
            make_textbox("e0", "Email address"),
            make_textbox("e1", "Password"),
        ];
        let found = find_field(&nodes, USERNAME_PATTERNS);
        assert_eq!(found.unwrap().ref_id, "e0");
    }

    #[test]
    fn find_password_field() {
        let nodes = vec![
            make_textbox("e0", "Username"),
            make_textbox("e1", "Password"),
        ];
        let found = find_field(&nodes, PASSWORD_PATTERNS);
        assert_eq!(found.unwrap().ref_id, "e1");
    }

    #[test]
    fn find_totp_field() {
        let nodes = vec![make_textbox("e0", "Enter verification code")];
        let found = find_field(&nodes, TOTP_PATTERNS);
        assert!(found.is_some());
    }

    #[test]
    fn find_submit_by_name() {
        let nodes = vec![make_button("e0", "Cancel"), make_button("e1", "Sign In")];
        let found = find_submit_button(&nodes);
        assert_eq!(found.unwrap().ref_id, "e1");
    }

    #[test]
    fn find_submit_fallback_to_any_button() {
        let nodes = vec![make_button("e0", "Go")];
        let found = find_submit_button(&nodes);
        assert_eq!(found.unwrap().ref_id, "e0");
    }

    #[test]
    fn no_match_returns_none() {
        let nodes = vec![make_textbox("e0", "Search")];
        assert!(find_field(&nodes, PASSWORD_PATTERNS).is_none());
    }
}
