//! Keyring-backed storage for the OpenCode Go `auth` cookie.
//!
//! The cookie is a session secret: it is never persisted to the plaintext
//! configuration file and is never printed in logs or diagnostics. The keyring
//! namespace is isolated from the VOLC Status application so both applications
//! can be installed at the same time.

use crate::OpenCodeError;

const SERVICE: &str = "opencode-quota-checker";
const ACCOUNT: &str = "opencode-auth";

/// Keyring entry for the OpenCode Go `auth` cookie.
#[derive(Debug, Clone, Copy, Default)]
pub struct OpenCodeAuthStore;

impl OpenCodeAuthStore {
    fn entry() -> Result<keyring::Entry, OpenCodeError> {
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(OpenCodeError::Keyring)
    }

    /// Saves a raw `auth` cookie value after trimming surrounding whitespace.
    pub fn save(&self, cookie: &str) -> Result<(), OpenCodeError> {
        let cookie = cookie.trim();
        if cookie.is_empty() {
            return Err(OpenCodeError::CredentialsInvalid(
                "auth cookie must not be empty".to_owned(),
            ));
        }
        Self::entry()?
            .set_password(cookie)
            .map_err(OpenCodeError::Keyring)
    }

    /// Loads the stored `auth` cookie value.
    pub fn load(&self) -> Result<String, OpenCodeError> {
        match Self::entry()?.get_password() {
            Ok(value) if value.trim().is_empty() => Err(OpenCodeError::CredentialsInvalid(
                "stored auth cookie is empty".to_owned(),
            )),
            Ok(value) => Ok(value),
            Err(keyring::Error::NoEntry) => Err(OpenCodeError::CredentialsMissing),
            Err(error) => Err(OpenCodeError::Keyring(error)),
        }
    }

    /// Removes the stored cookie. Missing entries are accepted.
    pub fn clear(&self) -> Result<(), OpenCodeError> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(OpenCodeError::Keyring(error)),
        }
    }

    /// Reports whether a cookie is currently stored.
    pub fn has(&self) -> bool {
        self.load().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyring_namespace_is_isolated_from_volc_status() {
        assert_eq!(SERVICE, "opencode-quota-checker");
        assert_eq!(ACCOUNT, "opencode-auth");
    }
}
