use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Errors that can occur when loading [`MarketfeedSecrets`] from disk.
#[derive(Debug, thiserror::Error)]
pub enum MarketfeedSecretsError {
    /// Could not determine the user's home configuration directory.
    #[error("could not determine user home configuration directory for marketfeed secrets")]
    HomeConfigDirUnavailable,

    /// Failed to read a secrets file from disk.
    #[error("failed to read secrets file '{path}': {source}")]
    Io {
        /// Path to the secrets file that could not be read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// Failed to parse a secrets file as JSON.
    #[error("failed to parse secrets file '{path}' as JSON: {source}")]
    Json {
        /// Path to the secrets file that could not be parsed.
        path: PathBuf,
        /// Underlying JSON deserialization error.
        #[source]
        source: serde_json::Error,
    },
}

/// API credentials for `marketfeed` online data providers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MarketfeedSecrets {
    /// Optional Marketstack API access key.
    pub marketstack_access_key: Option<String>,
    /// Optional CoinMarketCap Pro API key.
    pub coinmarketcap_api_key: Option<String>,
    /// Optional OpenExchangeRates application ID.
    pub oxr_app_id: Option<String>,
}

impl MarketfeedSecrets {
    /// Standard secrets file name searched inside configuration directories (`"secrets.json"`).
    pub const FILE_NAME: &'static str = "secrets.json";

    /// Loads secrets from the default path in the user's home configuration directory
    /// (`<config_dir>/marketfeed/secrets.json`), returning [`Self::default()`] if the file
    /// does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`MarketfeedSecretsError`] if the home configuration directory cannot be
    /// determined or if reading or parsing an existing secrets file fails.
    pub fn from_default_path() -> Result<Self, MarketfeedSecretsError> {
        Self::resolve_with_fallback(None, Self::default_config_dir)
    }

    /// Searches `config_dir` for [`Self::FILE_NAME`] (`<config_dir>/secrets.json`) first,
    /// taking precedence over the default `marketfeed` configuration directory if a file
    /// exists there, and falling back to [`Self::from_default_path()`] otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`MarketfeedSecretsError`] if an existing secrets file in `config_dir` or the
    /// fallback default directory cannot be read or parsed, or if the fallback home
    /// configuration directory is unavailable when `config_dir` contains no secrets file.
    pub fn from_search_path(config_dir: &Path) -> Result<Self, MarketfeedSecretsError> {
        Self::resolve_with_fallback(Some(config_dir), Self::default_config_dir)
    }

    /// Loads secrets from a custom JSON file path.
    ///
    /// # Errors
    ///
    /// Returns [`MarketfeedSecretsError::Io`] if the file cannot be read or
    /// [`MarketfeedSecretsError::Json`] if the content is not valid JSON.
    pub fn from_path(path: &Path) -> Result<Self, MarketfeedSecretsError> {
        let content = fs::read_to_string(path).map_err(|source| MarketfeedSecretsError::Io {
            path: path.to_path_buf(),
            source,
        })?;

        serde_json::from_str(&content).map_err(|source| MarketfeedSecretsError::Json {
            path: path.to_path_buf(),
            source,
        })
    }

    fn default_config_dir() -> Result<PathBuf, MarketfeedSecretsError> {
        let dirs = ProjectDirs::from("", "", "marketfeed")
            .ok_or(MarketfeedSecretsError::HomeConfigDirUnavailable)?;
        Ok(dirs.config_dir().to_path_buf())
    }

    fn resolve_with_fallback<F>(
        search_dir: Option<&Path>,
        default_dir: F,
    ) -> Result<Self, MarketfeedSecretsError>
    where
        F: FnOnce() -> Result<PathBuf, MarketfeedSecretsError>,
    {
        if let Some(dir) = search_dir {
            let candidate = dir.join(Self::FILE_NAME);
            if candidate.is_file() {
                return Self::from_path(&candidate);
            }
        }

        let fallback = default_dir()?.join(Self::FILE_NAME);
        if fallback.is_file() {
            return Self::from_path(&fallback);
        }

        Ok(Self::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;
    use tempfile::TempDir;

    #[googletest::test]
    fn from_path_deserializes_secrets_json() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        let temp_dir = TempDir::new()?;
        let secrets_file = temp_dir.path().join(MarketfeedSecrets::FILE_NAME);
        fs::write(
            &secrets_file,
            r#"{
                "marketstack_access_key": "file-ms-key",
                "coinmarketcap_api_key": "file-cmc-key",
                "oxr_app_id": "file-oxr-id"
            }"#,
        )?;

        let loaded = MarketfeedSecrets::from_path(&secrets_file)?;
        let expected = MarketfeedSecrets {
            marketstack_access_key: Some("file-ms-key".to_string()),
            coinmarketcap_api_key: Some("file-cmc-key".to_string()),
            oxr_app_id: Some("file-oxr-id".to_string()),
        };
        assert_that!(&loaded, eq(&expected));
        Ok(())
    }

    #[googletest::test]
    fn from_path_missing_file_returns_io_error() {
        let missing = Path::new("/nonexistent/marketfeed/secrets.json");
        let result = MarketfeedSecrets::from_path(missing);
        assert_that!(result, err(pat!(MarketfeedSecretsError::Io { .. })));
    }

    #[googletest::test]
    fn from_search_path_prefers_custom_config_dir_over_default_dir()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let custom_dir = TempDir::new()?;
        let fallback_dir = TempDir::new()?;
        fs::write(
            custom_dir.path().join(MarketfeedSecrets::FILE_NAME),
            r#"{"marketstack_access_key": "custom-priority-key"}"#,
        )?;
        fs::write(
            fallback_dir.path().join(MarketfeedSecrets::FILE_NAME),
            r#"{"marketstack_access_key": "fallback-key", "oxr_app_id": "fallback-oxr"}"#,
        )?;

        let direct = MarketfeedSecrets::from_search_path(custom_dir.path())?;
        assert_that!(
            direct.marketstack_access_key.as_deref(),
            some(eq("custom-priority-key"))
        );
        assert_that!(direct.oxr_app_id, none());

        let resolved = MarketfeedSecrets::resolve_with_fallback(Some(custom_dir.path()), || {
            Ok(fallback_dir.path().to_path_buf())
        })?;
        assert_that!(
            resolved.marketstack_access_key.as_deref(),
            some(eq("custom-priority-key"))
        );
        assert_that!(resolved.oxr_app_id, none());
        Ok(())
    }

    #[googletest::test]
    fn resolve_with_fallback_loads_default_dir_or_empty_when_custom_missing()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let empty_custom_dir = TempDir::new()?;
        let fallback_dir = TempDir::new()?;
        fs::write(
            fallback_dir.path().join(MarketfeedSecrets::FILE_NAME),
            r#"{"oxr_app_id": "fallback-oxr-id"}"#,
        )?;

        let fallback_loaded =
            MarketfeedSecrets::resolve_with_fallback(Some(empty_custom_dir.path()), || {
                Ok(fallback_dir.path().to_path_buf())
            })?;
        assert_that!(
            fallback_loaded.oxr_app_id.as_deref(),
            some(eq("fallback-oxr-id"))
        );

        let empty_fallback_dir = TempDir::new()?;
        let default_empty =
            MarketfeedSecrets::resolve_with_fallback(Some(empty_custom_dir.path()), || {
                Ok(empty_fallback_dir.path().to_path_buf())
            })?;
        assert_that!(&default_empty, eq(&MarketfeedSecrets::default()));
        Ok(())
    }

    #[googletest::test]
    fn from_search_path_surfaces_malformed_json_error_in_custom_dir()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let custom_dir = TempDir::new()?;
        fs::write(
            custom_dir.path().join(MarketfeedSecrets::FILE_NAME),
            "{ invalid json",
        )?;

        let result = MarketfeedSecrets::from_search_path(custom_dir.path());
        assert_that!(result, err(pat!(MarketfeedSecretsError::Json { .. })));
        Ok(())
    }
}
