use crate::error::AppError;
use base64::{Engine, engine::general_purpose};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use uuid::Uuid;

#[derive(Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub web: bool,
}

#[derive(Deserialize, Debug)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Serialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub description: String,
    pub sql: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_sql: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Project {
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlanStepSpec {
    pub id: Uuid,
    pub change_database_config: ChangeDatabaseConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlanStep {
    pub specs: Vec<PlanStepSpec>,
}

#[derive(Serialize)]
pub struct SqlCheckRequest {
    pub name: String,
    pub statement: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
pub enum SqlCheckStatus {
    Success,
    StatusUnspecified,
    Warning,
    Error,
}

#[derive(Deserialize, Debug)]
pub struct IssuesResponse {
    pub issues: Vec<Issue>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Issue {
    pub name: IssueName,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IssueStatus {
    Done,
    Open,
}

impl std::fmt::Display for IssueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                IssueStatus::Done => "DONE",
                IssueStatus::Open => "OPEN",
            }
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IssuesFilter {
    pub create_time: Option<chrono::DateTime<chrono::Utc>>,
    pub status: Option<IssueStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RevisionVersion {
    pub project_name: String,
    pub number: u32,
}

impl<'de> Deserialize<'de> for RevisionVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::new(raw).map_err(|e| de::Error::custom(e.to_string()))
    }
}

impl RevisionVersion {
    pub fn new(version: String) -> Result<Self, AppError> {
        let split = version.split('#').collect::<Vec<&str>>();
        if split.len() != 2 {
            return Err(AppError::InvalidRevisionVersion(format!(
                "Invalid revision version: {version}",
            )));
        }

        let issue_no = split[1].parse::<u32>().map_err(|e| {
            AppError::InvalidRevisionVersion(format!("Invalid issue number: {version}: {e}"))
        })?;
        let project_name = split[0].to_string();
        Ok(Self {
            project_name,
            number: issue_no,
        })
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Revision {
    #[serde(rename = "createTime")]
    pub create_time: Option<chrono::DateTime<chrono::Utc>>,
    pub version: Option<RevisionVersion>,
}

#[derive(Debug, Clone)]
pub struct IssueName {
    pub project: String,
    pub number: u32,
}

impl<'de> Deserialize<'de> for IssueName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let mut split = raw.split('/');
        let project = split
            .nth(1)
            .ok_or(de::Error::custom("cannot find project name"))?
            .to_string();
        let number = split
            .nth(1)
            .ok_or(de::Error::custom("cannot find issue number"))?
            .parse()
            .map_err(|_| de::Error::custom("invalid issue number"))?;
        Ok(Self { project, number })
    }
}

impl std::fmt::Display for IssueName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "projects/{}/issues/{}", self.project, self.number)
    }
}

impl Serialize for IssueName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ChangeLogName {
    pub instance: String,
    pub database: String,
    pub number: u32,
}

impl<'de> Deserialize<'de> for ChangeLogName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;

        let mut split = raw.split('/');
        let instance = split
            .nth(1)
            .ok_or(de::Error::custom("cannot find instance name"))?
            .to_string();
        let database = split
            .nth(1)
            .ok_or(de::Error::custom("cannot find database name"))?
            .to_string();
        let number = split
            .nth(1)
            .ok_or(de::Error::custom("cannot find changelog number"))?
            .parse()
            .map_err(|_| de::Error::custom("invalid changelog number"))?;

        Ok(Self {
            instance,
            database,
            number,
        })
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct StringStatement(String);

impl StringStatement {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for StringStatement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Database {
    pub name: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ChangedResource {
    #[serde(default)]
    pub databases: Vec<Database>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Changelog {
    pub name: ChangeLogName,
    #[serde(rename = "createTime")]
    pub create_time: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub statement: StringStatement,
    pub issue: IssueName,
    #[serde(rename = "changedResources", default)]
    pub changed_resources: ChangedResource,
    #[serde(rename = "type", default)]
    pub changelog_type: Option<ChangelogType>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ChangelogType {
    Migrate,
    Baseline,
    Data,
}

/// All supported SQL dialects. ref: https://docs.bytebase.com/api-reference/sheetservice/post-v1projects-sheets#body-engine
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "UPPERCASE")]
#[allow(dead_code)]
pub enum SQLDialect {
    EngineUnspecified,
    MySQL,
    PostgreSQL,
    ClickHouse,
    Postgres,
    Snowflake,
    SQLite,
    TiDB,
    MongoDB,
    Redis,
    Oracle,
    Spanner,
    MsSQL,
    Redshift,
    MariaDB,
    OceanBase,
    StarRocks,
    Doris,
    Hive,
    Elasticsearch,
    BigQuery,
    DynamoDB,
    Databricks,
    CockroachDB,
    CosmosDB,
    Trino,
    Cassandra,
}

#[derive(Serialize, Debug, Clone)]
pub struct EncodedStatement(String);

impl From<StringStatement> for EncodedStatement {
    fn from(statement: StringStatement) -> Self {
        let base64 = general_purpose::STANDARD.encode(statement.0);
        Self(base64)
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct SheetRequest {
    #[serde(rename = "content")]
    pub sql_statement: EncodedStatement,
    pub engine: SQLDialect,
}

#[derive(Debug, Clone)]
pub struct SheetName {
    pub project_name: String,
    pub number: u32,
}

impl<'de> Deserialize<'de> for SheetName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let mut split = raw.split('/');
        let project_name = split
            .nth(1)
            .ok_or(de::Error::custom("cannot find project name"))?
            .to_string();
        let number = split
            .nth(1)
            .ok_or(de::Error::custom("cannot find sheet number"))?
            .parse()
            .map_err(|_| de::Error::custom("invalid sheet number"))?;

        Ok(Self {
            project_name,
            number,
        })
    }
}

impl Serialize for SheetName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl std::fmt::Display for SheetName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "projects/{}/sheets/{}", self.project_name, self.number)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct PostSheetsResponse {
    pub name: SheetName,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum ChangeDatabaseConfigType {
    Migrate,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChangeDatabaseConfig {
    pub target: String,
    pub sheet: SheetName,
    #[serde(rename = "type")]
    pub config_type: ChangeDatabaseConfigType,
}

#[derive(Serialize, Debug, Clone)]
pub struct PostPlansRequest {
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone)]
pub struct PlanName {
    pub project_name: String,
    pub number: u32,
}

impl<'de> Deserialize<'de> for PlanName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let mut split = raw.split('/');
        let project_name = split
            .nth(1)
            .ok_or(de::Error::custom("cannot find project name"))?
            .to_string();
        let number = split
            .nth(1)
            .ok_or(de::Error::custom("cannot find plan number"))?
            .parse()
            .map_err(|_| de::Error::custom("invalid plan number"))?;
        Ok(Self {
            project_name,
            number,
        })
    }
}

impl std::fmt::Display for PlanName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "projects/{}/plans/{}", self.project_name, self.number)
    }
}

impl Serialize for PlanName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct PostPlansResponse {
    pub name: PlanName,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PostIssuesResponse {
    pub name: IssueName,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Instance {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct RevisionName {
    pub instance: String,
    pub database: String,
    pub number: u32,
}

impl<'de> Deserialize<'de> for RevisionName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let mut split = raw.split('/');
        let instance = split
            .nth(1)
            .ok_or(de::Error::custom("cannot find instance name"))?
            .to_string();
        let database = split
            .nth(1)
            .ok_or(de::Error::custom("cannot find database name"))?
            .to_string();
        let number = split
            .nth(1)
            .ok_or(de::Error::custom("cannot find revision number"))?
            .parse()
            .map_err(|_| de::Error::custom("invalid revision number"))?;
        Ok(Self {
            instance,
            database,
            number,
        })
    }
}

impl std::fmt::Display for RevisionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "instances/{}/databases/{}/revisions/{}",
            self.instance, self.database, self.number
        )
    }
}

impl Serialize for RevisionName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[test]
fn test_issue_name_deserialization() {
    let happy_inputs = vec![
        // input, expected project, expected number
        ("projects/dev-project/issues/101", "dev-project", 101),
        ("projects/dev-project1/issues/102", "dev-project1", 102),
        ("projects/dev-project2/issues/103", "dev-project2", 103),
    ];
    let unhappy_inputs = vec![
        (
            "instances/my-instance/databases/my-db/changelogs/101",
            "invalid issue number",
        ),
        ("projects/dev-project1/issues", "cannot find issue number"),
        ("projects/issues/103", "cannot find issue number"),
    ];

    for input in happy_inputs {
        let issue_name: IssueName =
            serde_json::from_str(format!("\"{}\"", input.0).as_str()).unwrap();
        let (expected_project, expected_number) = (input.1, input.2);
        assert_eq!(issue_name.project, expected_project);
        assert_eq!(issue_name.number, expected_number);
    }

    for input in unhappy_inputs {
        let result = serde_json::from_str::<IssueName>(format!("\"{}\"", input.0).as_str());
        let expected_error = input.1;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(expected_error));
    }
}

#[test]
fn test_changelog_name_deserialization() {
    let happy_inputs = vec![
        // input, expected instance, expected database, expected number
        (
            "instances/my-instance/databases/my-db/changelogs/101",
            "my-instance",
            "my-db",
            101,
        ),
        (
            "instances/my-instance1/databases/my-db1/changelogs/102",
            "my-instance1",
            "my-db1",
            102,
        ),
        (
            "instances/my-instance2/databases/my-db2/changelogs/103",
            "my-instance2",
            "my-db2",
            103,
        ),
    ];
    let unhappy_inputs = vec![
        (
            "instances/my-instance/databases/my-db/changelogs",
            "cannot find changelog number",
        ),
        (
            "projects/dev-project1/issues/101",
            "cannot find changelog number",
        ),
        (
            "instances/my-instance/changelogs/102",
            "cannot find changelog number",
        ),
    ];

    for input in happy_inputs {
        let changelog_name: ChangeLogName =
            serde_json::from_str(format!("\"{}\"", input.0).as_str()).unwrap();
        let (expected_instance, expected_database, expected_number) = (input.1, input.2, input.3);
        assert_eq!(changelog_name.instance, expected_instance);
        assert_eq!(changelog_name.database, expected_database);
        assert_eq!(changelog_name.number, expected_number);
    }

    for input in unhappy_inputs {
        let result = serde_json::from_str::<ChangeLogName>(format!("\"{}\"", input.0).as_str());
        let expected_error = input.1;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(expected_error));
    }
}

#[test]
fn test_changelog_deserialization() {
    let changelog_json = r#"
    [
        {
        "name": "instances/daily-admin/databases/bridge/changelogs/672",
        "createTime": "2025-08-08T12:28:10.353882Z",
        "status": "DONE",
        "statement": "SELECT 1",
        "statementSize": "8",
        "statementSheet": "projects/eclipse-daily-project/sheets/923",
        "issue": "projects/eclipse-daily-project/issues/723",
        "taskRun": "projects/eclipse-daily-project/rollouts/722/stages/723/tasks/758/taskRuns/737",
        "changedResources": {},
        "type": "DATA"
        },
        {
        "name": "instances/daily-admin/databases/bridge/changelogs/666",
        "createTime": "2025-08-08T03:21:45.580535Z",
        "status": "DONE",
        "schema": "SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0;\nSET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0;\n--\n-- Table structure for `stove_itembox_transaction`\n--\nCREATE TABLE `stove_itembox_transaction` (\n  `id` bigint NOT NULL AUTO_INCREMENT COMMENT 'DB PK',\n  `transaction_id` varchar(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci NOT NULL COMMENT '아이템박스 트랜잭션 ID (스토브 제공)',\n  `created_at` datetime NOT NULL COMMENT '레코드 생성 시각',\n  PRIMARY KEY (`id`),\n  UNIQUE KEY `transaction_id_",
        "schemaSize": "554",
        "prevSchema": "SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0;\nSET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0;\n--\n-- Table structure for `stove_itembox_transaction`\n--\nCREATE TABLE `stove_itembox_transaction` (\n  `id` bigint NOT NULL AUTO_INCREMENT COMMENT 'DB PK',\n  `transaction_id` varchar(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci NOT NULL COMMENT '아이템박스 트랜잭션 ID (스토브 제공)',\n  `created_at` datetime NOT NULL COMMENT '레코드 생성 시각',\n  PRIMARY KEY (`id`),\n  UNIQUE KEY `transaction_id_",
        "prevSchemaSize": "554",
        "issue": "projects/eclipse-daily-project/issues/716",
        "taskRun": "projects/eclipse-daily-project/rollouts/716/stages/717/tasks/752/taskRuns/731",
        "type": "BASELINE"
        },
        {
        "name": "instances/daily-admin/databases/bridge/changelogs/409",
        "createTime": "2025-05-26T14:16:55.368892Z",
        "status": "DONE",
        "statement": "-- Change on 2025-05-22 13:57:54.140301+09:00 Issue Number : 444\nALTER TABLE `stove_voided_transaction` MODIFY COLUMN `market_product_id` varchar(255) NOT NULL COMMENT '마켓 상품 코드';\n\nALTER TABLE `stove_voided_transaction` MODIFY COLUMN `product_id` varchar(255) NOT NULL COMMENT 'STOVE 플랫폼 상품 코드';\n\n\n\n",
        "statementSize": "325",
        "statementSheet": "projects/eclipse-daily-project/sheets/576",
        "schema": "SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0;\nSET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0;\n--\n-- Table structure for `stove_itembox_transaction`\n--\nCREATE TABLE `stove_itembox_transaction` (\n  `id` bigint NOT NULL AUTO_INCREMENT COMMENT 'DB PK',\n  `transaction_id` varchar(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci NOT NULL COMMENT '아이템박스 트랜잭션 ID (스토브 제공)',\n  `created_at` datetime NOT NULL COMMENT '레코드 생성 시각',\n  PRIMARY KEY (`id`),\n  UNIQUE KEY `transaction_id_",
        "schemaSize": "554",
        "prevSchema": "SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0;\nSET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0;\n--\n-- Table structure for `stove_itembox_transaction`\n--\nCREATE TABLE `stove_itembox_transaction` (\n  `id` bigint NOT NULL AUTO_INCREMENT COMMENT 'DB PK',\n  `transaction_id` varchar(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci NOT NULL COMMENT '아이템박스 트랜잭션 ID (스토브 제공)',\n  `created_at` datetime NOT NULL COMMENT '레코드 생성 시각',\n  PRIMARY KEY (`id`),\n  UNIQUE KEY `transaction_id_",
        "prevSchemaSize": "554",
        "issue": "projects/eclipse-daily-project/issues/454",
        "taskRun": "projects/eclipse-daily-project/rollouts/454/stages/455/tasks/455/taskRuns/452",
        "changedResources": {
            "databases": [
            {
                "name": "bridge",
                "schemas": [
                {
                    "tables": [
                    {
                        "name": "stove_voided_transaction",
                        "ranges": [
                        {
                            "start": 65,
                            "end": 191
                        },
                        {
                            "start": 193,
                            "end": 321
                        }
                        ]
                    }
                    ]
                }
                ]
            }
            ]
        },
        "type": "MIGRATE"
        },
        {
        "name": "instances/daily-admin/databases/bridge/changelogs/388",
        "createTime": "2025-05-21T06:25:34.172703Z",
        "status": "DONE",
        "statement": "-- Change on 2025-04-29 18:47:44.208586+09:00 Issue Number : 354\nCREATE TABLE IF NOT EXISTS `stove_purchase_transaction` (\n  `id` bigint NOT NULL AUTO_INCREMENT COMMENT 'ID',\n  `account_id` bigint NOT NULL COMMENT '계졍 ID',\n  `character_uuid` varchar(36) NOT NULL COMMENT '캐릭터 UUID',\n  `tid` varchar(255) NOT NULL COMMENT 'TID',\n  `product_id` varchar(255) NOT NULL COMMENT '상품 ID',\n  `quantity` bigint NOT NULL COMMENT '구매 수량',\n  `voided_tid` varchar(255) COMMENT '비정상 환불건의 TID',\n  PRIMARY KEY (`id`),\n  UNIQUE K",
        "statementSize": "1626",
        "statementSheet": "projects/eclipse-daily-project/sheets/555",
        "schema": "SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0;\nSET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0;\n--\n-- Table structure for `stove_itembox_transaction`\n--\nCREATE TABLE `stove_itembox_transaction` (\n  `id` bigint NOT NULL AUTO_INCREMENT COMMENT 'DB PK',\n  `transaction_id` varchar(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci NOT NULL COMMENT '아이템박스 트랜잭션 ID (스토브 제공)',\n  `created_at` datetime NOT NULL COMMENT '레코드 생성 시각',\n  PRIMARY KEY (`id`),\n  UNIQUE KEY `transaction_id_",
        "schemaSize": "554",
        "prevSchema": "SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0;\nSET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0;\n--\n-- Table structure for `stove_itembox_transaction`\n--\nCREATE TABLE `stove_itembox_transaction` (\n  `id` bigint NOT NULL AUTO_INCREMENT COMMENT 'DB PK',\n  `transaction_id` varchar(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci NOT NULL COMMENT '아이템박스 트랜잭션 ID (스토브 제공)',\n  `created_at` datetime NOT NULL COMMENT '레코드 생성 시각',\n  PRIMARY KEY (`id`),\n  UNIQUE KEY `transaction_id_",
        "prevSchemaSize": "554",
        "issue": "projects/eclipse-daily-project/issues/433",
        "taskRun": "projects/eclipse-daily-project/rollouts/433/stages/434/tasks/434/taskRuns/432",
        "changedResources": {
            "databases": [
            {
                "name": "bridge",
                "schemas": [
                {
                    "tables": [
                    {
                        "name": "stove_purchase_transaction",
                        "ranges": [
                        {
                            "start": 65,
                            "end": 639
                        },
                        {
                            "start": 708,
                            "end": 849
                        },
                        {
                            "start": 851,
                            "end": 979
                        },
                        {
                            "start": 981,
                            "end": 1119
                        },
                        {
                            "start": 1121,
                            "end": 1180
                        },
                        {
                            "start": 1249,
                            "end": 1313
                        },
                        {
                            "start": 1315,
                            "end": 1419
                        },
                        {
                            "start": 1488,
                            "end": 1622
                        }
                        ]
                    }
                    ]
                }
                ]
            }
            ]
        },
        "type": "MIGRATE"
        },
        {
        "name": "instances/daily-admin/databases/bridge/changelogs/353",
        "createTime": "2025-05-15T06:05:54.173725Z",
        "status": "DONE",
        "statement": "SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0;\r\nSET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0;\r\n--\r\n-- Table structure for `stove_itembox_transaction`\r\n--\r\nCREATE TABLE `stove_itembox_transaction` (\r\n  `id` bigint NOT NULL AUTO_INCREMENT COMMENT 'DB PK',\r\n  `transaction_id` varchar(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci NOT NULL COMMENT '아이템박스 트랜잭션 ID (스토브 제공)',\r\n  `created_at` datetime NOT NULL COMMENT '레코드 생성 시각',\r\n  PRIMARY KEY (`id`),\r\n  UNIQUE KEY `trans",
        "statementSize": "2469",
        "statementSheet": "projects/eclipse-daily-project/sheets/511",
        "schema": "SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0;\nSET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0;\n--\n-- Table structure for `stove_itembox_transaction`\n--\nCREATE TABLE `stove_itembox_transaction` (\n  `id` bigint NOT NULL AUTO_INCREMENT COMMENT 'DB PK',\n  `transaction_id` varchar(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci NOT NULL COMMENT '아이템박스 트랜잭션 ID (스토브 제공)',\n  `created_at` datetime NOT NULL COMMENT '레코드 생성 시각',\n  PRIMARY KEY (`id`),\n  UNIQUE KEY `transaction_id_",
        "schemaSize": "554",
        "prevSchema": "SET @OLD_UNIQUE_CHECKS=@@UNIQUE_CHECKS, UNIQUE_CHECKS=0;\nSET @OLD_FOREIGN_KEY_CHECKS=@@FOREIGN_KEY_CHECKS, FOREIGN_KEY_CHECKS=0;\nSET FOREIGN_KEY_CHECKS=@OLD_FOREIGN_KEY_CHECKS;\nSET UNIQUE_CHECKS=@OLD_UNIQUE_CHECKS;\n",
        "prevSchemaSize": "215",
        "issue": "projects/eclipse-daily-project/issues/395",
        "taskRun": "projects/eclipse-daily-project/rollouts/395/stages/396/tasks/396/taskRuns/395",
        "changedResources": {
            "databases": [
            {
                "name": "bridge",
                "schemas": [
                {
                    "tables": [
                    {
                        "name": "stove_itembox_transaction",
                        "ranges": [
                        {
                            "start": 191,
                            "end": 728
                        }
                        ]
                    },
                    {
                        "name": "stove_voided_transaction",
                        "ranges": [
                        {
                            "start": 791,
                            "end": 2377
                        }
                        ]
                    }
                    ]
                }
                ]
            }
            ]
        },
        "type": "MIGRATE"
        }
    ]
    "#;

    let changelogs: Vec<Changelog> = serde_json::from_str(changelog_json).unwrap();
    assert_eq!(changelogs.len(), 5);
    assert_eq!(changelogs[0].name.instance, "daily-admin");
    assert_eq!(changelogs[0].name.database, "bridge");
    assert_eq!(changelogs[0].name.number, 672);
    assert_eq!(changelogs[0].statement.0, "SELECT 1".to_string());
    assert_eq!(changelogs[0].issue.project, "eclipse-daily-project");
    assert_eq!(changelogs[0].issue.number, 723);
    assert_eq!(
        changelogs[0]
            .create_time
            .to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
        "2025-08-08T12:28:10.353882Z".to_string()
    );
}

#[test]
fn test_revision_version_deserialization() {
    let happy_inputs = vec![
        ("dev-project#101", "dev-project", 101),
        ("dev-project1#102", "dev-project1", 102),
        ("dev-project2#103", "dev-project2", 103),
    ];

    for input in happy_inputs {
        let revision_version: RevisionVersion =
            serde_json::from_str(format!("\"{}\"", input.0).as_str()).unwrap();
        let (expected_project_name, expected_number) = (input.1, input.2);
        assert_eq!(revision_version.project_name, expected_project_name);
        assert_eq!(revision_version.number, expected_number);
    }
}

#[test]
fn test_sheet_name_serde() {
    let happy_inputs = vec![
        ("projects/dev-project/sheets/101", "dev-project", 101),
        ("projects/dev-project1/sheets/102", "dev-project1", 102),
        ("projects/dev-project2/sheets/103", "dev-project2", 103),
    ];

    for input in happy_inputs {
        let sheet_name: SheetName =
            serde_json::from_str(format!("\"{}\"", input.0).as_str()).unwrap();
        let (expected_project_name, expected_number) = (input.1, input.2);
        assert_eq!(sheet_name.project_name, expected_project_name);
        assert_eq!(sheet_name.number, expected_number);

        let serialized = serde_json::to_string(&sheet_name).unwrap();
        assert_eq!(serialized, format!("\"{}\"", input.0));
    }
}

#[test]
fn test_encoded_statement_from_string_statement() {
    let statement = StringStatement("SELECT 1".to_string());
    let encoded_statement: EncodedStatement = statement.into();
    assert_eq!(encoded_statement.0, "U0VMRUNUIDE=".to_string());
}

#[test]
fn test_plan_name_deserialization() {
    let happy_inputs = vec![
        ("projects/dev-project/plans/101", "dev-project", 101),
        ("projects/dev-project1/plans/102", "dev-project1", 102),
        ("projects/dev-project2/plans/103", "dev-project2", 103),
    ];

    for input in happy_inputs {
        let plan_name: PlanName =
            serde_json::from_str(format!("\"{}\"", input.0).as_str()).unwrap();
        let (expected_project_name, expected_number) = (input.1, input.2);
        assert_eq!(plan_name.project_name, expected_project_name);
        assert_eq!(plan_name.number, expected_number);
    }
}
