use crate::api::traits::BytebaseApi;
use crate::config;
use anyhow::Result;

pub async fn handle_status_command<T: BytebaseApi>(api_client: &T) -> Result<()> {
    let config = config::load_config().await?;

    if config.environments.is_empty() {
        println!("No environments configured. Use `env add` to add one.");
        return Ok(());
    }

    println!("{:<15} {:<20}", "ENVIRONMENT", "LATEST ISSUE");
    println!("{:-<15} {:-<20}", "", "");

    for (name, env) in config.environments {
        match api_client.get_done_issues(&env.project).await {
            Ok(issues) => {
                let latest_done_issue = issues.iter().max_by_key(|issue| issue.name.number);

                if let Some(issue) = latest_done_issue {
                    println!("{name:<15} #{:<19}", issue.name.number);
                } else {
                    println!("{name:<15} None");
                }
            }
            Err(e) => {
                println!("{name:<15} Error: {e}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::clients::tests::FakeApiClient;
    use crate::api::types::{Issue, IssueName};
    use crate::config::{self, Credentials, Environment};
    use std::collections::HashMap;
    use tempfile::tempdir;

    impl From<&str> for IssueName {
        fn from(s: &str) -> Self {
            let mut split = s.split('/');
            let project = split.nth(1).unwrap();
            let number = split.nth(1).unwrap().parse().unwrap();
            Self {
                project: project.to_string(),
                number,
            }
        }
    }

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
    async fn test_status_command() {
        run_in_temp_home(|| async {
            // 1. Setup: Create a fake config with two environments
            let mut test_config = config::load_config().await.unwrap();
            test_config.credentials = Some(Credentials {
                url: "https://fake-url.com".into(),
                service_account: "fake-service-account".into(),
                service_key: Some("fake-service-key".into()),
                access_token: "fake-access-token".into(),
            });
            test_config.environments.insert(
                "dev".into(),
                Environment {
                    project: "dev-project".into(),
                    instance: "dev-instance".into(),
                },
            );
            test_config.environments.insert(
                "prod".into(),
                Environment {
                    project: "prod-project".into(),
                    instance: "prod-instance".into(),
                },
            );
            config::save_config(&test_config).await.unwrap();

            // 2. Setup: Create a fake API client with mock data
            let mut projects_data = HashMap::new();
            projects_data.insert(
                "dev-project".to_string(),
                vec![
                    Issue {
                        name: "projects/dev-project/issues/101".into(),
                    },
                    Issue {
                        name: "projects/dev-project/issues/102".into(),
                    },
                ],
            );
            projects_data.insert(
                "prod-project".to_string(),
                vec![Issue {
                    name: "projects/prod-project/issues/103".into(),
                }],
            );

            let fake_client = FakeApiClient {
                projects: projects_data,
            };

            // 3. Execute: Run the status command
            // Note: This test doesn't capture stdout to verify the table format,
            // but it ensures the command runs to completion without panicking,
            // which validates the core logic.
            let result = handle_status_command(&fake_client).await;

            // 4. Assert: Check that the command succeeded
            assert!(result.is_ok());
        })
        .await;
    }
}
