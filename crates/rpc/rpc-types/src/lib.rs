//! Reth RPC type definitions.
//!
//! Provides all relevant types for the various RPC endpoints, grouped by namespace.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

mod eth;
pub mod irys_payload;
mod mev;
mod net;
mod peer;
mod rpc;

// re-export for convenience
pub use alloy_rpc_types::serde_helpers;

// Ethereum specific rpc types coming from alloy.
pub use alloy_rpc_types::*;

pub mod trace {
    //! RPC types for trace endpoints and inspectors.
    pub use alloy_rpc_types_trace::*;
}

// Anvil specific rpc types coming from alloy.
pub use alloy_rpc_types_anvil as anvil;

// re-export beacon
pub use alloy_rpc_types_beacon as beacon;

// Ethereum specific rpc types related to typed transaction requests and the engine API.
pub use eth::{
    engine,
    engine::{
        BlobsBundleV1, ExecutionPayloadBodyV1, ExecutionPayloadFieldV2, ExecutionPayloadInputV2,
        /* ExecutionPayload */ ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3,
        ExecutionPayloadV4, PayloadError,
    },
    error::ToRpcError,
    transaction::{self, TransactionRequest, TypedTransactionRequest},
};
pub mod exec_payload;
pub use exec_payload::*;

pub use mev::*;
pub use net::*;
pub use peer::*;
pub use rpc::*;
