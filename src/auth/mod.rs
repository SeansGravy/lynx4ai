pub mod form_fill;
pub mod op_cli;

/// Generic credential provider trait
/// Extensible to support password managers beyond 1Password
pub trait CredentialProvider {
    fn get_credentials(
        &self,
        item: &str,
        vault: Option<&str>,
    ) -> Result<Credentials, crate::error::LynxError>;
}

/// Credentials returned by a password manager
#[derive(Debug, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub totp: Option<String>,
}
