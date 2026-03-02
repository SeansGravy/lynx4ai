pub mod config;
pub mod helpers;
pub mod instance;

use std::collections::HashMap;

use crate::error::LynxError;
use crate::types::{InstanceId, InstanceInfo};

pub struct BrowserManager {
    instances: HashMap<InstanceId, instance::BrowserInstance>,
    config: config::LynxConfig,
}

/// Result of checking if an instance is alive, with enough info to recover
struct DeadInstanceInfo {
    id: InstanceId,
    profile: String,
    headless: bool,
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserManager {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            config: config::LynxConfig::from_env(),
        }
    }

    /// Check if the resolved instance is dead and return its info for recovery
    fn check_dead(&self, id: &Option<String>) -> Option<DeadInstanceInfo> {
        let inst = match id {
            Some(id) => self.instances.get(id),
            None => self.instances.values().next(),
        };
        match inst {
            Some(inst) if !inst.is_alive() => Some(DeadInstanceInfo {
                id: inst.id.clone(),
                profile: inst.profile.clone(),
                headless: inst.headless,
            }),
            _ => None,
        }
    }

    /// If the target instance is dead, relaunch it with the same profile.
    /// Returns Ok(true) if recovery happened, Ok(false) if instance was fine.
    async fn recover_if_dead(&mut self, id: &Option<String>) -> Result<bool, LynxError> {
        if let Some(dead) = self.check_dead(id) {
            tracing::warn!(
                "Instance '{}' is dead — auto-recovering with profile '{}'",
                dead.id, dead.profile
            );
            // Remove the dead instance
            self.instances.remove(&dead.id);
            // Relaunch with same profile and headless setting
            let new_inst = instance::BrowserInstance::launch(
                &self.config,
                &dead.profile,
                dead.headless,
            ).await?;
            // Keep the same ID so callers don't need to update references
            let mut recovered = new_inst;
            recovered.id = dead.id.clone();
            self.instances.insert(dead.id, recovered);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get a reference to an instance by ID, or the default instance
    fn get_instance(&self, id: &Option<String>) -> Result<&instance::BrowserInstance, LynxError> {
        match id {
            Some(id) => self
                .instances
                .get(id)
                .ok_or_else(|| LynxError::InstanceNotFound(id.clone())),
            None => self
                .instances
                .values()
                .next()
                .ok_or_else(|| LynxError::NoInstance("no instances running".into())),
        }
    }

    /// Get a mutable reference to an instance
    fn get_instance_mut(
        &mut self,
        id: &Option<String>,
    ) -> Result<&mut instance::BrowserInstance, LynxError> {
        match id {
            Some(id) => self
                .instances
                .get_mut(id)
                .ok_or_else(|| LynxError::InstanceNotFound(id.clone())),
            None => self
                .instances
                .values_mut()
                .next()
                .ok_or_else(|| LynxError::NoInstance("no instances running".into())),
        }
    }

    pub async fn create_instance(
        &mut self,
        profile: &str,
        headless: bool,
    ) -> Result<String, LynxError> {
        let inst = instance::BrowserInstance::launch(&self.config, profile, headless).await?;
        let id = inst.id.clone();
        self.instances.insert(id.clone(), inst);
        Ok(id)
    }

    pub fn list_instances(&self) -> Vec<InstanceInfo> {
        self.instances.values().map(|i| i.info()).collect()
    }

    pub async fn destroy_instance(&mut self, id: &str) -> Result<(), LynxError> {
        match self.instances.remove(id) {
            Some(inst) => {
                // Clean up SingletonLock files so the profile can be reused immediately
                let profile_dir = self.config.profile_dir.join(&inst.profile);
                for lock_file in &["SingletonLock", "SingletonCookie", "SingletonSocket"] {
                    let lock_path = profile_dir.join(lock_file);
                    if lock_path.exists() {
                        tracing::info!("Cleaning {lock_file} from profile '{}'", inst.profile);
                        let _ = std::fs::remove_file(&lock_path);
                    }
                }
                Ok(())
            }
            None => Err(LynxError::InstanceNotFound(id.to_string())),
        }
    }

    pub async fn navigate(
        &mut self,
        id: &Option<String>,
        url: &str,
        _block_images: bool,
        wait_ms: u64,
    ) -> Result<String, LynxError> {
        self.recover_if_dead(id).await?;
        let inst = self.get_instance_mut(id)?;
        inst.navigate(url, wait_ms).await
    }

    pub async fn snapshot(
        &mut self,
        id: &Option<String>,
        filter: Option<&str>,
        diff: bool,
        format: &str,
        selector: Option<&str>,
        max_tokens: Option<usize>,
    ) -> Result<String, LynxError> {
        self.recover_if_dead(id).await?;
        let inst = self.get_instance_mut(id)?;
        inst.snapshot(filter, diff, format, selector, max_tokens)
            .await
    }

    pub async fn text(&mut self, id: &Option<String>, max_tokens: usize) -> Result<String, LynxError> {
        self.recover_if_dead(id).await?;
        let inst = self.get_instance(id)?;
        inst.text(max_tokens).await
    }

    pub async fn click(&mut self, id: &Option<String>, ref_id: &str) -> Result<String, LynxError> {
        self.recover_if_dead(id).await?;
        let inst = self.get_instance_mut(id)?;
        inst.click(ref_id).await
    }

    pub async fn type_text(
        &mut self,
        id: &Option<String>,
        ref_id: &str,
        text: &str,
        clear_first: bool,
    ) -> Result<String, LynxError> {
        self.recover_if_dead(id).await?;
        let inst = self.get_instance_mut(id)?;
        inst.type_text(ref_id, text, clear_first).await
    }

    pub async fn press(
        &mut self,
        id: &Option<String>,
        ref_id: &str,
        key: &str,
    ) -> Result<String, LynxError> {
        self.recover_if_dead(id).await?;
        let inst = self.get_instance_mut(id)?;
        inst.press(ref_id, key).await
    }

    pub async fn upload_file(
        &mut self,
        id: &Option<String>,
        file_paths: &[String],
    ) -> Result<String, LynxError> {
        self.recover_if_dead(id).await?;
        let inst = self.get_instance(id)?;
        inst.upload_file(file_paths).await
    }

    pub async fn eval(
        &mut self,
        id: &Option<String>,
        expression: &str,
    ) -> Result<String, LynxError> {
        self.recover_if_dead(id).await?;
        let inst = self.get_instance(id)?;
        inst.eval(expression).await
    }

    pub async fn dismiss_overlays(&mut self, id: &Option<String>) -> Result<String, LynxError> {
        self.recover_if_dead(id).await?;
        let inst = self.get_instance(id)?;
        inst.dismiss_overlays().await
    }

    pub async fn wait_for_stable(
        &mut self,
        id: &Option<String>,
        timeout_ms: u64,
    ) -> Result<String, LynxError> {
        self.recover_if_dead(id).await?;
        let inst = self.get_instance(id)?;
        inst.wait_for_stable(timeout_ms).await
    }

    pub async fn screenshot(
        &mut self,
        id: &Option<String>,
        full_page: bool,
    ) -> Result<String, LynxError> {
        self.recover_if_dead(id).await?;
        let inst = self.get_instance(id)?;
        inst.screenshot(full_page).await
    }

    pub async fn pdf(&mut self, id: &Option<String>) -> Result<String, LynxError> {
        self.recover_if_dead(id).await?;
        let inst = self.get_instance(id)?;
        inst.pdf().await
    }

    pub async fn auth_login(
        &mut self,
        id: &Option<String>,
        item: &str,
        url: &str,
        vault: Option<&str>,
    ) -> Result<String, LynxError> {
        let inst = self.get_instance_mut(id)?;
        inst.auth_login(item, url, vault).await
    }
}
