use crate::prelude::*;

impl DynamicType for String {}

impl MemberType for String {
    const TYPE_NAME: &'static str = "string";
    fn encode_data(&self) -> Bytes32 {
        keccak(self)
    }
    #[inline(always)]
    fn add_members(&self, _builder: &mut TypeHashBuilder) {}
}

// TODO: Vec<u8>
