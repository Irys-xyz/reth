use std::sync::{Arc, RwLock};

use irys_storage::reth_provider::IrysRethProvider;
use reth_chainspec::ChainSpec;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]

/// Container struct for all the objects we want to route through to reth
pub struct IrysExt {
    /// deprecated
    pub reload: Arc<RwLock<UnboundedSender<ReloadPayload>>>,
    /// the provider that gives Reth access to Irys node components
    pub provider: IrysRethProvider,
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Enum containing the different node reload payloads
pub enum ReloadPayload {
    /// Reload the node with a specific chainspec - normally used to provide a new genesis config
    ReloadConfig(ChainSpec),
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Enum so the node exit reason can be propagated to a higher level caller
pub enum NodeExitReason {
    /// the node should exit
    Normal,
    /// the node should reload with the specified payload
    Reload(ReloadPayload),
}
