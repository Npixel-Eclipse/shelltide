use crate::api::traits::BytebaseApi;
use crate::cli::EnvCommand;
use crate::config::{ConfigOperations, Environment, ProductionConfig};
use anyhow::Result;

/// Handles the `env` command by creating a live API client and dispatching to the appropriate sub-command.
pub async fn handle_env_command<T: BytebaseApi>(command: EnvCommand, client: &T) -> Result<()> {
    let config_ops = ProductionConfig;
    handle_env_command_with_config(command, client, &config_ops).await
}

/// Internal function that accepts dependency-injected config operations
pub async fn handle_env_command_with_config<T: BytebaseApi, C: ConfigOperations>(
    command: EnvCommand,
    client: &T,
    config_ops: &C,
) -> Result<()> {
    match command {
        EnvCommand::Add {
            name,
            project,
            instance,
        } => add_env_with_config(client, config_ops, &name, &project, &instance).await,
        EnvCommand::List => list_envs_with_config(config_ops).await,
        EnvCommand::Remove { name } => remove_env_with_config(config_ops, &name).await,
    }
}

async fn add_env_with_config<T: BytebaseApi, C: ConfigOperations>(
    api_client: &T,
    config_ops: &C,
    name: &str,
    project: &str,
    instance: &str,
) -> Result<()> {
    print!("Verifying project '{project}'...");
    match api_client.get_project(project).await {
        Ok(p) => println!(" ✅ Found project '{}'.", p.title),
        Err(e) => {
            println!(" ❌ FAILED");
            return Err(e.into());
        }
    }

    print!("Verifying instance '{instance}'...");
    match api_client.get_instance(instance).await {
        Ok(i) => println!(" ✅ Found instance '{}'.", i.name),
        Err(e) => {
            println!(" ❌ FAILED");
            return Err(e.into());
        }
    }

    let mut config = config_ops.load_config().await?;
    let new_env = Environment {
        project: project.to_string(),
        instance: instance.to_string(),
    };
    config.environments.insert(name.to_string(), new_env);
    config_ops.save_config(&config).await?;

    println!("\nSuccessfully added environment '{name}' for project '{project}'.");
    Ok(())
}

async fn list_envs_with_config<C: ConfigOperations>(config_ops: &C) -> Result<()> {
    let config = config_ops.load_config().await?;
    if config.environments.is_empty() {
        println!("No environments configured. Use `env add` to add one.");
        return Ok(());
    }

    println!("{:<15} {:<30}", "NAME", "PROJECT");
    println!("{:-<15} {:-<30}", "", "");
    for (name, env) in config.environments {
        println!("{:<15} {:<30}", name, env.project);
    }
    Ok(())
}

async fn remove_env_with_config<C: ConfigOperations>(config_ops: &C, name: &str) -> Result<()> {
    let mut config = config_ops.load_config().await?;
    if config.environments.remove(name).is_some() {
        config_ops.save_config(&config).await?;
        println!("Removed environment '{name}'.");
    } else {
        println!("Error: Environment '{name}' not found.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::api::clients::tests::FakeApiClient;
    use crate::config::{self, Credentials, TestConfig};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_add_existing_project() {
        // Test with completely isolated config using dependency injection
        let temp_dir = tempdir().unwrap();
        let test_config = TestConfig {
            test_dir: temp_dir.path().to_path_buf(),
        };

        // Initialize test config with credentials
        let mut config = config::AppConfig::default();
        config.credentials = Some(Credentials {
            url: "https://fake-url.com".to_string(),
            service_account: "fake-service-account".to_string(),
            service_key: Some("fake-service-key".to_string()),
            access_token: "fake-access-token".to_string(),
        });
        test_config.save_config(&config).await.unwrap();

        // Test the add_env function with dependency injection
        let fake_client = FakeApiClient {
            projects: HashMap::new(),
        };

        let add_command = EnvCommand::Add {
            name: "dev".to_string(),
            project: "existing-project".to_string(),
            instance: "existing-instance".to_string(),
        };

        // This should now work completely in isolation
        let result = handle_env_command_with_config(add_command, &fake_client, &test_config).await;
        assert!(result.is_ok());

        // Verify the environment was added correctly to the test config
        let loaded_config = test_config.load_config().await.unwrap();
        assert!(loaded_config.environments.contains_key("dev"));
        assert_eq!(
            loaded_config.environments.get("dev").unwrap().project,
            "existing-project"
        );
    }

    #[tokio::test]
    async fn test_add_non_existing_project() {
        // Test with completely isolated config using dependency injection
        let temp_dir = tempdir().unwrap();
        let test_config = TestConfig {
            test_dir: temp_dir.path().to_path_buf(),
        };

        // Initialize test config with credentials
        let mut config = config::AppConfig::default();
        config.credentials = Some(Credentials {
            url: "https://fake-url.com".to_string(),
            service_account: "fake-service-account".to_string(),
            service_key: Some("fake-service-key".to_string()),
            access_token: "fake-access-token".to_string(),
        });
        test_config.save_config(&config).await.unwrap();

        // Test that adding non-existing project fails
        let fake_client = FakeApiClient {
            projects: HashMap::new(),
        };

        let add_command = EnvCommand::Add {
            name: "dev".to_string(),
            project: "non-existing-project".to_string(),
            instance: "existing-instance".to_string(),
        };

        // This should fail because the project doesn't exist in FakeApiClient
        let result = handle_env_command_with_config(add_command, &fake_client, &test_config).await;
        assert!(result.is_err());

        // Verify no environment was added to the test config
        let loaded_config = test_config.load_config().await.unwrap();
        assert!(!loaded_config.environments.contains_key("dev"));
    }
}
