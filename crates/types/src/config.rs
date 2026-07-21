//! Layered configuration with secret redaction.

use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::Path;

/// A redacted string for secrets.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SecretString {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Serialize for SecretString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("[REDACTED]")
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self)
    }
}

/// Server binding configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

/// Database connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub username: String,
    pub password: SecretString,
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            name: "moqentra".to_string(),
            username: "moqentra".to_string(),
            password: SecretString::new("moqentra"),
            max_connections: 10,
        }
    }
}

/// Object storage configuration.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectStorageConfig {
    pub endpoint: String,
    pub bucket: String,
    pub access_key_id: SecretString,
    pub secret_access_key: SecretString,
    pub region: String,
}

impl fmt::Debug for ObjectStorageConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjectStorageConfig")
            .field("endpoint", &self.endpoint)
            .field("bucket", &self.bucket)
            .field("access_key_id", &"[REDACTED]")
            .field("secret_access_key", &self.secret_access_key)
            .field("region", &self.region)
            .finish()
    }
}

impl Default for ObjectStorageConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:9000".to_string(),
            bucket: "moqentra".to_string(),
            access_key_id: SecretString::new("minioadmin"),
            secret_access_key: SecretString::new("minioadmin"),
            region: "us-east-1".to_string(),
        }
    }
}

/// Authentication configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthConfig {
    pub oidc_issuer: String,
    pub oidc_client_id: String,
}

/// Top-level configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub object_storage: ObjectStorageConfig,
    #[serde(default)]
    pub auth: AuthConfig,
}

fn percent_encode_userinfo(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

impl Configuration {
    /// Loads configuration from built-in defaults, an optional file, and
    /// environment variables prefixed with `MOQENTRA_`.
    pub fn load(file: Option<&Path>) -> Result<Self, ConfigError> {
        let defaults = serde_json::to_string(&Configuration::default()).map_err(|e| {
            ConfigError::Message(format!("default config serialization failed: {e}"))
        })?;
        let mut builder =
            Config::builder().add_source(File::from_str(&defaults, config::FileFormat::Json));

        if let Some(path) = file {
            builder = builder.add_source(File::from(path).required(true));
        }

        builder = builder
            .add_source(Environment::with_prefix("MOQENTRA").prefix_separator("_").separator("__"));

        builder.build()?.try_deserialize()
    }

    /// Returns the database DSN without exposing the password in a string.
    /// Userinfo characters that would break the URL are percent-encoded.
    pub fn database_dsn(&self) -> SecretString {
        SecretString::new(format!(
            "postgresql://{}:{}@{}:{}/{}",
            percent_encode_userinfo(&self.database.username),
            percent_encode_userinfo(self.database.password.expose_secret()),
            self.database.host,
            self.database.port,
            self.database.name
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_serializes() {
        let config = Configuration::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        // Secret values are redacted; plain access/bucket names may appear.
        assert!(json.contains("[REDACTED]"));
        assert!(json.contains("secret_access_key"));
        assert!(json.contains("password"));
    }

    #[test]
    fn config_unknown_field_rejected() {
        use std::io::Write;
        let mut file = tempfile::Builder::new().suffix(".yaml").tempfile().unwrap();
        writeln!(file, "server:\n  host: 0.0.0.0\nunknown_field: true").unwrap();
        let result = Configuration::load(Some(file.path()));
        assert!(result.is_err());
    }

    #[test]
    fn database_dsn_is_secret() {
        let config = Configuration::default();
        let dsn = config.database_dsn();
        assert!(!format!("{:?}", dsn).contains("moqentra"));
    }

    #[test]
    fn database_dsn_encodes_special_characters() {
        let mut config = Configuration::default();
        config.database.username = "user@dom:ain".to_string();
        config.database.password = SecretString::new("p@ss:w0rd#");
        let dsn = config.database_dsn().expose_secret().to_string();
        assert!(dsn.starts_with("postgresql://user%40dom%3Aain:p%40ss%3Aw0rd%23@"));
        assert!(!dsn.contains("p@ss:w0rd#"));
    }
}
