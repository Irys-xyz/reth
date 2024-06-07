#![allow(missing_docs)]
use bytes::Buf;
use reth_codecs::Compact;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

/// Shadow represents a validator withdrawal from the consensus layer.
// #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, RlpEncodable, RlpDecodable)]
// #[cfg_attr(
//     any(test, feature = "arbitrary"),
//     derive(proptest_derive::Arbitrary, arbitrary::Arbitrary)
// )]
// #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// #[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
// pub struct Shadow {
//     /// Monotonically increasing identifier issued by consensus layer.
//     #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u64_via_ruint"))]
//     pub index: u64,
//     /// Index of validator associated with withdrawal.
//     #[cfg_attr(
//         feature = "serde",
//         serde(with = "alloy_serde::u64_via_ruint", rename = "validatorIndex")
//     )]
//     pub validator_index: u64,
//     /// Target address for withdrawn ether.
//     pub address: Address,
//     /// Value of the withdrawal in gwei.
//     #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u64_via_ruint"))]
//     pub amount: u64,
// }
use acyc::pledge::TxId;
use alloy_rlp::{
    Decodable, Encodable, Error as RlpError, RlpDecodable, RlpDecodableWrapper, RlpEncodable,
    RlpEncodableWrapper,
};
use reth_codecs::main_codec;
// use alloy_rlp::{RlpDecodable, RlpEncodable};
//shadow needs to have:
// a type
// a set of fields it can change (mapped to the account state fields)
// a tx_id (mapped to OG tx id)
use arbitrary::Arbitrary as ShadowArbitrary;
use proptest_derive::Arbitrary as ShadowPropTestArbitrary;
use revm_primitives::{Address, U256};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    // RlpEncodable,
    // RlpDecodable,
    // serde::Serialize,
    // serde::Deserialize,
    ShadowArbitrary,
    ShadowPropTestArbitrary,
    RlpEncodable,
    RlpDecodable,
)]
#[main_codec(no_arbitrary)]

pub struct ShadowTx {
    pub tx_id: TxId,
    // address the tx is from
    pub address: Address,
    pub tx: ShadowTxType,
}
#[main_codec(no_arbitrary)]
#[derive(
    ShadowArbitrary,
    ShadowPropTestArbitrary,
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    // RlpEncodableWrapper,
    // RlpDecodableWrapper,
)]
pub enum ShadowTxType {
    Null, // because default is a required derive TODO: replace with a null TransferShadow or some other no-op
    Transfer(TransferShadow),
    Data(DataShadow),
    MiningAddressPledge(MiningAddressPledgeShadow),
    PartitionPledge(PartitionPledgeShadow),
    PartitionUnPledge(PartitionUnPledgeShadow),
    UnpledgeAll(UnpledgeAllShadow),
    Slash(SlashShadow),
}

// encode/decode boundary type IDs and related casting impls

#[derive(Debug)]
pub enum ShadowTxTypeId {
    Null = 0,
    Transfer = 1,
    Data = 2,
    MiningAddressPledge = 3,
    PartitionPledge = 4,
    PartitionUnPledge = 5,
    UnpledgeAll = 6,
    Slash = 7,
}

#[derive(thiserror::Error, Debug)]
pub enum ShadowTxTypeIdDecodeError {
    #[error("unknown reserved Shadow Tx type ID: {0}")]
    UnknownShadowTypeId(u8),
}

impl TryFrom<u8> for ShadowTxTypeId {
    type Error = ShadowTxTypeIdDecodeError;
    fn try_from(id: u8) -> Result<Self, Self::Error> {
        match id {
            0 => Ok(ShadowTxTypeId::Null),
            1 => Ok(ShadowTxTypeId::Transfer),
            2 => Ok(ShadowTxTypeId::Data),
            3 => Ok(ShadowTxTypeId::MiningAddressPledge),
            4 => Ok(ShadowTxTypeId::PartitionPledge),
            5 => Ok(ShadowTxTypeId::PartitionUnPledge),
            6 => Ok(ShadowTxTypeId::UnpledgeAll),
            7 => Ok(ShadowTxTypeId::Slash),
            _ => Err(ShadowTxTypeIdDecodeError::UnknownShadowTypeId(id)),
        }
    }
}

// impl ShadowTxType {
//     fn as_str(&self) -> &'static str {
//         match self {
//             ShadowTxType::Null => "null",
//             ShadowTxType::Transfer(_) => "transfer",
//             ShadowTxType::Data(_) => "data",
//             ShadowTxType::MiningAddressPledge(_) => "mining address pledge",
//             ShadowTxType::PartitionPledge(_) => "partition pledge",
//             ShadowTxType::Unpledge(_) => "unpledge",
//             ShadowTxType::UnpledgeAll(_) => "unpledge all",
//             ShadowTxType::Slash(_) => "slash",
//         }
//     }
// }

// impl std::str::FromStr for ShadowTxType {
//     type Err;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         match s {
//             _ => Err(Err),
//         }
//     }
// }

impl Encodable for ShadowTxType {
    fn encode(&self, out: &mut dyn bytes::BufMut) {
        match self {
            ShadowTxType::Null => out.put_u8(0x00), // len([0x01, 0x00, 0xc0]) = 3
            ShadowTxType::Transfer(msg) => msg.encode(out),
            ShadowTxType::Data(msg) => msg.encode(out),
            ShadowTxType::MiningAddressPledge(msg) => msg.encode(out),
            ShadowTxType::PartitionPledge(msg) => msg.encode(out),
            ShadowTxType::PartitionUnPledge(msg) => msg.encode(out),
            ShadowTxType::UnpledgeAll(msg) => msg.encode(out),
            ShadowTxType::Slash(msg) => msg.encode(out),
        }
    }
    fn length(&self) -> usize {
        let payload_len = match self {
            ShadowTxType::Null => 3, // len([0x01, 0x00, 0xc0]) = 3
            ShadowTxType::Transfer(msg) => msg.length(),
            ShadowTxType::Data(msg) => msg.length(),
            ShadowTxType::MiningAddressPledge(msg) => msg.length(),
            ShadowTxType::PartitionPledge(msg) => msg.length(),
            ShadowTxType::PartitionUnPledge(msg) => msg.length(),
            ShadowTxType::UnpledgeAll(msg) => msg.length(),
            ShadowTxType::Slash(msg) => msg.length(),
            // P2PMessage::Disconnect(msg) => msg.length(),
            // // id + snappy encoded payload
            // P2PMessage::Ping => 3, // len([0x01, 0x00, 0xc0]) = 3
            // P2PMessage::Pong => 3, // len([0x01, 0x00, 0xc0]) = 3
        };
        // payload_len + 1 // (1 for length of p2p message id)
        payload_len
    }
}

impl Decodable for ShadowTxType {
    // type Error = ShadowTxTypeIdDecodeError;
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let enc_tx_type_id = u8::decode(&mut &buf[..])?;
        let id = ShadowTxTypeId::try_from(enc_tx_type_id)
            .or(Err(RlpError::Custom("unknown tx type id")))?;
        buf.advance(1);
        match id {
            ShadowTxTypeId::Null => Ok(ShadowTxType::Null),
            ShadowTxTypeId::Data => Ok(ShadowTxType::Data(DataShadow::decode(buf)?)),
            ShadowTxTypeId::Transfer => Ok(ShadowTxType::Transfer(TransferShadow::decode(buf)?)),
            ShadowTxTypeId::MiningAddressPledge => {
                Ok(ShadowTxType::MiningAddressPledge(MiningAddressPledgeShadow::decode(buf)?))
            }
            ShadowTxTypeId::PartitionPledge => {
                Ok(ShadowTxType::PartitionPledge(PartitionPledgeShadow::decode(buf)?))
            }
            ShadowTxTypeId::PartitionUnPledge => {
                Ok(ShadowTxType::PartitionUnPledge(PartitionUnPledgeShadow::decode(buf)?))
            }
            ShadowTxTypeId::UnpledgeAll => {
                Ok(ShadowTxType::UnpledgeAll(UnpledgeAllShadow::decode(buf)?))
            }
            ShadowTxTypeId::Slash => Ok(ShadowTxType::Slash(SlashShadow::decode(buf)?)),
        }
    }
}

impl Default for ShadowTxType {
    fn default() -> Self {
        ShadowTxType::Null
    }
}

pub fn exec_shadow(shadow: ShadowTx) {
    match shadow.tx {
        ShadowTxType::Null => {
            eprintln!("Unexpected null shadow tx")
        }
        ShadowTxType::Transfer(TransferShadow { to, amount }) => {
            println!("got transfer to {} amount {}", to, amount)
        }
        ShadowTxType::Data(_) => todo!(),
        ShadowTxType::MiningAddressPledge(_) => todo!(),
        ShadowTxType::PartitionPledge(_) => todo!(),
        ShadowTxType::PartitionUnPledge(_) => todo!(),
        ShadowTxType::UnpledgeAll(_) => todo!(),
        ShadowTxType::Slash(_) => todo!(),
    }
}

pub fn test_shadow() {
    let shadow = ShadowTx {
        tx_id: TxId::random(),
        address: Address::random(),
        tx: ShadowTxType::Transfer(TransferShadow { to: Address::random(), amount: U256::from(1) }),
    };
    exec_shadow(shadow);
}

#[main_codec(no_arbitrary)]
#[derive(
    ShadowArbitrary,
    ShadowPropTestArbitrary,
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    RlpEncodable,
    RlpDecodable,
)]
pub struct TransferShadow {
    // don't need from as that's a constant field
    pub to: Address,
    pub amount: U256,
}

#[main_codec(no_arbitrary)]
#[derive(
    ShadowArbitrary,
    ShadowPropTestArbitrary,
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    RlpEncodable,
    RlpDecodable,
)]

pub struct DataShadow {
    pub fee: U256,
}
#[main_codec(no_arbitrary)]
#[derive(
    ShadowArbitrary,
    ShadowPropTestArbitrary,
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    RlpEncodable,
    RlpDecodable,
)]

pub struct MiningAddressPledgeShadow {
    pub value: U256,
}
#[main_codec(no_arbitrary)]
#[derive(
    ShadowArbitrary,
    ShadowPropTestArbitrary,
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    RlpEncodable,
    RlpDecodable,
)]

pub struct PartitionPledgeShadow {
    pub quantity: U256,
    pub part_hash: TxId,
    pub height: u64,
}

// todo: below are NOT FINAL
#[main_codec(no_arbitrary)]
#[derive(
    ShadowArbitrary,
    ShadowPropTestArbitrary,
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    RlpEncodable,
    RlpDecodable,
)]
pub struct PartitionUnPledgeShadow {
    pub part_hash: TxId,
}
#[main_codec(no_arbitrary)]
#[derive(
    ShadowArbitrary,
    ShadowPropTestArbitrary,
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    RlpEncodable,
    RlpDecodable,
)]

pub struct UnpledgeAllShadow {}

#[main_codec(no_arbitrary)]
#[derive(
    ShadowArbitrary,
    ShadowPropTestArbitrary,
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    RlpEncodable,
    RlpDecodable,
)]
pub struct SlashShadow {
    pub slashed_addr: Address,
}

#[main_codec]
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, RlpEncodableWrapper, RlpDecodableWrapper)]
pub struct Shadows(Vec<ShadowTx>);

impl Shadows {
    /// Create a new Shadows instance.
    pub fn new(shadows: Vec<ShadowTx>) -> Self {
        Self(shadows)
    }

    /// Calculate the total size, including capacity, of the Shadows.
    #[inline]
    pub fn total_size(&self) -> usize {
        self.capacity() * std::mem::size_of::<ShadowTx>()
    }

    /// Calculate a heuristic for the in-memory size of the [Shadows].
    #[inline]
    pub fn size(&self) -> usize {
        self.len() * std::mem::size_of::<ShadowTx>()
    }

    /// Get an iterator over the Shadows.
    pub fn iter(&self) -> std::slice::Iter<'_, ShadowTx> {
        self.0.iter()
    }

    /// Get a mutable iterator over the Shadows.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, ShadowTx> {
        self.0.iter_mut()
    }

    /// Convert [Self] into raw vec of withdrawals.
    pub fn into_inner(self) -> Vec<ShadowTx> {
        self.0
    }
}

impl IntoIterator for Shadows {
    type Item = ShadowTx;
    type IntoIter = std::vec::IntoIter<ShadowTx>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl AsRef<[ShadowTx]> for Shadows {
    fn as_ref(&self) -> &[ShadowTx] {
        &self.0
    }
}

impl Deref for Shadows {
    type Target = Vec<ShadowTx>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Shadows {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<ShadowTx>> for Shadows {
    fn from(withdrawals: Vec<ShadowTx>) -> Self {
        Self(withdrawals)
    }
}
