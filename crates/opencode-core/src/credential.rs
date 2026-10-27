//! Keyring-backed storage for the OpenCode Console API key.
//!
//! The key (`oc_sk_...`) grants access to the workspace's Go allowance and
//! Console usage; it is a secret and is never persisted to the plaintext
//! configuration file or written to logs.
//!
//! The pre-API-key clients stored a dashboard `auth` cookie under a different
//! account. That entry is kept untouched so a user can roll back, and is read
//! only to tell the UI that a migration is pending.

use crate::OpenCodeError;

const SERVICE: &str = "opencode-quota-checker";
/// Keyring account holding the Console API key.
const API_KEY_ACCOUNT: &str = "opencode-api-key";
/// Keyring account that stored the retired dashboard `auth` cookie.
pub const LEGACY_COOKIE_ACCOUNT: &str = "opencode-auth";

/// Keyring entry for the OpenCode Console API key.
#[derive(Debug, Clone, Copy, Default)]
pub struct OpenCodeApiKeyStore;

impl OpenCodeApiKeyStore {
    fn entry(account: &str) -> Result<keyring::Entry, OpenCodeError> {
        keyring::Entry::new(SERVICE, account).map_err(OpenCodeError::Keyring)
    }

    /// Saves an API key value after trimming surrounding whitespace.
    pub fn save(&self, api_key: &str) -> Result<(), OpenCodeError> {
        let api_key = api_key.trim();
        if api_key.is_empty() {
            return Err(OpenCodeError::CredentialsInvalid(
                "API key must not be empty".to_owned(),
            ));
        }
        Self::entry(API_KEY_ACCOUNT)?
            .set_password(api_key)
            .map_err(OpenCodeError::Keyring)
    }

    /// Loads the stored API key value.
    pub fn load(&self) -> Result<String, OpenCodeError> {
        match Self::entry(API_KEY_ACCOUNT)?.get_password() {
            Ok(value) if value.trim().is_empty() => Err(OpenCodeError::CredentialsInvalid(
                "stored API key is empty".to_owned(),
            )),
            Ok(value) => Ok(value),
            Err(keyring::Error::NoEntry) => Err(OpenCodeError::CredentialsMissing),
            Err(error) => Err(OpenCodeError::Keyring(error)),
        }
    }

    /// Removes the stored API key and any leftover legacy cookie.
    ///
    /// Missing entries are accepted, and a failure to remove the legacy cookie
    /// never masks the outcome of the API key removal.
    pub fn clear(&self) -> Result<(), OpenCodeError> {
        let key_result = match Self::entry(API_KEY_ACCOUNT) {
            Ok(entry) => match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(error) => Err(OpenCodeError::Keyring(error)),
            },
            Err(error) => Err(error),
        };
        if let Ok(legacy) = Self::entry(LEGACY_COOKIE_ACCOUNT) {
            let _ = legacy.delete_credential();
        }
        key_result
    }

    /// Reports whether an API key is currently stored.
    pub fn has(&self) -> bool {
        self.load().is_ok()
    }

    /// Reports whether the retired dashboard `auth` cookie is still stored.
    ///
    /// Used only to show the migration hint; the value never leaves the keyring
    /// through this check.
    pub fn has_legacy_cookie(&self) -> bool {
        match Self::entry(LEGACY_COOKIE_ACCOUNT) {
            Ok(entry) => matches!(
                entry.get_password(),
                Ok(value) if !value.trim().is_empty()
            ),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyring_namespace_uses_opencode_identifiers() {
        assert_eq!(SERVICE, "opencode-quota-checker");
        assert_eq!(API_KEY_ACCOUNT, "opencode-api-key");
        // The legacy cookie lives under its own account so migration is explicit.
        assert_eq!(LEGACY_COOKIE_ACCOUNT, "opencode-auth");
        assert_ne!(API_KEY_ACCOUNT, LEGACY_COOKIE_ACCOUNT);
    }
}
