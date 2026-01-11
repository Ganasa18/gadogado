use crate::domain::error::{AppError, Result};
use keyring::Entry;

pub struct KeyringManager {
    service: String,
}

impl KeyringManager {
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_string(),
        }
    }

    pub fn set_secret(&self, key: &str, secret: &str) -> Result<()> {
        let entry = Entry::new(&self.service, key)
            .map_err(|e| AppError::SecurityError(format!("Failed to create entry: {}", e)))?;

        entry
            .set_password(secret)
            .map_err(|e| AppError::SecurityError(format!("Failed to set password: {}", e)))?;

        Ok(())
    }

    pub fn get_secret(&self, key: &str) -> Result<String> {
        let entry = Entry::new(&self.service, key)
            .map_err(|e| AppError::SecurityError(format!("Failed to create entry: {}", e)))?;

        entry
            .get_password()
            .map_err(|e| AppError::SecurityError(format!("Failed to get password: {}", e)))
    }

    pub fn delete_secret(&self, key: &str) -> Result<()> {
        let entry = Entry::new(&self.service, key)
            .map_err(|e| AppError::SecurityError(format!("Failed to create entry: {}", e)))?;

        entry
            .delete_credential()
            .map_err(|e| AppError::SecurityError(format!("Failed to delete password: {}", e)))
    }
}
