//! `reth recover` command.

use clap::{Parser, Subcommand};
use reth_cli_runner::CliContext;
use reth_node_core::irys_ext::NodeExitReason;

mod storage_tries;

/// `reth recover` command
#[derive(Debug, Parser, Clone)]
pub struct Command {
    #[command(subcommand)]
    command: Subcommands,
}

/// `reth recover` subcommands
#[derive(Subcommand, Clone, Debug)]
pub enum Subcommands {
    /// Recover the node by deleting dangling storage tries.
    StorageTries(storage_tries::Command),
}

impl Command {
    /// Execute `recover` command
    pub async fn execute(self, ctx: CliContext) -> eyre::Result<NodeExitReason> {
        match self.command {
            Subcommands::StorageTries(command) => command.execute(ctx).await,
        }?;
        Ok(NodeExitReason::Normal)
    }
}
