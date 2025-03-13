use irys_storage::reth_provider::IrysRethProvider;

#[derive(Debug, Clone)]

/// Container struct for all the objects we want to route through to reth.
pub struct IrysExt {
    /// the provider that gives Reth access to Irys node components
    pub provider: IrysRethProvider,
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Enum so the node exit reason can be propagated to a higher level caller
pub enum NodeExitReason {
    /// the node should exit
    Normal,
}
