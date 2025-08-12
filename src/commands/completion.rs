use crate::cli::Cli;
use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

pub fn handle_completion_command(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    let cmd_name = cmd.get_name().to_string();
    generate(shell, &mut cmd, cmd_name, &mut io::stdout());
    Ok(())
}
