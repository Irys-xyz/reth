use std::sync::{Arc, Mutex};

use reth_primitives::ChainSpec;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Default, Clone)]

pub struct IrysExtWrapped(pub Arc<Mutex<IrysExt>>);

#[derive(Debug, Default)]
pub struct IrysExt {
    pub reload: Option<UnboundedSender<ReloadPayload>>,
}

#[derive(Debug, Clone, Eq, PartialEq)]

pub enum ReloadPayload {
    ReloadConfig(ChainSpec),
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Enum so the node exit reason can be propagated to a higher level caller
pub enum NodeExitReason {
    Normal,
    Reload(ReloadPayload),
}
