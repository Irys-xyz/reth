use op_alloy_rpc_types_engine::OpExecutionPayloadEnvelopeV3;
use reth::rpc::types::{engine::{ExecutionPayloadEnvelopeV3, ExecutionPayloadV3}, irys_payload::{ExecutionPayloadEnvelopeV1Irys, ExecutionPayloadV1Irys}};

/// The execution payload envelope type.
pub trait PayloadEnvelopeExt: Send + Sync + std::fmt::Debug {
    /// Returns the execution payload V3 from the payload
    fn execution_payload(&self) -> ExecutionPayloadV1Irys;
}

impl PayloadEnvelopeExt for OpExecutionPayloadEnvelopeV3 {
    fn execution_payload(&self) -> ExecutionPayloadV3 {
        self.execution_payload.clone()
    }
}

impl PayloadEnvelopeExt for ExecutionPayloadEnvelopeV3 {
    fn execution_payload(&self) -> ExecutionPayloadV3 {
        self.execution_payload.clone()
    }
}


impl PayloadEnvelopeExt for ExecutionPayloadEnvelopeV1Irys {
    fn execution_payload(&self) -> ExecutionPayloadV1Irys {
        self.execution_payload.clone()
    }
    fn blobs_bundle(&self) -> BlobsBundleV1 {
        self.blobs_bundle.clone()
    }
}
