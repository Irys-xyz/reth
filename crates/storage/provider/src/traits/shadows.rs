// // use reth_db::transaction::DbTxMut;
// use reth_interfaces::provider::ProviderResult;
// use reth_primitives::B256;
// use reth_rpc_types::irys::ShadowSubmission;
// use revm::primitives::shadow::Shadows;

// pub trait ShadowsProvider {
//     fn add_pending_shadows(
//         self,
//         block_id: B256,
//         shadows: Shadows,
//     ) -> ProviderResult<ShadowSubmission>;
// }
