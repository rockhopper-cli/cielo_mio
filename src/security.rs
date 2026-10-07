// src/security.rs

use keyring::Entry;

pub const SERVICE_NAME: &str = "cielo_mio";
pub const KEY_GEMINI_API: &str = "gemini_api_key";
pub const KEY_DB_PASSWORD: &str = "db_password";

pub struct SecretManager;

impl SecretManager {
    /// Retrieves a secret following the priority:
    /// 1. OS Secure Credential Store (KWallet / FreeDesktop Secret Service)
    /// 2. Environment variable (for headless testing / CI)
    /// 3. None
    pub fn get_secret(key_name: &str, env_var_name: &str) -> Option<String> {
        // 1. Check OS Keyring
        if let Ok(entry) = Entry::new(SERVICE_NAME, key_name) {
            if let Ok(password) = entry.get_password() {
                if !password.trim().is_empty() {
                    return Some(password.trim().to_string());
                }
            }
        }

        // 2. Fallback to environment variable
        if let Ok(val) = std::env::var(env_var_name) {
            if !val.trim().is_empty() {
                return Some(val.trim().to_string());
            }
        }

        None
    }

    /// Persists a secret into the OS Keyring (KWallet / GNOME Keyring)
    pub fn save_secret(key_name: &str, secret: &str) -> Result<(), String> {
        let entry = Entry::new(SERVICE_NAME, key_name)
            .map_err(|e| format!("Failed to create keyring entry: {}", e))?;
        
        entry.set_password(secret.trim())
            .map_err(|e| format!("Failed to store secret in OS Keyring: {}", e))
    }


    /// Removes a secret from the OS Keyring
    pub fn delete_secret(key_name: &str) -> Result<(), String> {
        let entry = Entry::new(SERVICE_NAME, key_name)
            .map_err(|e| format!("Failed to create keyring entry: {}", e))?;
        
        // Use delete_credential() in keyring
        entry.delete_credential()
            .map_err(|e| format!("Failed to remove secret from OS Keyring: {}", e))
    }

    /// Checks if a key is currently stored in the OS Keyring
    pub fn is_persisted_in_keyring(key_name: &str) -> bool {
        if let Ok(entry) = Entry::new(SERVICE_NAME, key_name) {
            entry.get_password().map(|p| !p.trim().is_empty()).unwrap_or(false)
        } else {
            false
        }
    }
}