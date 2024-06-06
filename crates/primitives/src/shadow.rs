/// Withdrawal represents a validator withdrawal from the consensus layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, RlpEncodable, RlpDecodable)]
#[cfg_attr(
    any(test, feature = "arbitrary"),
    derive(proptest_derive::Arbitrary, arbitrary::Arbitrary)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ssz", derive(ssz_derive::Encode, ssz_derive::Decode))]
pub struct Withdrawal {
    /// Monotonically increasing identifier issued by consensus layer.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u64_via_ruint"))]
    pub index: u64,
    /// Index of validator associated with withdrawal.
    #[cfg_attr(
        feature = "serde",
        serde(with = "alloy_serde::u64_via_ruint", rename = "validatorIndex")
    )]
    pub validator_index: u64,
    /// Target address for withdrawn ether.
    pub address: Address,
    /// Value of the withdrawal in gwei.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u64_via_ruint"))]
    pub amount: u64,
}

//shadow needs to have:
// a type
// a set of fields it can change (mapped to the account state fields)
// a tx_id (mapped to OG tx id)

pub struct ShadowTx<T> {
    pub tx_id: String,
    pub address: String,
    pub tx_type: ShadowTxType,
    pub data: T,
}

pub enum ShadowTxType {
    DATA = 1,
    MINING_ADDRESS_PLEDGE = 2,
    PARTITION_PLEDGE = 3,
    UNPLEDGE = 4,
    UNPLEDGE_ALL = 5,
    SLASH = 6,
}

// pub trait ShadowTx:
