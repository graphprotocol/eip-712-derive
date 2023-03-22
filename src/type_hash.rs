use crate::prelude::*;
use std::any::TypeId;
use std::collections::BTreeMap;

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
        s.push('(');
        let mut members = t.members.iter();
        if let Some(member) = members.next() {
            member.write(s);
        }
        for member in members {
            s.push(',');
            member.write(s);
        }
        s.push(')');
    }

    add_type(&mut buffer, &outer);
    for inner in builder.inner.values() {
        add_type(&mut buffer, inner);
    }
    buffer
}

/// Memoized type hash
pub fn type_hash<T: StructType>(value: &T) -> Bytes32 {
    // (SPEC) keccak256(encodeType(typeOf(s)))
    let encoded = encode_type(value);
    keccak(encoded.as_bytes())
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
    pub fn struct_type<T: StructType>(&mut self) -> StructTypeBuilder {
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

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
struct Member {
    pub r#type: &'static str,
    pub name: &'static str,
}

impl Member {
    pub fn write(&self, buffer: &mut String) {
        buffer.push_str(self.r#type);
        buffer.push(' ');
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
        if let Some(encoded_type) = self.parent.get_encoded_type_mut(T::TYPE_NAME) {
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
        value.add_members(self.parent);
    }
}
