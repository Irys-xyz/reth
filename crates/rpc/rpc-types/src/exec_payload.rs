use alloy_rpc_types::Withdrawal;
use revm_primitives::B256;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    irys_payload::ExecutionPayloadV1Irys, ExecutionPayloadV1, ExecutionPayloadV2,
    ExecutionPayloadV3, ExecutionPayloadV4,
};

/// An execution payload, which can be either [ExecutionPayloadV1], [ExecutionPayloadV2], or
/// [ExecutionPayloadV3].

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ExecutionPayload {
    // /// V1 payload
    // V1(ExecutionPayloadV1),
    // /// V2 payload
    // V2(ExecutionPayloadV2),
    // /// V3 payload
    // V3(ExecutionPayloadV3),
    // /// V4 payload
    // V4(ExecutionPayloadV4),
    /// V1 Irys payload
    V1Irys(ExecutionPayloadV1Irys),
}

impl ExecutionPayload {
    // /// Returns a reference to the V1 payload.
    pub const fn as_v1(&self) -> &ExecutionPayloadV1 {
        match self {
            // ExecutionPayload::V1(payload) => payload,
            // ExecutionPayload::V2(payload) => &payload.payload_inner,
            // ExecutionPayload::V3(payload)
            ExecutionPayload::V1Irys(payload) => &payload.payload_inner.payload_inner.payload_inner, // ExecutionPayload::V4(payload) => &payload.payload_inner.payload_inner.payload_inner,
                                                                                                     // ExecutionPayload::V1Irys(payload) => &payload
        }
    }

    /// Returns a mutable reference to the V1 payload.
    pub fn as_v1_mut(&mut self) -> &mut ExecutionPayloadV1 {
        match self {
            // ExecutionPayload::V1(payload) => payload,
            // ExecutionPayload::V2(payload) => &mut payload.payload_inner,
            // ExecutionPayload::V3(payload) => &mut payload.payload_inner.payload_inner,
            // ExecutionPayload::V4(payload) => &mut payload.payload_inner.payload_inner.payload_inner,
            ExecutionPayload::V1Irys(payload) => {
                &mut payload.payload_inner.payload_inner.payload_inner
            }
        }
    }

    /// Consumes the payload and returns the V1 payload.
    pub fn into_v1(self) -> ExecutionPayloadV1 {
        match self {
            // ExecutionPayload::V1(payload) => payload,
            // ExecutionPayload::V2(payload) => payload.payload_inner,
            // ExecutionPayload::V3(payload) => payload.payload_inner.payload_inner,
            // ExecutionPayload::V4(payload) => payload.payload_inner.payload_inner.payload_inner,
            ExecutionPayload::V1Irys(payload) => payload.payload_inner.payload_inner.payload_inner,
        }
    }

    /// Returns a reference to the V2 payload, if any.
    pub const fn as_v2(&self) -> Option<&ExecutionPayloadV2> {
        match self {
            // ExecutionPayload::V1(_) => None,
            // ExecutionPayload::V2(payload) => Some(payload),
            // ExecutionPayload::V3(payload) => Some(&payload.payload_inner),
            // ExecutionPayload::V4(payload) => Some(&payload.payload_inner.payload_inner),
            ExecutionPayload::V1Irys(payload) => Some(&payload.payload_inner.payload_inner),
        }
    }

    /// Returns a mutable reference to the V2 payload, if any.
    pub fn as_v2_mut(&mut self) -> Option<&mut ExecutionPayloadV2> {
        match self {
            // ExecutionPayload::V1(_) => None,
            // ExecutionPayload::V2(payload) => Some(payload),
            // ExecutionPayload::V3(payload) => Some(&mut payload.payload_inner),
            // ExecutionPayload::V4(payload) => Some(&mut payload.payload_inner.payload_inner),
            ExecutionPayload::V1Irys(payload) => Some(&mut payload.payload_inner.payload_inner),
        }
    }

    /// Returns a reference to the V2 payload, if any.
    pub const fn as_v3(&self) -> Option<&ExecutionPayloadV3> {
        match self {
            // ExecutionPayload::V1(_) | ExecutionPayload::V2(_) => None,
            // ExecutionPayload::V3(payload) => Some(payload),
            // ExecutionPayload::V4(payload) => Some(&payload.payload_inner),
            ExecutionPayload::V1Irys(payload) => Some(&payload.payload_inner),
        }
    }

    /// Returns a mutable reference to the V2 payload, if any.
    pub fn as_v3_mut(&mut self) -> Option<&mut ExecutionPayloadV3> {
        match self {
            // ExecutionPayload::V1(_) | ExecutionPayload::V2(_) => None,
            // ExecutionPayload::V3(payload) => Some(payload),
            // ExecutionPayload::V4(payload) => Some(&mut payload.payload_inner),
            ExecutionPayload::V1Irys(payload) => Some(&mut payload.payload_inner),
        }
    }

    /// Returns a reference to the V4 payload, if any.
    pub const fn as_v4(&self) -> Option<&ExecutionPayloadV4> {
        match self {
            // ExecutionPayload::V1(_) | ExecutionPayload::V2(_) | ExecutionPayload::V3(_) => None,
            // ExecutionPayload::V4(payload) => Some(payload),
            ExecutionPayload::V1Irys(_) => None,
        }
    }

    /// Returns a mutable reference to the V4 payload, if any.
    pub fn as_v4_mut(&mut self) -> Option<&mut ExecutionPayloadV4> {
        match self {
            // ExecutionPayload::V1(_) | ExecutionPayload::V2(_) | ExecutionPayload::V3(_) => None,
            // ExecutionPayload::V4(payload) => Some(payload),
            ExecutionPayload::V1Irys(_) => None,
        }
    }
    pub const fn as_v1_irys(&self) -> Option<&ExecutionPayloadV1Irys> {
        match self {
            ExecutionPayload::V1Irys(payload) => Some(payload),
        }
    }

    pub fn as_v1_irys_mut(&mut self) -> Option<&mut ExecutionPayloadV1Irys> {
        match self {
            ExecutionPayload::V1Irys(payload) => Some(payload),
        }
    }

    /// Returns the withdrawals for the payload.
    pub const fn withdrawals(&self) -> Option<&Vec<Withdrawal>> {
        // match self.as_v2() {
        match self.as_v1_irys() {
            Some(payload) => Some(&payload.payload_inner.payload_inner.withdrawals),
            None => None,
        }
    }

    /// Returns the timestamp for the payload.
    pub const fn timestamp(&self) -> u64 {
        // self.as_v1().timestamp
        self.as_v1().timestamp
    }

    /// Returns the parent hash for the payload.
    pub const fn parent_hash(&self) -> B256 {
        self.as_v1().parent_hash
    }

    /// Returns the block hash for the payload.
    pub const fn block_hash(&self) -> B256 {
        self.as_v1().block_hash
    }

    /// Returns the block number for this payload.
    pub const fn block_number(&self) -> u64 {
        self.as_v1().block_number
    }

    /// Returns the prev randao for this payload.
    pub const fn prev_randao(&self) -> B256 {
        self.as_v1().prev_randao
    }
}

// impl From<ExecutionPayloadV1> for ExecutionPayload {
//     fn from(payload: ExecutionPayloadV1) -> Self {
//         Self::V1(payload)
//     }
// }

// impl From<ExecutionPayloadV2> for ExecutionPayload {
//     fn from(payload: ExecutionPayloadV2) -> Self {
//         Self::V2(payload)
//     }
// }

// impl From<ExecutionPayloadV3> for ExecutionPayload {
//     fn from(payload: ExecutionPayloadV3) -> Self {
//         Self::V3(payload)
//     }
// }

// impl From<ExecutionPayloadV4> for ExecutionPayload {
//     fn from(payload: ExecutionPayloadV4) -> Self {
//         Self::V4(payload)
//     }
// }

impl From<ExecutionPayloadV1Irys> for ExecutionPayload {
    fn from(payload: ExecutionPayloadV1Irys) -> Self {
        Self::V1Irys(payload)
    }
}

// Deserializes untagged ExecutionPayload by trying each variant in falling order
impl<'de> Deserialize<'de> for ExecutionPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ExecutionPayloadDesc {
            // V4(ExecutionPayloadV4),
            // V3(ExecutionPayloadV3),
            // V2(ExecutionPayloadV2),
            // V1(ExecutionPayloadV1),
            V1Irys(ExecutionPayloadV1Irys),
        }
        match ExecutionPayloadDesc::deserialize(deserializer)? {
            // ExecutionPayloadDesc::V4(payload) => Ok(Self::V4(payload)),
            // ExecutionPayloadDesc::V3(payload) => Ok(Self::V3(payload)),
            // ExecutionPayloadDesc::V2(payload) => Ok(Self::V2(payload)),
            // ExecutionPayloadDesc::V1(payload) => Ok(Self::V1(payload)),
            ExecutionPayloadDesc::V1Irys(payload) => Ok(Self::V1Irys(payload)),
        }
    }
}
