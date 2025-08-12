use crate::api::traits::BytebaseApi;
use crate::cli::EnvCommand;
use crate::config::{self, Environment};
use anyhow::Result;

/// Handles the `env` command by creating a live API client and dispatching to the appropriate sub-command.
pub async fn handle_env_command<T: BytebaseApi>(command: EnvCommand, client: &T) -> Result<()> {
    match command {
        EnvCommand::Add {
            name,
            project,
            instance,
        } => add_env(client, &name, &project, &instance).await,
        EnvCommand::List => list_envs().await,
        EnvCommand::Remove { name } => remove_env(&name).await,
    }
}

async fn add_env<T: BytebaseApi>(
    api_client: &T,
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

    let mut config = config::load_config().await?;
    let new_env = Environment {
        project: project.to_string(),
        instance: instance.to_string(),
    };
    config.environments.insert(name.to_string(), new_env);
    config::save_config(&config).await?;

    println!("\nSuccessfully added environment '{name}' for project '{project}'.");
    Ok(())
}

async fn list_envs() -> Result<()> {
    let config = config::load_config().await?;
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

async fn remove_env(name: &str) -> Result<()> {
    let mut config = config::load_config().await?;
    if config.environments.remove(name).is_some() {
        config::save_config(&config).await?;
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
    use crate::config::{self, Credentials};
    use tempfile::tempdir;

    async fn run_in_temp_home<F, Fut>(test_body: F)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let temp_dir = tempdir().unwrap();
        let home_path = temp_dir.path();
        let original_home = std::env::var("HOME");
        unsafe {
            std::env::set_var("HOME", home_path);
        }

        // Pre-seed the config with login credentials for tests.
        let mut config = config::load_config().await.unwrap();
        config.credentials = Some(Credentials {
            url: "https://fake-url.com".to_string(),
            service_account: "fake-service-account".to_string(),
            service_key: Some("fake-service-key".to_string()),
            access_token: "fake-access-token".to_string(),
        });
        config::save_config(&config).await.unwrap();

        test_body().await;

        unsafe {
            if let Ok(val) = original_home {
                std::env::set_var("HOME", val);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }

    #[tokio::test]
    async fn test_add_existing_project() {
        run_in_temp_home(|| async {
            let fake_client = FakeApiClient {
                projects: HashMap::new(),
            };
            let add_command = EnvCommand::Add {
                name: "dev".to_string(),
                project: "existing-project".to_string(),
                instance: "existing-instance".to_string(),
            };

            let result = handle_env_command(add_command, &fake_client).await;
            assert!(result.is_ok());

            let config = config::load_config().await.unwrap();
            assert!(config.environments.contains_key("dev"));
        })
        .await;
    }

    #[tokio::test]
    async fn test_add_non_existing_project() {
        run_in_temp_home(|| async {
            let fake_client = FakeApiClient {
                projects: HashMap::new(),
            };
            let add_command = EnvCommand::Add {
                name: "dev".to_string(),
                project: "non-existing-project".to_string(),
                instance: "existing-instance".to_string(),
            };

            let result = handle_env_command(add_command, &fake_client).await;
            assert!(result.is_err());
        })
        .await;
    }
}
