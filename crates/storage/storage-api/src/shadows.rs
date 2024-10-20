use alloy_eips::BlockHashOrNumber;
use reth_primitives::irys_primitives::Shadows;
use reth_storage_errors::provider::ProviderResult;

/// Client trait for fetching EIP-7685 [Requests] for blocks.
#[auto_impl::auto_impl(&, Arc)]
pub trait ShadowsProvider: Send + Sync {
    /// Get withdrawals by block id.
    fn shadows_by_block(
        &self,
        id: BlockHashOrNumber
    ) -> ProviderResult<Option<Shadows>>;
}
