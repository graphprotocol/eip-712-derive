//! eip-712-derive: The `derive` is aspirational

mod atomic_types;
pub mod chain_id;
mod dynamic_types;
mod prelude;
mod type_hash;
mod types;
extern crate lazy_static;

use clear_on_drop::clear_stack_on_return;
use prelude::*;
use secp256k1::{Message, RecoveryId, SecretKey, Signature};
use std::io::{Cursor, Write};

// API
pub use atomic_types::*;
pub use type_hash::{encode_type, type_hash};
pub use types::{MemberType, MemberVisitor, StructType};

#[derive(Copy, Clone)]
pub struct DomainSeparator(Bytes32);

impl DomainSeparator {
    pub const fn from_bytes(value: &Bytes32) -> Self {
        Self(*value)
    }
    /// Creates a DomainSeparator from an EIP712Domain
    /// The exact fields of the EIP712Domain are not enforced
    /// by EIP-712, but recommended fields include name, version
    /// chainId, verifyingContract, and salt. If using all of these,
    /// pass in the provided Eip712Domain struct
    pub fn new<T: StructType>(eip_712_domain: &T) -> Self {
        let hash = hash_struct(eip_712_domain);
        Self(hash)
    }

    pub fn as_bytes(&self) -> &Bytes32 {
        &self.0
    }
}

pub type PrivateKey = Bytes32;

pub struct Eip712Domain {
    pub name: String,
    pub version: String,
    pub chain_id: U256,
    pub verifying_contract: Bytes20,
    pub salt: Bytes32,
}

impl StructType for Eip712Domain {
    const TYPE_NAME: &'static str = "EIP712Domain";

    fn visit_members<T: MemberVisitor>(&self, v: &mut T) {
        v.visit("name", &self.name);
        v.visit("version", &self.version);
        v.visit("chainId", &self.chain_id);
        v.visit("verifyingContract", &self.verifying_contract);
        v.visit("salt", &self.salt);
    }
}

pub fn encode_data<T: StructType>(s: &T) -> Vec<u8> {
    let mut buffer = Vec::new();

    buffer.extend_from_slice(&type_hash(s));

    struct EncodeVisitor<'a> {
        buffer: &'a mut Vec<u8>,
    }
    let mut visitor = EncodeVisitor {
        buffer: &mut buffer,
    };
    impl MemberVisitor for EncodeVisitor<'_> {
        fn visit<T: MemberType>(&mut self, _name: &'static str, value: &T) {
            let member_value = value.encode_data();
            self.buffer.extend_from_slice(&member_value);
        }
    }
    s.visit_members(&mut visitor);
    buffer
}

pub fn hash_struct<T: StructType>(s: &T) -> Bytes32 {
    // hashStruct(s : 𝕊) = keccak256(typeHash ‖ encodeData(s))
    // Looks like typeHash is missing here! But, it's in encodeData.
    keccak(encode_data(s))
}

pub fn encode<T: StructType>(domain_separator: &DomainSeparator, message: &T) -> [u8; 66] {
    // encode(domainSeparator : 𝔹²⁵⁶, message : 𝕊) = "\x19\x01" ‖ domainSeparator ‖ hashStruct(message)
    let mut result = [0u8; 66];
    let mut cursor = Cursor::new(&mut result[..]);
    cursor.write("\x19\x01".as_bytes()).unwrap();
    cursor.write(domain_separator.as_bytes()).unwrap();
    cursor.write(&hash_struct(message)).unwrap();
    result
}

pub fn sign_hash<T: StructType>(domain_separator: &DomainSeparator, message: &T) -> Bytes32 {
    let data = encode(domain_separator, message);
    keccak(&data[..])
}

/// Returns the serialized secp256k1 signature and the recoveryId on success.
pub fn sign_typed<T: StructType>(
    domain_separator: &DomainSeparator,
    value: &T,
    key: &PrivateKey,
) -> Result<([u8; 64], u8), impl std::error::Error> {
    let message = Message::parse(&sign_hash(domain_separator, value));

    // Security: clear_stack_on_return zeroizes the temporary copy of SecretKey
    // created by SecretKey::parse
    let result =
        clear_stack_on_return::<_, Result<(Signature, RecoveryId), secp256k1::Error>>(1, || {
            let secret_key = SecretKey::parse(key)?;
            Ok(secp256k1::sign(&message, &secret_key))
        });

    // Shenanigans to satisfy the compiler.
    // `?` doesn't work because returning impl Error
    let (sig, recovery) = match result {
        Ok(s) => s,
        Err(e) => return Err(e),
    };

    Ok((sig.serialize(), recovery.serialize() + 27))
}
