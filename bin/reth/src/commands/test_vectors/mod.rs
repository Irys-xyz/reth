//! Command for generating test vectors.

use clap::{Parser, Subcommand};
use reth_node_core::irys_ext::NodeExitReason;

mod tables;

/// Generate test-vectors for different data types.
#[derive(Debug, Parser, Clone)]
pub struct Command {
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand, Clone, Debug)]
/// `reth test-vectors` subcommands
pub enum Subcommands {
    /// Generates test vectors for specified tables. If no table is specified, generate for all.
    Tables {
        /// List of table names. Case-sensitive.
        names: Vec<String>,
    },
}

impl Command {
    /// Execute the command
    pub async fn execute(self) -> eyre::Result<NodeExitReason> {
        match self.command {
            Subcommands::Tables { names } => {
                tables::generate_vectors(names)?;
            }
        }
        Ok(NodeExitReason::Normal)
    }
}
