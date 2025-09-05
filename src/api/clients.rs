use crate::api::traits::BytebaseApi;
use crate::api::types::{
    ChangeDatabaseConfig, ChangeDatabaseConfigType, Changelog, Instance, Issue, IssueName,
    IssuesResponse, LoginRequest, LoginResponse, PlanName, PlanStep, PlanStepSpec,
    PostIssuesResponse, PostPlansRequest, PostPlansResponse, PostSheetsResponse, Project, Revision,
    SheetName, SheetRequest, SqlCheckRequest,
};
use crate::config::Credentials;
use crate::error::AppError;
use async_trait::async_trait;
use reqwest::header;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::json;
use uuid::Uuid;

pub async fn get_access_token(
    base_url: &str,
    service_account: &str,
    service_key: &str,
) -> Result<LoginResponse, AppError> {
    let client = reqwest::Client::new();
    let login_url = format!("{base_url}/v1/auth/login");
    let request = LoginRequest {
        email: service_account.to_string(),
        password: service_key.to_string(),
        web: true,
    };
    let response = client.post(&login_url).json(&request).send().await?;
    Ok(response.json().await?)
}

/// A client for interacting with the live Bytebase API.
#[derive(Debug)]
pub struct LiveApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl LiveApiClient {
    /// Helper function to handle API responses with consistent error logging
    async fn handle_response<T: serde::de::DeserializeOwned>(
        response: reqwest::Response,
        operation: &str,
    ) -> Result<T, AppError> {
        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            println!("{operation} failed - Status: {status}, Response: {response_text}",);
            return Err(AppError::ApiError(format!(
                "{operation} failed. Status: {status}, Response: {response_text}",
            )));
        }

        match serde_json::from_str::<T>(&response_text) {
            Ok(result) => Ok(result),
            Err(e) => {
                println!(
                    "Failed to parse {operation} response - Status: {status}, Response: {response_text}",
                );
                Err(AppError::ApiError(format!(
                    "Failed to parse {operation} response: {e}",
                )))
            }
        }
    }

    /// Creates a new API client with the given credentials.
    pub fn new(credentials: &Credentials) -> Result<Self, AppError> {
        let mut headers = HeaderMap::new();
        let auth_value = format!("Bearer {}", credentials.access_token);
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&auth_value)
                .map_err(|_| AppError::Config("Invalid authentication token".to_string()))?,
        );
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            base_url: credentials.url.clone(),
        })
    }

    pub fn login(&mut self, credentials: &Credentials) -> Result<(), AppError> {
        let mut headers = HeaderMap::new();
        let auth_value = format!("Bearer {}", credentials.access_token);
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&auth_value)
                .map_err(|_| AppError::Config("Invalid authentication token".to_string()))?,
        );
        self.client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(())
    }

    /// Ensures the client is authenticated with a valid token, refreshing if necessary
    pub async fn ensure_authenticated(&mut self) -> Result<(), AppError> {
        // Token validation by trying to list projects (most basic authenticated endpoint)
        let url = format!("{}/v1/projects", self.base_url);
        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED
            || response.status() == reqwest::StatusCode::FORBIDDEN
        {
            println!("Token expired, attempting to refresh...");

            // Load current credentials
            let config = crate::config::load_config().await?;
            let credentials = config.get_credentials()?;

            // Check if we have service_key for refresh
            if let Some(service_key) = &credentials.service_key {
                let login_response =
                    get_access_token(&credentials.url, &credentials.service_account, service_key)
                        .await?;

                // Update credentials and save to config
                let mut updated_credentials = credentials.clone();
                updated_credentials.access_token = login_response.token;

                let mut updated_config = config;
                updated_config.credentials = Some(updated_credentials.clone());
                crate::config::save_config(&updated_config).await?;

                // Update client with new token
                self.login(&updated_credentials)?;

                println!("Token refreshed successfully.");
                Ok(())
            } else {
                Err(AppError::Config(
                    "No service key available for token refresh. Please login again.".to_string(),
                ))
            }
        } else {
            // Token is still valid
            Ok(())
        }
    }
}

#[async_trait]
impl BytebaseApi for LiveApiClient {
    async fn get_project(&self, project_name: &str) -> Result<Project, AppError> {
        let url = format!("{}/v1/projects/{}", self.base_url, project_name);
        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::ApiError(format!(
                "Project '{project_name}' not found."
            )));
        }

        Self::handle_response(response, &format!("Get project '{project_name}'")).await
    }

    async fn get_instance(&self, instance_name: &str) -> Result<Instance, AppError> {
        let url = format!("{}/v1/instances/{}", self.base_url, instance_name);
        let response = self.client.get(&url).send().await?;
        Self::handle_response(response, &format!("Get instance '{instance_name}'")).await
    }

    async fn get_done_issues(&self, project_name: &str) -> Result<Vec<Issue>, AppError> {
        let url = format!(
            "{}/v1/projects/{}/issues?filter=status=\"DONE\"",
            self.base_url, project_name
        );
        let response = self.client.get(&url).send().await?;
        let res_json: IssuesResponse = Self::handle_response(
            response,
            &format!("Get done issues for project '{project_name}'"),
        )
        .await?;
        Ok(res_json.issues)
    }

    async fn create_sheet(
        &self,
        target_project_name: &str,
        sheet: SheetRequest,
    ) -> Result<PostSheetsResponse, AppError> {
        let url = format!(
            "{}/v1/projects/{}/sheets",
            self.base_url, target_project_name
        );
        let response = self.client.post(&url).json(&sheet).send().await?;
        Self::handle_response(
            response,
            &format!("Create sheet for project '{target_project_name}'"),
        )
        .await
    }

    /// For now, createing a new Database is not supported.  
    async fn create_plan(
        &self,
        project: &str,
        target_instance: &str,
        target_database: &str,
        sheet_name: SheetName,
    ) -> Result<PostPlansResponse, AppError> {
        let url = format!("{}/v1/projects/{project}/plans", self.base_url);
        let steps = vec![PlanStep {
            specs: vec![PlanStepSpec {
                id: Uuid::new_v4(),
                change_database_config: ChangeDatabaseConfig {
                    target: format!("instances/{target_instance}/databases/{target_database}"),
                    sheet: sheet_name,
                    config_type: ChangeDatabaseConfigType::Migrate,
                },
            }],
        }];

        let plan = PostPlansRequest { steps };
        let response = self.client.post(&url).json(&plan).send().await?;
        Self::handle_response(response, &format!("Create plan for project '{project}'")).await
    }

    async fn create_rollout(
        &self,
        target_project_name: &str,
        plan_name: PlanName,
        issue_name: IssueName,
    ) -> Result<(), AppError> {
        let url = format!(
            "{}/v1/projects/{}/rollouts",
            self.base_url, target_project_name
        );

        let body = json!({
            "plan": plan_name,
            "issue": issue_name,
        });
        let response = self.client.post(&url).json(&body).send().await?;
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::ApiError(format!(
                "Failed to create rollout: {error_body}"
            )));
        }
        Ok(())
    }

    async fn create_issue(
        &self,
        project_name: &str,
        plan: &PlanName,
    ) -> Result<PostIssuesResponse, AppError> {
        let url = format!("{}/v1/projects/{}/issues", self.base_url, project_name);
        let body = json!({
            "plan": plan,
            "title": "auto-generated issue by Shelltide",
            "type": "DATABASE_CHANGE",
        });
        let response = self.client.post(&url).json(&body).send().await?;
        Self::handle_response(
            response,
            &format!("Create issue for project '{project_name}'"),
        )
        .await
    }

    async fn check_sql(&self, instance: &str, database: &str, sql: &str) -> Result<(), AppError> {
        let url = format!("{}/v1/sql/check", self.base_url);
        let request = SqlCheckRequest {
            name: format!("instances/{instance}/databases/{database}"),
            statement: sql.to_string(),
        };

        let response = self.client.post(&url).json(&request).send().await?;
        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            println!("SQL check failed - Status: {status}, Response: {response_text}",);
            return Err(AppError::ApiError(format!(
                "SQL check failed. Status: {status}, Response: {response_text}",
            )));
        }

        // 성공하면 빈 오브젝트가옴
        match serde_json::from_str::<serde_json::Value>(&response_text) {
            Ok(res_json) => {
                if res_json.get("advises").is_some() {
                    Err(AppError::ApiError(format!("SQL check failed: {res_json}")))
                } else {
                    Ok(())
                }
            }
            Err(e) => {
                println!(
                    "Failed to parse SQL check response - Status: {status}, Response: {response_text}",
                );
                Err(AppError::ApiError(format!(
                    "Failed to parse SQL check response: {e}"
                )))
            }
        }
    }

    async fn get_latests_revisions(
        &self,
        instance: &str,
        database: &str,
    ) -> Result<Revision, AppError> {
        let url = format!(
            "{}/v1/instances/{instance}/databases/{database}/revisions",
            self.base_url,
        );
        let response = self.client.get(&url).send().await?;
        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            println!("Get latest revisions failed - Status: {status}, Response: {response_text}",);
            return Err(AppError::ApiError(format!(
                "Get latest revisions failed. Status: {status}, Response: {response_text}",
            )));
        }

        let response_value: serde_json::Value = match serde_json::from_str(&response_text) {
            Ok(value) => value,
            Err(e) => {
                println!(
                    "Failed to parse latest revisions response - Status: {status}, Response: {response_text}",
                );
                return Err(AppError::ApiError(format!(
                    "Failed to parse latest revisions response: {e}",
                )));
            }
        };
        let revisions = response_value
            .get("revisions")
            .ok_or_else(|| AppError::ApiError("No revisions field found".to_string()))?
            .as_array()
            .ok_or_else(|| AppError::ApiError("No revisions array found".to_string()))?
            .iter()
            .filter_map(|r| serde_json::from_value::<Revision>(r.clone()).ok())
            .collect::<Vec<Revision>>();
        revisions
            .iter()
            .filter(|r| r.create_time.is_some())
            .max_by_key(|r| r.create_time.as_ref().unwrap())
            .cloned()
            .ok_or_else(|| {
                AppError::ApiError("No revisions with valid create_time found".to_string())
            })
    }

    async fn get_changelogs(
        &self,
        instance: &str,
        database: &str,
        project_name: &str,
    ) -> Result<Vec<Changelog>, AppError> {
        let url = format!(
            "{}/v1/instances/{instance}/databases/{database}/changelogs",
            self.base_url,
        );

        let response = self
            .client
            .get(&url)
            .query(&[("pageSize", "1000"), ("view", "CHANGELOG_VIEW_FULL")])
            .send()
            .await?;
        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            println!("Get changelogs failed - Status: {status}, Response: {response_text}",);
            return Err(AppError::ApiError(format!(
                "Get changelogs failed. Status: {status}, Response: {response_text}",
            )));
        }

        let response_value: serde_json::Value = match serde_json::from_str(&response_text) {
            Ok(value) => value,
            Err(e) => {
                println!(
                    "Failed to parse changelogs response - Status: {status}, Response: {response_text}",
                );
                return Err(AppError::ApiError(format!(
                    "Failed to parse changelogs response: {e}"
                )));
            }
        };

        Ok(response_value
            .get("changelogs")
            .ok_or_else(|| AppError::ApiError("No changelogs field found".to_string()))?
            .as_array()
            .ok_or_else(|| AppError::ApiError("No changelogs array found".to_string()))?
            .iter()
            .filter_map(|v| match serde_json::from_value::<Changelog>(v.clone()) {
                Ok(c) => {
                    if c.issue.project == project_name && !c.statement.is_empty() {
                        Some(c)
                    } else {
                        None
                    }
                }
                Err(_) => None,
            })
            .collect())
    }

    async fn create_revision(
        &self,
        instance: &str,
        database: &str,
        name: &str,
        version: &str,
        sheet: &str,
    ) -> Result<Revision, AppError> {
        let url = format!(
            "{}/v1/instances/{instance}/databases/{database}/revisions",
            self.base_url,
        );

        let body = json!({
            "name": name,
            "version": version,
            "sheet": sheet,
        });
        let response = self.client.post(&url).json(&body).send().await?;
        let status = response.status();

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            println!("Revision creation failed - Status: {status}, Response: {error_body}");
            return Err(AppError::ApiError(format!(
                "Failed to create revision. Status: {status}, Response: {error_body}",
            )));
        }

        let response_text = response.text().await?;
        match serde_json::from_str::<Revision>(&response_text) {
            Ok(revision) => Ok(revision),
            Err(e) => {
                println!(
                    "Failed to parse revision response - Status: {status}, Response: {response_text}"
                );
                let error_msg = format!("Failed to parse revision response: {e}");
                Err(AppError::ApiError(error_msg))
            }
        }
    }

    async fn get_databases(&self, instance: &str) -> Result<Vec<String>, AppError> {
        let mut all_databases = Vec::new();
        let mut page_token: Option<String> = None;
        
        loop {
            let url = format!("{}/v1/instances/{}/databases", self.base_url, instance);
            let mut request = self.client.get(&url).query(&[("pageSize", "100")]);
            
            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }
            
            let response = request.send().await?;
            let status = response.status();
            let response_text = response.text().await?;

            if !status.is_success() {
                println!("Get databases failed - Status: {}, Response: {}", status, response_text);
                return Err(AppError::ApiError(format!(
                    "Get databases failed. Status: {}, Response: {}", status, response_text
                )));
            }

            // Parse the response to extract database names and next page token
            match serde_json::from_str::<serde_json::Value>(&response_text) {
                Ok(response_value) => {
                    if let Some(databases_array) = response_value.get("databases").and_then(|v| v.as_array()) {
                        let database_names: Vec<String> = databases_array
                            .iter()
                            .filter_map(|db| {
                                db.get("name")
                                    .and_then(|name| name.as_str())
                                    .map(|name_str| {
                                        // Extract database name from full path like "instances/xxx/databases/bridge"
                                        name_str.split('/').last().unwrap_or(name_str).to_string()
                                    })
                            })
                            .collect();
                        all_databases.extend(database_names);
                    }
                    
                    // Check for next page token
                    page_token = response_value
                        .get("nextPageToken")
                        .and_then(|token| token.as_str())
                        .map(|s| s.to_string());
                    
                    // If no next page token, we're done
                    if page_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    println!("Failed to parse databases response - Status: {}, Response: {}", status, response_text);
                    return Err(AppError::ApiError(format!("Failed to parse databases response: {}", e)));
                }
            }
        }
        
        Ok(all_databases)
    }

    async fn get_latests_revisions_silent(&self, instance: &str, database: &str) -> Result<Revision, AppError> {
        let url = format!(
            "{}/v1/instances/{instance}/databases/{database}/revisions",
            self.base_url,
        );
        let response = self.client.get(&url).send().await?;
        let status = response.status();
        let response_text = response.text().await?;
        
        if !status.is_success() {
            // Don't print error messages for status command
            return Err(AppError::ApiError(format!(
                "Get latest revisions failed. Status: {}", status
            )));
        }
        
        let response_value: serde_json::Value = match serde_json::from_str(&response_text) {
            Ok(value) => value,
            Err(e) => {
                return Err(AppError::ApiError(format!(
                    "Failed to parse latest revisions response: {}", e
                )));
            }
        };
        let revisions = response_value
            .get("revisions")
            .ok_or_else(|| AppError::ApiError("No revisions field found".to_string()))?
            .as_array()
            .ok_or_else(|| AppError::ApiError("No revisions array found".to_string()))?
            .iter()
            .filter_map(|r| serde_json::from_value::<Revision>(r.clone()).ok())
            .collect::<Vec<Revision>>();
        revisions
            .iter()
            .filter(|r| r.create_time.is_some())
            .max_by_key(|r| r.create_time.as_ref().unwrap())
            .cloned()
            .ok_or_else(|| {
                AppError::ApiError("No revisions with valid create_time found".to_string())
            })
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;

    use async_trait::async_trait;

    use crate::{
        api::{
            traits::BytebaseApi,
            types::{
                Changelog, Instance, Issue, IssueName, PlanName, PostIssuesResponse,
                PostPlansResponse, PostSheetsResponse, Project, Revision, SheetName, SheetRequest,
            },
        },
        config::Credentials,
        error::AppError,
    };

    #[derive(Debug, Default)]
    pub struct FakeApiClient {
        pub projects: HashMap<String, Vec<Issue>>,
    }

    impl FakeApiClient {
        pub fn login(&mut self, _credentials: &Credentials) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[async_trait]
    impl BytebaseApi for FakeApiClient {
        async fn get_project(&self, project_name: &str) -> Result<Project, AppError> {
            if project_name == "existing-project" {
                Ok(Project {
                    title: "Existing Project".to_string(),
                })
            } else {
                Err(AppError::ApiError("Project not found".to_string()))
            }
        }
        async fn get_instance(&self, instance_name: &str) -> Result<Instance, AppError> {
            Ok(Instance {
                name: instance_name.to_string(),
            })
        }
        async fn get_done_issues(&self, project_name: &str) -> Result<Vec<Issue>, AppError> {
            self.projects
                .get(project_name)
                .cloned()
                .ok_or_else(|| AppError::ApiError("Project not found".to_string()))
        }
        async fn check_sql(
            &self,
            _instance: &str,
            _database: &str,
            _sql: &str,
        ) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn create_plan(
            &self,
            _project_name: &str,
            _instance: &str,
            _database: &str,
            _sheet_name: SheetName,
        ) -> Result<PostPlansResponse, AppError> {
            unimplemented!()
        }
        async fn create_sheet(
            &self,
            _project_name: &str,
            _sheet: SheetRequest,
        ) -> Result<PostSheetsResponse, AppError> {
            unimplemented!()
        }
        async fn create_rollout(
            &self,
            _project_name: &str,
            _plan_name: PlanName,
            _issue_name: IssueName,
        ) -> Result<(), AppError> {
            unimplemented!()
        }
        async fn create_issue(
            &self,
            _project_name: &str,
            _plan: &PlanName,
        ) -> Result<PostIssuesResponse, AppError> {
            unimplemented!()
        }
        async fn get_latests_revisions(
            &self,
            _instance: &str,
            _database: &str,
        ) -> Result<Revision, AppError> {
            unimplemented!()
        }
        async fn get_changelogs(
            &self,
            _instance: &str,
            _database: &str,
            _project_name: &str,
        ) -> Result<Vec<Changelog>, AppError> {
            unimplemented!()
        }
        async fn create_revision(
            &self,
            _instance: &str,
            _database: &str,
            _name: &str,
            _version: &str,
            _sheet: &str,
        ) -> Result<Revision, AppError> {
            unimplemented!()
        }
        
        async fn get_databases(&self, _instance: &str) -> Result<Vec<String>, AppError> {
            Ok(vec!["bridge".to_string(), "admin".to_string()])
        }
        
        async fn get_latests_revisions_silent(&self, _instance: &str, _database: &str) -> Result<Revision, AppError> {
            unimplemented!()
        }
    }
}
