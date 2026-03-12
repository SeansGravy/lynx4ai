pub mod form_fill;
pub mod op_cli;

/// Credentials returned by a password manager
#[derive(Debug, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub totp: Option<String>,
}
