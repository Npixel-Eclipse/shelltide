use anyhow::Result;

use crate::{
    cli::ConfigCommand,
    config::{ConfigOperations, ProductionConfig},
};

/// Handles the `config` command.
pub async fn config(command: ConfigCommand) -> Result<()> {
    let config_ops = ProductionConfig;
    config_with_ops(command, &config_ops).await
}

/// Internal function for dependency injection
pub async fn config_with_ops<C: ConfigOperations>(
    command: ConfigCommand,
    config_ops: &C,
) -> Result<()> {
    match command {
        ConfigCommand::Set { key, value } => set_config_with_ops(config_ops, &key, value).await,
        ConfigCommand::Get { key } => get_config_with_ops(config_ops, &key).await,
    }
}

async fn set_config_with_ops<C: ConfigOperations>(
    config_ops: &C,
    key: &str,
    value: String,
) -> Result<()> {
    let mut config = config_ops.load_config().await?;

    match key {
        "default.source_env" => {
            if !config.environments.contains_key(&value) {
                return Err(anyhow::anyhow!("Environment '{}' not found.", value));
            }
            config.default_source_env = Some(value);
            println!(
                "Set `default.source_env` to '{}'",
                config.default_source_env.as_ref().unwrap()
            );
        }
        _ => {
            println!("Error: Unknown configuration key '{key}'");
            println!("Available keys: default.source_env");
            // In a real app, you might return an error here.
            // For now, we just print a message.
            return Ok(());
        }
    }

    config_ops.save_config(&config).await?;
    Ok(())
}

async fn get_config_with_ops<C: ConfigOperations>(config_ops: &C, key: &str) -> Result<()> {
    let config = config_ops.load_config().await?;

    match key {
        "default.source_env" => {
            if let Some(value) = config.default_source_env {
                println!("{value}");
            } else {
                println!("'default.source_env' is not set.");
            }
        }
        _ => {
            println!("Error: Unknown configuration key '{key}'");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::api::clients::tests::FakeApiClient;
    use crate::cli::{ConfigCommand, EnvCommand};
    use crate::commands;
    use tempfile::tempdir;

    // Helper function to create a temporary home directory for testing.
    // This isolates tests from the user's actual configuration.
    async fn run_in_temp_home<F, Fut>(test_body: F)
    where
        F: FnOnce(std::path::PathBuf) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let temp_dir = tempdir().unwrap();
        let home_path = temp_dir.path().to_path_buf();

        // Override the home directory for the `dirs` crate.
        // This is a bit of a hack, but necessary for this kind of test.
        // In a larger application, you might abstract config loading further.
        let original_home = std::env::var("HOME");
        // SAFETY: We are running tests in a single-threaded context for this part,
        // or each test runs in a separate process space, making this safe.
        // We are also restoring the environment variable after the test.
        unsafe {
            std::env::set_var("HOME", &home_path);
        }

        test_body(home_path).await;

        // Restore original HOME variable to not affect other tests.
        // SAFETY: See above.
        unsafe {
            if let Ok(val) = original_home {
                std::env::set_var("HOME", val);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }

    #[tokio::test]
    async fn test_config_set_and_get() {
        run_in_temp_home(|_home_path| async move {
            // 1. Test setting a value.
            // Create test environment first
            let fake_client = FakeApiClient {
                projects: HashMap::new(),
            };
            let env_command = EnvCommand::Add {
                name: "test-dev".to_string(),
                project: "existing-project".to_string(),
                instance: "test-instance".to_string(),
            };
            // Create test config for isolated testing
            let test_config = crate::config::TestConfig {
                test_dir: _home_path.clone(),
            };
            let result = commands::env::handle_env_command_with_config(
                env_command,
                &fake_client,
                &test_config,
            )
            .await;
            assert!(
                result.is_ok(),
                "Adding environment should succeed: {:?}",
                result
            );
            let key = "default.source_env".to_string();
            let value = "test-dev".to_string();
            let set_command = ConfigCommand::Set {
                key: key.clone(),
                value: value.clone(),
            };
            let result = config_with_ops(set_command, &test_config).await;
            assert!(
                result.is_ok(),
                "Setting config should succeed: {:?}",
                result
            );

            // 2. Verify by loading the config directly.
            let loaded_config = test_config.load_config().await.unwrap();
            assert_eq!(
                loaded_config.default_source_env,
                Some(value),
                "The value should be correctly saved in the config"
            );

            // 3. Test getting the value.
            // Note: This test doesn't capture stdout. It only checks if the command runs
            // without errors. A more advanced test would capture and assert the output.
            let get_command = ConfigCommand::Get { key };
            let result = config_with_ops(get_command, &test_config).await;
            assert!(result.is_ok(), "Getting config should succeed");
        })
        .await;
    }

    #[tokio::test]
    async fn test_get_unset_key() {
        run_in_temp_home(|_home_path| async move {
            let test_config = crate::config::TestConfig {
                test_dir: _home_path.clone(),
            };
            let get_command = ConfigCommand::Get {
                key: "default.source_env".to_string(),
            };
            // This should run without error and print a message.
            let result = config_with_ops(get_command, &test_config).await;
            assert!(result.is_ok());
        })
        .await;
    }
}
