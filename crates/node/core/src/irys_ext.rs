use std::sync::{Arc, RwLock};

use reth_primitives::ChainSpec;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Default, Clone)]

/// Wrapper type for the irys extension
pub struct IrysExtWrapped(pub Arc<RwLock<IrysExt>>);

#[derive(Debug, Default)]
/// Custom Irys extension that allows for node-wide access to the reload channel
pub struct IrysExt {
    /// reload sender channel - TODO: replace with a oneshot channel
    pub reload: Option<UnboundedSender<ReloadPayload>>,
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
