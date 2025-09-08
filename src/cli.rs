use clap::{Parser, Subcommand};
use clap_complete::Shell;

/// A CLI for managing database migrations with Bytebase.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Log in to a Bytebase instance
    Login(LoginArgs),

    /// Manage CLI configuration
    Config(ConfigArgs),

    /// Manage environments
    Env(EnvArgs),

    /// Apply migrations to a target environment
    Migrate(MigrateArgs),

    /// Show the current migration status of all environments
    Status(StatusArgs),

    /// Generate shell completions
    Completion(CompletionArgs),

    /// Extract changelog scripts from a database
    Extract(ExtractArgs),
}

// --- Argument Structs ---

#[derive(Parser, Debug)]
pub struct LoginArgs {
    /// The URL of the Bytebase instance
    #[arg(long)]
    pub url: String,
    /// The service account email (e.g., "your-sa@service.bytebase.com")
    #[arg(long)]
    pub service_account: String,
    /// The service key associated with the service account
    #[arg(long)]
    pub service_key: String,
}

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Set a configuration key-value pair
    Set {
        /// The configuration key (e.g., "default.source_env")
        key: String,
        /// The value to set
        value: String,
    },
    /// Get the value of a configuration key
    Get {
        /// The configuration key to retrieve
        key: String,
    },
}

#[derive(Parser, Debug)]
pub struct EnvArgs {
    #[command(subcommand)]
    pub command: EnvCommand,
}

#[derive(Subcommand, Debug)]
pub enum EnvCommand {
    /// Add a new environment
    Add {
        /// A short, memorable name for the environment (e.g., "staging")
        name: String,
        /// The full name of the corresponding Bytebase project
        project: String,
        /// The instance name
        instance: String,
    },
    /// List all configured environments
    List,
    /// Remove a configured environment
    Remove {
        /// The name of the environment to remove
        name: String,
    },
}

#[derive(Debug, Clone)]
pub struct EnvDb {
    pub env: String,
    pub db: String,
}

impl std::str::FromStr for EnvDb {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid value '{s}'. Use '<env>/<database>'"));
        }
        Ok(EnvDb {
            env: parts[0].to_string(),
            db: parts[1].to_string(),
        })
    }
}

#[derive(Parser, Debug)]
pub struct MigrateArgs {
    /// Source database name
    pub source_db: String,
    /// Target as "<env>/<database>"
    pub target: EnvDb,

    /// The version to migrate to, number or "LATEST"
    #[arg(long, short)]
    pub to: String,
}

#[derive(Parser, Debug)]
pub struct RevertArgs {
    /// The target environment to revert migrations from
    pub target_env: String,

    /// The version to revert to, specified by an issue number
    #[arg(long, short)]
    pub to: String,
}

#[derive(Parser, Debug)]
pub struct CompletionArgs {
    /// The shell to generate completions for
    #[clap(value_enum)]
    pub shell: Shell,
}

#[derive(Parser, Debug)]
pub struct StatusArgs {
    /// Optional filter for specific environment/database as "<env>/<database>" or just "<env>"
    pub filter: Option<String>,
}

#[derive(Parser, Debug)]
pub struct ExtractArgs {
    /// Target database as "<env>/<database>"
    pub target: EnvDb,

    /// Starting issue number (inclusive)
    #[arg(long)]
    pub from: Option<u32>,

    /// Ending issue number (inclusive)
    #[arg(long)]
    pub to: Option<u32>,
}
