use std::io::Read;

use alloy_primitives::{wrap_fixed_bytes, U256};
use arbitrary::Arbitrary as PledgeArbitrary;
use proptest_derive::Arbitrary as PledgePropTestArbitrary;
use reth_codecs::{main_codec, Compact};

#[derive(PartialEq, Debug, Default, Eq, Clone, Copy /* Serialize, Deserialize */)]
#[main_codec(no_arbitrary)]
#[derive(PledgeArbitrary, PledgePropTestArbitrary)]
pub struct Pledge {
    tx_id: TxId,
    quantity: U256,
    dest_hash: TxId,
    height: u64,
    tx_type: u8,
}
// #[main_codec(no_arbitrary)]
wrap_fixed_bytes!(
    extra_derives: [],
    pub struct TxId<32>;
);

// thread_local! {
// pub static PLEDGE_COMPRESSOR: RefCell<Compressor<'static>> =
//     RefCell::new(Compressor::new(4).expect("failed to initialize account compressor"));

//     pub static ACCOUNT_DECOMPRESSOR: RefCell<Decompressor<'static>> =
//     RefCell::new(Decompressor::new().expect("failed to initialize account decompressor"));
// }

// impl Compact for Pledge {
//     fn to_compact<B>(self, buf: &mut B) -> usize
//     where
//         B: bytes::BufMut + AsMut<[u8]>,
//     {
//         todo!()
//     }

//     fn from_compact(buf: &[u8], len: usize) -> (Self, &[u8]) {
//         todo!()
//     }
// }
const TXID_LENGTH: usize = 32;
// dumb, but the automatic derive doesn't work
impl Compact for TxId {
    fn to_compact<B>(self, buf: &mut B) -> usize
    where
        B: bytes::BufMut + AsMut<[u8]>,
    {
        buf.put(self.as_slice());
        TXID_LENGTH
    }

    fn from_compact(mut buf: &[u8], len: usize) -> (Self, &[u8]) {
        let mut tx_buf = [0; TXID_LENGTH];
        buf.read_exact(&mut tx_buf).expect("unable to read buf");
        let tx_id = TxId::from(tx_buf);
        (tx_id, buf)
    }
}

// impl Compact for Account {
//     fn to_compact<B>(self, buf: &mut B) -> usize
//     where
//         B: bytes::BufMut + AsMut<[u8]>,
//     {
//         let mut flags = AccountFlags::default();
//         let mut total_length = 0;
//         let mut buffer = bytes::BytesMut::new();
//         let nonce_len = self.nonce.to_compact(&mut buffer);
//         flags.set_nonce_len(nonce_len as u8);
//         let balance_len = self.balance.to_compact(&mut buffer);
//         flags.set_balance_len(balance_len as u8);
//         let bytecode_hash_len = self
//             .bytecode_hash
//             .specialized_to_compact(&mut buffer);
//         flags.set_bytecode_hash_len(bytecode_hash_len as u8);
//         let flags = flags.into_bytes();
//         total_length += flags.len() + buffer.len();
//         buf.put_slice(&flags);
//         buf.put(buffer);
//         total_length
//     }
//     fn from_compact(mut buf: &[u8], len: usize) -> (Self, &[u8]) {
//         let (flags, mut buf) = AccountFlags::from(buf);
//         let (nonce, new_buf) = u64::from_compact(buf, flags.nonce_len() as usize);
//         buf = new_buf;
//         let (balance, new_buf) = U256::from_compact(
//             buf,
//             flags.balance_len() as usize,
//         );
//         buf = new_buf;
//         let (bytecode_hash, new_buf) = Option::specialized_from_compact(
//             buf,
//             flags.bytecode_hash_len() as usize,
//         );
//         buf = new_buf;
//         let obj = Account {
//             nonce: nonce,
//             balance: balance,
//             bytecode_hash: bytecode_hash,
//         };
//         (obj, buf)
//     }
// }
