use reth::rpc::types::{
    engine::{ExecutionPayloadEnvelopeV3, OptimismExecutionPayloadEnvelopeV3},
    irys_payload::{ExecutionPayloadEnvelopeV1Irys, ExecutionPayloadV1Irys},
    ExecutionPayloadV3,
};

/// The execution payload envelope type.
pub trait PayloadEnvelopeExt: Send + Sync + std::fmt::Debug {
    /// Returns the execution payload V3 from the payload
    fn execution_payload(&self) -> ExecutionPayloadV1Irys;
}

// impl PayloadEnvelopeExt for OptimismExecutionPayloadEnvelopeV3 {
//     fn execution_payload(&self) -> ExecutionPayloadV1Irys {
//         self.execution_payload.clone()
//     }
// }

impl PayloadEnvelopeExt for ExecutionPayloadEnvelopeV1Irys {
    fn execution_payload(&self) -> ExecutionPayloadV1Irys {
        self.execution_payload.clone()
    }
}
