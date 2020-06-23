//! eip-712-derive: The `derive` is aspirational

mod atomic_types;
mod dynamic_types;
mod prelude;
mod type_hash;
mod types;
extern crate lazy_static;

use clear_on_drop::clear_stack_on_return;
use prelude::*;
use secp256k1::{sign, Message, RecoveryId, SecretKey, Signature};
use std::io::{Cursor, Write};

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
    name: String,
    version: String,
    chain_id: U256,
    verifying_contract: Bytes20,
    salt: Bytes32,
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

fn hash_struct<T: StructType>(s: &T) -> Bytes32 {
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
            Ok(sign(&message, &secret_key))
        });

    // Shenanigans to satisfy the compiler.
    // `?` doesn't work because returning impl Error
    let (sig, recovery) = match result {
        Ok(s) => s,
        Err(e) => return Err(e),
    };

    Ok((sig.serialize(), recovery.serialize() + 27))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;
    use std::convert::TryInto;

    // No salt
    struct DomainStruct {
        name: String,
        version: String,
        chain_id: U256,
        verifying_contract: Address,
    }

    impl StructType for DomainStruct {
        const TYPE_NAME: &'static str = "EIP712Domain";
        fn visit_members<T: MemberVisitor>(&self, visitor: &mut T) {
            visitor.visit("name", &self.name);
            visitor.visit("version", &self.version);
            visitor.visit("chainId", &self.chain_id);
            visitor.visit("verifyingContract", &self.verifying_contract);
        }
    }

    struct Person {
        name: String,
        wallet: Address,
    }
    impl StructType for Person {
        const TYPE_NAME: &'static str = "Person";
        fn visit_members<T: MemberVisitor>(&self, visitor: &mut T) {
            visitor.visit("name", &self.name);
            visitor.visit("wallet", &self.wallet);
        }
    }

    struct Mail {
        from: Person,
        to: Person,
        contents: String,
    }
    impl StructType for Mail {
        const TYPE_NAME: &'static str = "Mail";
        fn visit_members<T: MemberVisitor>(&self, visitor: &mut T) {
            visitor.visit("from", &self.from);
            visitor.visit("to", &self.to);
            visitor.visit("contents", &self.contents);
        }
    }

    #[test]
    fn spec_case() {
        // Taken from the JSON RPC section of the spec,
        // as well as the accompanying example here:
        // https://github.com/ethereum/EIPs/blob/master/assets/eip-712/Example.js

        let domain = DomainStruct {
            name: "Ether Mail".to_owned(),
            version: "1".to_owned(),
            chain_id: U256([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            verifying_contract: Address(
                (&(hex::decode("CcCCccccCCCCcCCCCCCcCcCccCcCCCcCcccccccC").unwrap())[..])
                    .try_into()
                    .unwrap(),
            ),
        };
        let domain_separator = DomainSeparator::new(&domain);

        let message = Mail {
            from: Person {
                name: "Cow".to_owned(),
                wallet: Address(
                    (&(hex::decode("CD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826").unwrap())[..])
                        .try_into()
                        .unwrap(),
                ),
            },
            to: Person {
                name: "Bob".to_owned(),
                wallet: Address(
                    (&(hex::decode("bBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB").unwrap())[..])
                        .try_into()
                        .unwrap(),
                ),
            },
            contents: "Hello, Bob!".to_owned(),
        };

        assert_eq!(
            &encode_type(&message),
            "Mail(Person from,Person to,string contents)Person(string name,address wallet)"
        );
        assert_eq!(
            &hex::encode(type_hash(&message)),
            "a0cedeb2dc280ba39b857546d74f5549c3a1d7bdc2dd96bf881f76108e23dac2"
        );

        assert_eq!(
            &hex::encode(encode_data(&message)),
            "a0cedeb2dc280ba39b857546d74f5549c3a1d7bdc2dd96bf881f76108e23dac2fc71e5fa27ff56c350aa531bc129ebdf613b772b6604664f5d8dbe21b85eb0c8cd54f074a4af31b4411ff6a60c9719dbd559c221c8ac3492d9d872b041d703d1b5aadf3154a261abdd9086fc627b61efca26ae5702701d05cd2305f7c52a2fc8"
        );

        assert_eq!(
            &hex::encode(hash_struct(&message)),
            "c52c0ee5d84264471806290a3f2c4cecfc5490626bf912d01f240d7a274b371e"
        );

        assert_eq!(
            &hex::encode(domain_separator.as_bytes()),
            "f2cee375fa42b42143804025fc449deafd50cc031ca257e0b194a650a912090f"
        );

        assert_eq!(
            &hex::encode(sign_hash(&domain_separator, &message)),
            "be609aee343fb3c4b28e1df9e632fca64fcfaede20f02e86244efddf30957bd2",
        );

        let pk = keccak("cow".as_bytes());

        let result = sign_typed(&domain_separator, &message, &pk).unwrap();
        let mut serialized = Vec::new();
        serialized.extend_from_slice(&result.0);
        serialized.push(result.1);
        let result = hex::encode(&serialized);
        let expected = "4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c";

        assert_eq!(expected, result);
    }
}
