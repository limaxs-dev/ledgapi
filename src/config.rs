//! Application configuration loaded from environment variables with
//! prefix `APP__`. Sections use `__` as separator (e.g.
//! `APP__SERVER__BIND`).

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Top-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub embed: EmbedConfig,
    pub log: LogConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub initial_admin_username: Option<String>,
    pub initial_admin_password: Option<String>,
    #[serde(default = "default_issuer")]
    pub issuer: String,
    #[serde(with = "humantime_serde", default = "default_session_ttl")]
    pub session_ttl: Duration,
    #[serde(with = "humantime_serde", default = "default_access_token_ttl")]
    pub access_token_ttl: Duration,
    #[serde(with = "humantime_serde", default = "default_refresh_token_ttl")]
    pub refresh_token_ttl: Duration,
    #[serde(with = "humantime_serde", default = "default_authorization_code_ttl")]
    pub authorization_code_ttl: Duration,
    #[serde(default)]
    pub cookie_secure: bool,
}

fn default_issuer() -> String {
    "http://localhost:18080".to_owned()
}

fn default_session_ttl() -> Duration {
    Duration::from_hours(8)
}

fn default_access_token_ttl() -> Duration {
    Duration::from_hours(1)
}

fn default_refresh_token_ttl() -> Duration {
    Duration::from_hours(24 * 30)
}

fn default_authorization_code_ttl() -> Duration {
    Duration::from_mins(1)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address, e.g. `0.0.0.0:18080`.
    pub bind: String,
    /// Maximum time to wait for in-flight requests to finish on SIGTERM.
    #[serde(with = "humantime_serde", default = "default_shutdown_timeout")]
    pub shutdown_timeout: Duration,
}

fn default_shutdown_timeout() -> Duration {
    Duration::from_secs(30)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Path to the SQLite file. Parent directory is auto-created.
    pub path: String,
    /// Busy timeout in milliseconds (passed to `PRAGMA busy_timeout`).
    #[serde(default = "default_busy_timeout")]
    pub busy_timeout_ms: u64,
}

fn default_busy_timeout() -> u64 {
    5000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedConfig {
    /// Directory where fastembed caches the ONNX model.
    pub cache_dir: String,
    /// fastembed model identifier. Defaults to `all-MiniLM-L6-v2`.
    #[serde(default = "default_embed_model")]
    pub model: String,
    /// Cosine-similarity threshold for the dup-check on `create_contract`.
    /// Contracts with similarity >= threshold trigger a `warning_similar_found`.
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,
    /// Number of candidates the dup-check retrieves.
    #[serde(default = "default_knn_top_k")]
    pub knn_top_k: i64,
    /// Default `limit` for `search_contract` when not specified.
    #[serde(default = "default_hybrid_limit")]
    pub hybrid_limit: i64,
}

fn default_embed_model() -> String {
    "sentence-transformers/all-MiniLM-L6-v2".to_owned()
}
fn default_similarity_threshold() -> f32 {
    0.85
}
fn default_knn_top_k() -> i64 {
    5
}
fn default_hybrid_limit() -> i64 {
    10
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Pretty,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    #[serde(default)]
    pub format: LogFormat,
    /// env-filter style directive, e.g. `"info,ledgapi=debug"`.
    #[serde(default = "default_log_level")]
    pub level: String,
}

fn default_log_level() -> String {
    "info".to_owned()
}

impl AppConfig {
    /// Load config from environment. Reads `.env` via `dotenvy` first.
    ///
    /// # Errors
    /// Returns `Err` if the configuration is missing required keys or
    /// has invalid values (e.g., bad `bind` address).
    pub fn from_env() -> anyhow::Result<Self> {
        use config::{Config as Cfg, Environment};
        let _ = dotenvy::dotenv();

        let cfg = Cfg::builder()
            .set_default("server.bind", "0.0.0.0:18080")?
            .set_default("database.path", "/data/ledgapi.db")?
            .set_default("embed.cache_dir", "/data/.cache/fastembed")?
            .set_default("log.format", "pretty")?
            .set_default("log.level", "info")?
            .set_default("auth.issuer", "http://localhost:18080")?
            .set_default("auth.cookie_secure", false)?
            .add_source(Environment::with_prefix("APP").separator("__").try_parsing(true))
            .build()?;

        let parsed: AppConfig = cfg.try_deserialize()?;
        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        // Test the default helpers directly since we can't easily set env vars in unit tests.
        assert_eq!(default_shutdown_timeout(), Duration::from_secs(30));
        assert_eq!(default_busy_timeout(), 5000);
        assert_eq!(default_embed_model(), "sentence-transformers/all-MiniLM-L6-v2");
        assert!((default_similarity_threshold() - 0.85).abs() < 1e-6);
        assert_eq!(default_knn_top_k(), 5);
        assert_eq!(default_hybrid_limit(), 10);
        assert_eq!(default_log_level(), "info");
    }

    #[test]
    fn log_format_default_is_pretty() {
        assert_eq!(LogFormat::default(), LogFormat::Pretty);
    }
}
