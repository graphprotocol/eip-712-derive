pub(crate) use crate::atomic_types::*;
pub(crate) use crate::type_hash::*;
pub(crate) use crate::types::*;

pub(crate) fn keccak<T: AsRef<[u8]>>(buffer: T) -> Bytes32 {
    keccak_hash::keccak(buffer).to_fixed_bytes()
}
