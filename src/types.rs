use crate::prelude::*;

/// (SPEC) Definition: The atomic types are bytes1 to bytes32, uint8 to uint256, int8
/// to int256, bool and address. These correspond to their definition in
/// Solidity. Note that there are no aliases uint and int. Note that contract
/// addresses are always plain address. Fixed point numbers are not supported by
/// the standard. Future versions of this standard may add new atomic types.
pub trait AtomicType {}

/// (SPEC) Definition: The dynamic types are bytes and string. These are like the
/// atomic types for the purposed of type declaration, but their treatment in
/// encoding is different.
pub trait DynamicType {}

/// (SPEC) Definition: The reference types are arrays and structs. Arrays are
/// either fixed size or dynamic and denoted by Type[n] or Type[] respectively.
/// Structs are references to other structs by their name. The standard supports
/// recursive struct types.
///
/// TODO: Technically we only need visit_members on this type, but that would
/// require specialization.
pub trait ReferenceType {}

/// (SPEC) Definition: A struct type has valid identifier as name and contains zero or
/// more member variables. Member variables have a member type and a name.
// TODO: We would like to remove the 'static bound, but it is necessary for obtaining the TypeId,
// which is a part of verifying unique names for types
pub trait StructType: 'static {
    const TYPE_NAME: &'static str;
    /// Call visitor.visit on each of the fields.
    ///
    /// This API exists to make it very easy to implement, without requiring too much
    /// very similar boilerplate for the requirements of add_members and encode_data.
    /// It will likely go away if a derive is added.
    fn visit_members<T: MemberVisitor>(&self, visitor: &mut T);
}

pub trait MemberVisitor {
    /// The name should be the Ethereum name (usually camel case)
    fn visit<T: MemberType>(&mut self, name: &'static str, value: &T);
}

/// (SPEC) Definition: A member type can be either an atomic type, a dynamic
/// type or a reference type.
///
/// There is no need for a consumer of a crate to implement this manually.
/// It is easier to implement StructType instead.
pub trait MemberType: 'static {
    const TYPE_NAME: &'static str;
    fn encode_data(&self) -> Bytes32;
    fn add_members(&self, builder: &mut TypeHashBuilder);
}

impl<T: StructType> MemberType for T {
    const TYPE_NAME: &'static str = T::TYPE_NAME;
    fn add_members(&self, builder: &mut TypeHashBuilder) {
        let mut builder = builder.struct_type::<T>();
        self.visit_members(&mut builder);
    }
    fn encode_data(&self) -> Bytes32 {
        crate::hash_struct(self)
    }
}

impl<T: StructType> ReferenceType for T {}
// We would like to simply do the following, but this has to wait on
// some variation of https://github.com/rust-lang/rfcs/issues/1053
// For the moment we auto-impl for StructType only, and
// manually implement the rest for Dynamic and Atomic types.
// This makes sense, because all atomic and dynamic types are implemented
// in this crate, we so we leave less work for consumers of this API
//
//impl<T: DynamicType> MemberType for T {}
//impl<T: AtomicType> MemberType for T {}
