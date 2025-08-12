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

        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::ApiError(format!(
                "Failed to get project '{project_name}': {error_body}"
            )));
        }
        Ok(response.json().await?)
    }

    async fn get_instance(&self, instance_name: &str) -> Result<Instance, AppError> {
        let url = format!("{}/v1/instances/{}", self.base_url, instance_name);
        let response = self.client.get(&url).send().await?;
        Ok(serde_json::from_value(response.json().await?).map_err(|_| {
            // When the instance is not found, the response has a different structure.
            AppError::ApiError(format!("Failed to get instance '{instance_name}'"))
        })?)
    }

    async fn get_done_issues(&self, project_name: &str) -> Result<Vec<Issue>, AppError> {
        let url = format!(
            "{}/v1/projects/{}/issues?filter=status=\"DONE\"",
            self.base_url, project_name
        );
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::ApiError(format!(
                "Failed to fetch issues for project '{project_name}': {error_body}"
            )));
        }
        let res_json: IssuesResponse = response.json().await?;
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
        Ok(response.json().await?)
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
        Ok(response.json().await?)
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
        Ok(response.json().await?)
    }

    async fn check_sql(&self, instance: &str, database: &str, sql: &str) -> Result<(), AppError> {
        let url = format!("{}/v1/sql/check", self.base_url);
        let request = SqlCheckRequest {
            name: format!("instances/{instance}/databases/{database}"),
            statement: sql.to_string(),
        };

        let response = self.client.post(&url).json(&request).send().await?;

        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::ApiError(format!(
                "SQL check failed: {error_body}"
            )));
        }

        // 성공하면 빈 오브젝트가옴
        let res_json: serde_json::Value = response.json().await?;
        if res_json.get("advises").is_some() {
            Err(AppError::ApiError(format!("SQL check failed: {res_json}")))
        } else {
            Ok(())
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
        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let revisions = response
            .get("revisions")
            .ok_or_else(|| AppError::ApiError("No revisions field found".to_string()))?
            .as_array()
            .ok_or_else(|| AppError::ApiError("No revisions array found".to_string()))?
            .iter()
            .filter_map(|r| serde_json::from_value::<Revision>(r.clone()).ok())
            .collect::<Vec<Revision>>();
        revisions
            .iter()
            .max_by_key(|r| r.create_time)
            .cloned()
            .ok_or_else(|| AppError::ApiError("No revisions found".to_string()))
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

        Ok(self
            .client
            .get(&url)
            .query(&[("pageSize", "1000")])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?
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
        Ok(response.json().await?)
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
        pub fn login(&mut self, credentials: &Credentials) -> Result<(), AppError> {
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
    }
}
