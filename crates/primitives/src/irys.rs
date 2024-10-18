use crate::TransactionSigned;
use alloy_rlp::{Decodable, Encodable, RlpDecodable, RlpEncodable};
use bytes::BufMut;
use reth_rpc_types::engine::BlobsBundleV1;
use serde::{Deserialize, Serialize};

/// RLP encoding for blobs bundle
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobsBundleV1Wrapper {
    pub inner: BlobsBundleV1,
}

impl Default for BlobsBundleV1Wrapper {
    fn default() -> Self {
        Self { inner: BlobsBundleV1 { commitments: vec![], proofs: vec![], blobs: vec![] } }
    }
}

// impl Serialize for BlobsBundleV1Wrapper {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         let mut buf = vec![];
//         Encodable::encode(&self, &mut buf);
//         serializer.serialize_bytes(buf.as_slice())
//     }
// }

// impl Deserialize for BlobsBundleV1Wrapper {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de> {
//             deserializer.deserialize_byte_buf(visitor)
//         Decodable::decode(buf)
//     }
// }

// impl Encodable for BlobsBundleV1Wrapper {
//     fn encode(&self, out: &mut dyn bytes::BufMut) {
//         self.inner.commitments.encode(out);
//         self.inner.proofs.encode(out);
//         self.inner.blobs.encode(out);
//     }
// }

// impl Decodable for BlobsBundleV1Wrapper {
//     fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
//         Ok(Self {
//             inner: BlobsBundleV1 {
//                 commitments: Decodable::decode(buf)?,
//                 proofs: Decodable::decode(buf)?,
//                 blobs: Decodable::decode(buf)?,
//             },
//         })
//     }
// }
