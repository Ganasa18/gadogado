use crate::domain::error::Result;
use crate::infrastructure::security::keyring::KeyringManager;

pub struct ConfigService {
    keyring: KeyringManager,
}

impl ConfigService {
    pub fn new() -> Self {
        Self {
            keyring: KeyringManager::new("PromptBridge"),
        }
    }

    pub fn save_api_key(&self, provider: &str, key: &str) -> Result<()> {
        self.keyring.set_secret(provider, key)
    }

    pub fn get_api_key(&self, provider: &str) -> Result<String> {
        self.keyring.get_secret(provider)
    }

    pub fn delete_api_key(&self, provider: &str) -> Result<()> {
        self.keyring.delete_secret(provider)
    }
}
