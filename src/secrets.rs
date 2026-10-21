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
    /// Loads secrets from the default path in the user's home configuration directory
    /// (`<config_dir>/marketfeed/secrets.json`), returning [`Self::default()`] if the file
    /// does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`MarketfeedSecretsError`] if the home configuration directory cannot be
    /// determined or if reading or parsing an existing secrets file fails.
    pub fn from_default_path() -> Result<Self, MarketfeedSecretsError> {
        let dirs = ProjectDirs::from("", "", "marketfeed")
            .ok_or(MarketfeedSecretsError::HomeConfigDirUnavailable)?;
        let path = dirs.config_dir().join("secrets.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        Self::from_path(&path)
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
        let secrets_file = temp_dir.path().join("secrets.json");
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
}
