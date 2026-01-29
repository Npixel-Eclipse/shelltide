use crate::api::traits::BytebaseApi;
use crate::cli::StatusArgs;
use anyhow::Result;

pub async fn handle_status_command<T: BytebaseApi>(
    api_client: &mut T,
    args: StatusArgs,
) -> Result<()> {
    let config_ops = crate::config::ProductionConfig;
    handle_status_command_with_config(api_client, args, &config_ops).await
}

pub async fn handle_status_command_with_config<
    T: BytebaseApi,
    C: crate::config::ConfigOperations,
>(
    api_client: &mut T,
    args: StatusArgs,
    config_ops: &C,
) -> Result<()> {
    let config = config_ops.load_config().await?;

    if config.environments.is_empty() {
        println!("No environments configured. Use `env add` to add one.");
        return Ok(());
    }

    // Get default source environment for reference - must be configured
    let default_source_env = config.default_source_env.as_deref()
        .ok_or_else(|| anyhow::anyhow!(
            "Configuration error: default.source_env not set. Please run: shelltide config set default.source_env <env-name>"
        ))?;
    let default_env = config.environments.get(default_source_env).ok_or_else(|| {
        anyhow::anyhow!(
            "Default source environment '{}' not found in config",
            default_source_env
        )
    })?;

    // Get reference issue number from default environment
    let reference_issue_number = match api_client.get_done_issues(&default_env.project).await {
        Ok(issues) => issues
            .iter()
            .max_by_key(|issue| issue.name.number)
            .map(|issue| issue.name.number)
            .unwrap_or(0),
        Err(e) => {
            println!("Error getting reference issues from {default_source_env}: {e}");
            return Ok(());
        }
    };

    // Parse filter if provided
    let (filter_env, filter_db) = if let Some(filter) = &args.filter {
        if filter.contains('/') {
            let parts: Vec<&str> = filter.split('/').collect();
            if parts.len() == 2 {
                (Some(parts[0]), Some(parts[1]))
            } else {
                println!("Invalid filter format. Use '<env>/<database>' or just '<env>'");
                return Ok(());
            }
        } else {
            (Some(filter.as_str()), None)
        }
    } else {
        (None, None)
    };

    // Get databases that exist in default environment using API
    let default_databases = match api_client.get_databases(&default_env.instance).await {
        Ok(databases) => databases,
        Err(e) => {
            println!("Error getting databases from {default_source_env}: {e}");
            return Ok(());
        }
    };

    if default_databases.is_empty() {
        println!("No databases found in default environment '{default_source_env}'");
        return Ok(());
    }

    // Collect database status information
    let mut database_info = Vec::new();

    for (env_name, env) in &config.environments {
        // Skip environment if filter is specified and doesn't match
        if let Some(filter_env) = filter_env
            && env_name != filter_env
        {
            continue;
        }

        // Skip default environment when showing all environments (no filter)
        if filter_env.is_none() && env_name == default_source_env {
            continue;
        }

        let databases_to_check: Vec<String> = if let Some(filter_db) = filter_db {
            vec![filter_db.to_string()]
        } else {
            default_databases.clone()
        };

        for database_name in &databases_to_check {
            match api_client
                .get_latests_revisions_silent(&env.instance, database_name)
                .await
            {
                Ok(revision) => {
                    if let Some(version) = revision.version.as_ref() {
                        let current_issue = version.number;
                        let status = if current_issue >= reference_issue_number {
                            "UP TO DATE".to_string()
                        } else {
                            format!("#{current_issue}")
                        };

                        database_info.push((
                            format!("{}/{}", env.instance, database_name),
                            env_name.clone(),
                            status,
                        ));
                    } else {
                        database_info.push((
                            format!("{}/{}", env.instance, database_name),
                            env_name.clone(),
                            "NO VERSION".to_string(),
                        ));
                    }
                }
                Err(_) => {
                    database_info.push((
                        format!("{}/{}", env.instance, database_name),
                        env_name.clone(),
                        "NOT EXIST".to_string(),
                    ));
                }
            }
        }
    }

    // Sort by database name (extract from schema path) for consistent display
    database_info.sort_by(|a, b| {
        let db_a = a.0.split('/').next_back().unwrap_or(&a.0);
        let db_b = b.0.split('/').next_back().unwrap_or(&b.0);
        db_a.cmp(db_b).then_with(|| a.1.cmp(&b.1))
    });

    // Display status table
    print_status_table(&database_info);

    println!(
        "\nReference environment: {default_source_env} (latest issue: #{reference_issue_number})"
    );

    Ok(())
}

fn print_status_table(database_info: &[(String, String, String)]) {
    if database_info.is_empty() {
        return;
    }

    // Calculate dynamic column widths
    let mut max_schema_width = "SCHEMA".len();
    let mut max_env_width = "ENVIRONMENT".len();
    let max_status_width = "LATEST CHANGELOG".len();

    for (schema_path, env_name, _status) in database_info {
        max_schema_width = max_schema_width.max(schema_path.len());
        max_env_width = max_env_width.max(env_name.len());
    }

    max_schema_width += 1;
    max_env_width += 1;
    println!(
        "{:<width1$} {:<width2$} {:<width3$}",
        "SCHEMA",
        "ENVIRONMENT",
        "LATEST CHANGELOG",
        width1 = max_schema_width,
        width2 = max_env_width,
        width3 = max_status_width
    );
    println!(
        "{:-<width1$} {:-<width2$} {:-<width3$}",
        "",
        "",
        "",
        width1 = max_schema_width,
        width2 = max_env_width,
        width3 = max_status_width
    );

    for (schema_path, env_name, status) in database_info {
        println!(
            "{schema_path:<max_schema_width$} {env_name:<max_env_width$} {status:<max_status_width$}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::clients::tests::FakeApiClient;
    use crate::api::types::{Issue, IssueName};
    use crate::config::{ConfigOperations, Credentials, Environment};
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
        F: FnOnce(std::path::PathBuf) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        test_body(temp_path).await;
    }

    #[tokio::test]
    async fn test_status_command() {
        run_in_temp_home(|temp_path| async move {
            let temp_config = crate::config::TestConfig {
                test_dir: temp_path,
            };
            let mut test_config = crate::config::AppConfig::default();
            test_config.default_source_env = Some("dev".to_string());
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
            temp_config.save_config(&test_config).await.unwrap();
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

            let mut fake_client = FakeApiClient {
                projects: projects_data,
            };

            let status_args = crate::cli::StatusArgs { filter: None };
            let result =
                handle_status_command_with_config(&mut fake_client, status_args, &temp_config)
                    .await;

            assert!(result.is_ok());
        })
        .await;
    }
}
