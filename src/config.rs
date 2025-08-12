use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Represents the main configuration for the application, stored in `~/.shelltide/config.json`.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AppConfig {
    /// Default source environment for `apply` commands.
    pub default_source_env: Option<String>,
    /// Bytebase instance credentials.
    pub credentials: Option<Credentials>,
    /// A map of environment names to their configuration details.
    #[serde(default)]
    pub environments: HashMap<String, Environment>,
    /// A map of release names to their details.
    #[serde(default)]
    pub releases: HashMap<String, Release>,
}

/// Stores details for a single release.
#[derive(Serialize, Deserialize, Debug)]
pub struct Release {
    /// The environment this release was created from.
    pub from_env: String,
    /// The latest issue number included in this release.
    pub issue_number: u32,
    /// The project name from which the issues are sourced.
    pub source_project: String,
}

/// Stores authentication credentials for the Bytebase API.
#[derive(Serialize, Deserialize, Debug)]
pub struct Credentials {
    pub url: String,
    pub service_account: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_key: Option<String>,
    pub access_token: String,
}

/// Stores details for a single environment.
#[derive(Serialize, Deserialize, Debug)]
pub struct Environment {
    /// The corresponding project name or ID in Bytebase.
    pub project: String,
    /// The instance name
    pub instance: String,
}

/// Returns the path to the shelltide configuration directory, `~/.shelltide`.
fn get_config_dir() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().context("Failed to find home directory")?;
    Ok(home_dir.join(".shelltide"))
}

/// Returns the full path to the configuration file, `~/.shelltide/config.json`.
fn get_config_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("config.json"))
}

/// Loads the application configuration from the default path.
/// If the config file or directory doesn't exist, it returns a default, empty config.
pub async fn load_config() -> Result<AppConfig> {
    let config_path = get_config_path()?;
    if !config_path.exists() {
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(&config_path)
        .await
        .with_context(|| format!("Failed to read config file at {config_path:?}"))?;

    let config: AppConfig = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file at {config_path:?}"))?;

    Ok(config)
}

/// Saves the provided application configuration to the default path.
/// It will create the necessary directory and file if they don't exist.
pub async fn save_config(config: &AppConfig) -> Result<()> {
    let config_path = get_config_path()?;
    let config_dir = config_path.parent().unwrap_or_else(|| Path::new(""));

    if !config_dir.exists() {
        fs::create_dir_all(config_dir)
            .await
            .with_context(|| format!("Failed to create config directory at {config_dir:?}"))?;
    }

    let content = serde_json::to_string_pretty(config)
        .context("Failed to serialize configuration to JSON")?;

    fs::write(&config_path, content)
        .await
        .with_context(|| format!("Failed to write config file to {config_path:?}"))?;

    Ok(())
}
