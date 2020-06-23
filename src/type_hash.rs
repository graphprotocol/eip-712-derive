use crate::prelude::*;
use lazy_static::lazy_static;
use std::any::TypeId;
use std::collections::{BTreeMap, HashMap};
use std::sync::RwLock;

// (SPEC) The type of a struct is encoded as name ‖ "(" ‖ member₁ ‖ "," ‖
// member₂ ‖ "," ‖ … ‖ memberₙ ")" where each member is written as type ‖ " " ‖
// name. For example, the above Mail struct is encoded as Mail(address
// from,address to,string contents)
pub fn encode_type<T: StructType>(value: &T) -> String {
    let mut builder = TypeHashBuilder {
        outer: None,
        inner: BTreeMap::new(),
    };

    value.add_members(&mut builder);

    let mut buffer = String::new();
    let outer = builder.outer.unwrap();
    assert!(outer.name == T::TYPE_NAME);

    fn add_type(s: &mut String, t: &EncodedType) {
        s.push_str(t.name);
        s.push_str("(");
        let mut members = t.members.iter();
        if let Some(member) = members.next() {
            member.write(s);
        }
        for member in members {
            s.push_str(",");
            member.write(s);
        }
        s.push_str(")");
    }

    add_type(&mut buffer, &outer);
    for inner in builder.inner.values() {
        add_type(&mut buffer, inner);
    }
    buffer
}

lazy_static! {
    static ref CACHE: RwLock<HashMap<TypeId, Bytes32>> = RwLock::new(HashMap::new());
}

/// Memoized type hash
pub fn type_hash<T: StructType>(value: &T) -> Bytes32 {
    let read = CACHE.read().unwrap();
    if let Some(cached) = read.get(&TypeId::of::<T>()) {
        return *cached;
    }
    drop(read);

    // (SPEC) keccak256(encodeType(typeOf(s)))
    let encoded = encode_type(value);
    let result = keccak(encoded.as_bytes());

    let mut write = CACHE.write().unwrap();
    write.insert(TypeId::of::<T>(), result);
    result
}

pub struct TypeHashBuilder {
    // (SPEC) If the struct type references other struct types (and these in
    // turn reference even more struct types), then the set of referenced struct
    // types is collected, sorted by name and appended to the encoding. An
    // example encoding is Transaction(Person from,Person to,Asset
    // tx)Asset(address token,uint256 amount)Person(address wallet,string name).
    //
    // NOTE: This means that the outer type gets special treatment, since it is not part
    // of the sorted set.
    outer: Option<EncodedType>,
    inner: BTreeMap<&'static str, EncodedType>,
}

impl TypeHashBuilder {
    fn get_encoded_type_mut(&mut self, name: &'static str) -> Option<&mut EncodedType> {
        if let Some(outer) = &self.outer {
            if outer.name == name {
                return self.outer.as_mut();
            }
        }
        self.inner.get_mut(name)
    }
    pub fn struct_type<'a, T: StructType>(&'a mut self) -> StructTypeBuilder<'a> {
        assert!(self.get_encoded_type_mut(T::TYPE_NAME).is_none());
        let value = EncodedType {
            type_id: TypeId::of::<T>(),
            name: T::TYPE_NAME,
            members: Vec::new(),
        };
        // Insert at this point as a marker to prevent recursion
        if self.outer.is_none() {
            self.outer = Some(value);
        } else {
            self.inner.insert(T::TYPE_NAME, value);
        }
        StructTypeBuilder {
            parent: self,
            own_type: T::TYPE_NAME,
        }
    }
}

#[derive(PartialEq, Eq)]
struct Member {
    pub r#type: &'static str,
    pub name: &'static str,
}

impl Member {
    pub fn write(&self, buffer: &mut String) {
        buffer.push_str(self.r#type);
        buffer.push_str(" ");
        buffer.push_str(self.name);
    }
}

struct EncodedType {
    type_id: TypeId,
    name: &'static str,
    members: Vec<Member>,
}

pub struct StructTypeBuilder<'a> {
    parent: &'a mut TypeHashBuilder,
    own_type: &'static str,
}

impl MemberVisitor for StructTypeBuilder<'_> {
    fn visit<T: MemberType>(&mut self, name: &'static str, value: &T) {
        // This unwrap is ok, because we know that this must exist because it was
        // added with this builder.
        let set = self.parent.get_encoded_type_mut(self.own_type).unwrap();
        let member = Member {
            name,
            r#type: T::TYPE_NAME,
        };
        // TODO: Assertion fail on duplicated member name?
        set.members.push(member);

        // Recurse into the members to add their types.
        // It's possible that types show up more than once, so we need
        // to check if this is a type we've already added. Recursion
        // is also possible, so verify that as well.
        match self.parent.get_encoded_type_mut(T::TYPE_NAME) {
            Some(encoded_type) => {
                // Ensure the uniqueness of type names. The spec doesn't seem to
                // address this, but it makes sense because with duplicated type
                // names the result of the sort by name step would be undefined.
                assert!(
                    encoded_type.type_id == TypeId::of::<T>(),
                    "Types with duplicated name: {}",
                    T::TYPE_NAME
                );
                return;
            }
            None => {}
        }
        value.add_members(self.parent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Transaction {
        from: Person,
        to: Person,
        tx: Asset,
    }

    impl StructType for Transaction {
        const TYPE_NAME: &'static str = "Transaction";
        fn visit_members<T: MemberVisitor>(&self, visitor: &mut T) {
            visitor.visit("from", &self.from);
            visitor.visit("to", &self.to);
            visitor.visit("tx", &self.tx);
        }
    }

    #[derive(Default)]
    struct Person {
        wallet: Address,
        name: String,
    }
    impl StructType for Person {
        const TYPE_NAME: &'static str = "Person";
        fn visit_members<T: MemberVisitor>(&self, visitor: &mut T) {
            visitor.visit("wallet", &self.wallet);
            visitor.visit("name", &self.name);
        }
    }

    #[derive(Default)]
    struct Asset {
        token: Address,
        amount: U256,
    }

    impl StructType for Asset {
        const TYPE_NAME: &'static str = "Asset";
        fn visit_members<T: MemberVisitor>(&self, visitor: &mut T) {
            visitor.visit("token", &self.token);
            visitor.visit("amount", &self.amount);
        }
    }

    #[test]
    fn encode_transaction_type() {
        let expected = "Transaction(Person from,Person to,Asset tx)Asset(address token,uint256 amount)Person(address wallet,string name)";

        let value: Transaction = Default::default();
        assert_eq!(encode_type(&value), expected);
    }
}
