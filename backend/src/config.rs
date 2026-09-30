//! Process configuration, read once at startup.
//!
//! Every environment variable the backend reads is enumerated here. Required
//! variables make startup fail with an error that names the variable; secret
//! values are never included in errors or logs.

use std::fmt;
use std::path::PathBuf;

use http::HeaderValue;

/// The value of `JWT_SECRET` that older builds silently fell back to. It was
/// committed to the repository, so it is public and refused as a real secret.
const RETIRED_JWT_FALLBACK: &str = "dev-secret-change-in-production";

#[derive(Clone)]
pub struct Config {
    /// `DATABASE_URL`, required. Contains credentials; never log it.
    pub database_url: String,
    /// `JWT_SECRET`, required and non-empty. Never log it.
    pub jwt_secret: String,
    /// `PORT`, default 3000.
    pub port: u16,
    /// `CORS_ORIGIN`, default `http://localhost:8080`.
    pub cors_origin: HeaderValue,
    /// `UPLOADS_DIR`, default `data/uploads` relative to the working directory.
    pub uploads_dir: PathBuf,
    /// `RUST_LOG`, default `lugia=debug,tower_http=debug`.
    pub log_filter: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// A required variable is unset or empty.
    Missing(&'static str),
    /// A variable is set but cannot be used. The reason never contains the value.
    Invalid(&'static str, &'static str),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(var) => write!(f, "{var} must be set"),
            Self::Invalid(var, why) => write!(f, "{var} is invalid: {why}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Debug output redacts the two secret-bearing fields so a stray `{:?}` in a
/// log line cannot leak them.
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("database_url", &"<redacted>")
            .field("jwt_secret", &"<redacted>")
            .field("port", &self.port)
            .field("cors_origin", &self.cors_origin)
            .field("uploads_dir", &self.uploads_dir)
            .field("log_filter", &self.log_filter)
            .finish()
    }
}

impl Config {
    /// Read configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Read configuration through `lookup`, which returns `None` for an unset
    /// variable. Whitespace-only values count as unset.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let get = |name: &str| lookup(name).filter(|v| !v.trim().is_empty());

        let database_url = get("DATABASE_URL").ok_or(ConfigError::Missing("DATABASE_URL"))?;

        let jwt_secret = get("JWT_SECRET").ok_or(ConfigError::Missing("JWT_SECRET"))?;
        if jwt_secret == RETIRED_JWT_FALLBACK {
            return Err(ConfigError::Invalid(
                "JWT_SECRET",
                "this value was committed to the repository and is public; choose a new one",
            ));
        }

        let port = match get("PORT") {
            None => 3000,
            Some(p) => p
                .parse()
                .map_err(|_| ConfigError::Invalid("PORT", "not a number in 0..=65535"))?,
        };

        let cors_origin = get("CORS_ORIGIN")
            .unwrap_or_else(|| "http://localhost:8080".to_string())
            .parse::<HeaderValue>()
            .map_err(|_| ConfigError::Invalid("CORS_ORIGIN", "not a valid header value"))?;

        let uploads_dir =
            PathBuf::from(get("UPLOADS_DIR").unwrap_or_else(|| "data/uploads".to_string()));

        let log_filter =
            get("RUST_LOG").unwrap_or_else(|| "lugia=debug,tower_http=debug".to_string());

        Ok(Self { database_url, jwt_secret, port, cors_origin, uploads_dir, log_filter })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> =
            pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |name| map.get(name).cloned()
    }

    const BASE: &[(&str, &str)] =
        &[("DATABASE_URL", "postgres://u:p@localhost/db"), ("JWT_SECRET", "unit-test-secret")];

    #[test]
    fn required_only_uses_defaults() {
        let cfg = Config::from_lookup(env(BASE)).unwrap();
        assert_eq!(cfg.port, 3000);
        assert_eq!(cfg.cors_origin, "http://localhost:8080");
        assert_eq!(cfg.uploads_dir, PathBuf::from("data/uploads"));
        assert_eq!(cfg.log_filter, "lugia=debug,tower_http=debug");
    }

    #[test]
    fn missing_jwt_secret_fails() {
        let err = Config::from_lookup(env(&[BASE[0]])).unwrap_err();
        assert_eq!(err, ConfigError::Missing("JWT_SECRET"));
    }

    #[test]
    fn blank_jwt_secret_counts_as_missing() {
        let err = Config::from_lookup(env(&[BASE[0], ("JWT_SECRET", "   ")])).unwrap_err();
        assert_eq!(err, ConfigError::Missing("JWT_SECRET"));
    }

    #[test]
    fn retired_fallback_secret_is_refused() {
        let err =
            Config::from_lookup(env(&[BASE[0], ("JWT_SECRET", RETIRED_JWT_FALLBACK)])).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid("JWT_SECRET", _)));
    }

    #[test]
    fn missing_database_url_fails() {
        let err = Config::from_lookup(env(&[BASE[1]])).unwrap_err();
        assert_eq!(err, ConfigError::Missing("DATABASE_URL"));
    }

    #[test]
    fn errors_never_contain_values() {
        let secret = "s3cr3t-value";
        let url = "postgres://user:pw@host/db";
        let err = Config::from_lookup(env(&[("DATABASE_URL", url), ("JWT_SECRET", secret), ("PORT", "nope")]))
            .unwrap_err();
        let text = err.to_string();
        assert!(!text.contains(secret) && !text.contains(url) && !text.contains("nope"), "{text}");
    }

    #[test]
    fn debug_output_redacts_secrets() {
        let cfg = Config::from_lookup(env(BASE)).unwrap();
        let text = format!("{cfg:?}");
        assert!(!text.contains("unit-test-secret") && !text.contains("postgres://"), "{text}");
        assert!(text.contains("<redacted>"));
    }

    #[test]
    fn optional_overrides_apply() {
        let mut pairs = BASE.to_vec();
        pairs.extend([("PORT", "8081"), ("CORS_ORIGIN", "https://app.example"), ("UPLOADS_DIR", "/var/up")]);
        let cfg = Config::from_lookup(env(&pairs)).unwrap();
        assert_eq!(cfg.port, 8081);
        assert_eq!(cfg.cors_origin, "https://app.example");
        assert_eq!(cfg.uploads_dir, PathBuf::from("/var/up"));
    }
}
