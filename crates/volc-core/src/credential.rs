use crate::VolcError;
use std::fmt;

const SERVICE: &str = "volc-status";
const ACCOUNT: &str = "volcengine-ak-sk";
const SEPARATOR: char = '\0';

/// A validated Access Key / Secret Key pair.
///
/// Its `Debug` implementation deliberately redacts both values.
#[derive(Clone, PartialEq, Eq)]
pub struct Credentials {
    access_key: String,
    secret_key: String,
}

impl Credentials {
    /// Creates a credential pair after trimming surrounding whitespace.
    pub fn new(
        access_key: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> Result<Self, VolcError> {
        let access_key = access_key.into().trim().to_owned();
        let secret_key = secret_key.into().trim().to_owned();

        if access_key.is_empty() || secret_key.is_empty() {
            return Err(VolcError::CredentialsInvalid(
                "access key and secret key must both be non-empty".to_owned(),
            ));
        }
        if access_key.contains(SEPARATOR) || secret_key.contains(SEPARATOR) {
            return Err(VolcError::CredentialsInvalid(
                "credentials contain an unsupported null character".to_owned(),
            ));
        }

        Ok(Self {
            access_key,
            secret_key,
        })
    }

    pub(crate) fn access_key(&self) -> &str {
        &self.access_key
    }

    pub(crate) fn secret_key(&self) -> &str {
        &self.secret_key
    }

    fn encode(&self) -> String {
        format!("{}{}{}", self.access_key, SEPARATOR, self.secret_key)
    }

    fn decode(value: &str) -> Result<Self, VolcError> {
        let (access_key, secret_key) = value.split_once(SEPARATOR).ok_or_else(|| {
            VolcError::CredentialsInvalid("stored credential has an invalid format".to_owned())
        })?;
        Self::new(access_key, secret_key)
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Credentials")
            .field("access_key", &"[REDACTED]")
            .field("secret_key", &"[REDACTED]")
            .finish()
    }
}

/// Persistence interface for platform credential stores.
pub trait CredentialStore {
    /// Saves a credential pair.
    fn save(&self, credentials: &Credentials) -> Result<(), VolcError>;
    /// Loads the saved credential pair.
    fn load(&self) -> Result<Credentials, VolcError>;
    /// Removes the saved credential pair. Missing entries are accepted.
    fn clear(&self) -> Result<(), VolcError>;

    /// Reports whether a valid credential pair is available.
    fn has(&self) -> bool {
        self.load().is_ok()
    }
}

/// System-keyring implementation using the legacy service and account names.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyringCredentialStore;

impl KeyringCredentialStore {
    fn entry() -> Result<keyring::Entry, VolcError> {
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(VolcError::Keyring)
    }
}

impl CredentialStore for KeyringCredentialStore {
    fn save(&self, credentials: &Credentials) -> Result<(), VolcError> {
        Self::entry()?
            .set_password(&credentials.encode())
            .map_err(VolcError::Keyring)
    }

    fn load(&self) -> Result<Credentials, VolcError> {
        match Self::entry()?.get_password() {
            Ok(value) => Credentials::decode(&value),
            Err(keyring::Error::NoEntry) => Err(VolcError::CredentialsMissing),
            Err(error) => Err(VolcError::Keyring(error)),
        }
    }

    fn clear(&self) -> Result<(), VolcError> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(VolcError::Keyring(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_trim_and_round_trip() {
        let credentials = Credentials::new("  test-ak ", " test-sk  ").expect("valid test input");
        assert_eq!(
            Credentials::decode(&credentials.encode()).expect("encoded credentials decode"),
            credentials
        );
    }

    #[test]
    fn credentials_reject_empty_values() {
        assert!(Credentials::new("", "secret").is_err());
        assert!(Credentials::new("access", "   ").is_err());
    }

    #[test]
    fn debug_redacts_both_credentials() {
        let credentials = Credentials::new("test-ak", "test-sk").expect("valid test input");
        let debug = format!("{credentials:?}");
        assert!(!debug.contains("test-ak"));
        assert!(!debug.contains("test-sk"));
        assert!(debug.contains("[REDACTED]"));
    }
}
