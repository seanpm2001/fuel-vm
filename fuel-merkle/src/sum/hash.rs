use digest::Digest;
use lazy_static::lazy_static;
use sha2::Sha256;
use std::convert::TryInto;

use crate::common::{Bytes32, LEAF, NODE};

lazy_static! {
    static ref EMPTY_SUM: Bytes32 = Sha256::new().finalize().try_into().unwrap();
}

// Merkle Tree hash of an empty list
// MTH({}) = Hash()
pub fn empty_sum() -> &'static Bytes32 {
    &*EMPTY_SUM
}

// Merkle tree hash of an n-element list D[n]
// MTH(D[n]) = Hash(0x01 || LHS fee || MTH(D[0:k]) || RHS fee || MTH(D[k:n])
pub fn node_sum(lhs_fee: u64, lhs_data: &[u8], rhs_fee: u64, rhs_data: &[u8]) -> Bytes32 {
    let mut hash = Sha256::new();
    hash.update(&[NODE]);
    hash.update(lhs_fee.to_be_bytes());
    hash.update(&lhs_data);
    hash.update(rhs_fee.to_be_bytes());
    hash.update(&rhs_data);
    hash.finalize().try_into().unwrap()
}

// Merkle tree hash of a list with one entry
// MTH({d(0)}) = Hash(0x00 || fee || d(0))
pub fn leaf_sum(fee: u64, data: &[u8]) -> Bytes32 {
    let mut hash = Sha256::new();
    hash.update(&[LEAF]);
    hash.update(fee.to_be_bytes());
    hash.update(&data);
    hash.finalize().try_into().unwrap()
}
