use crate::api::types::{
    Changelog, Instance, Issue, IssueName, PlanName, PostIssuesResponse, PostPlansResponse,
    PostSheetsResponse, Project, Revision, SheetName, SheetRequest,
};
use crate::error::AppError;
use async_trait::async_trait;

#[async_trait]
pub trait BytebaseApi: Send + Sync {
    async fn get_project(&self, project_name: &str) -> Result<Project, AppError>;
    async fn get_instance(&self, instance_name: &str) -> Result<Instance, AppError>;
    async fn get_done_issues(&self, project_name: &str) -> Result<Vec<Issue>, AppError>;
    async fn get_latests_revisions(
        &self,
        instance: &str,
        database: &str,
    ) -> Result<Revision, AppError>;
    async fn get_changelogs(
        &self,
        instance: &str,
        database: &str,
        project_name: &str,
    ) -> Result<Vec<Changelog>, AppError>;
    async fn create_plan(
        &self,
        project_name: &str,
        instance: &str,
        database: &str,
        sheet_name: SheetName,
    ) -> Result<PostPlansResponse, AppError>;
    async fn create_sheet(
        &self,
        project_name: &str,
        sheet: SheetRequest,
    ) -> Result<PostSheetsResponse, AppError>;
    async fn create_rollout(
        &self,
        project_name: &str,
        plan_name: PlanName,
        issue_name: IssueName,
    ) -> Result<(), AppError>;
    async fn create_issue(
        &self,
        project_name: &str,
        plan: &PlanName,
    ) -> Result<PostIssuesResponse, AppError>;
    async fn create_revision(
        &self,
        instance: &str,
        database: &str,
        name: &str,
        version: &str,
        sheet: &str,
    ) -> Result<Revision, AppError>;
    async fn check_sql(&self, instance: &str, database: &str, sql: &str) -> Result<(), AppError>;
    async fn get_databases(&self, instance: &str) -> Result<Vec<String>, AppError>;
    /// Get latest revisions without error logging (for status command)
    async fn get_latests_revisions_silent(
        &self,
        instance: &str,
        database: &str,
    ) -> Result<Revision, AppError>;
}
