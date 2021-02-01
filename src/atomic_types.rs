use crate::prelude::*;

macro_rules! impl_atomic {
    ($T: ident, $name:expr, $this:ident $encode_data:tt) => {
        impl MemberType for $T {
            const TYPE_NAME: &'static str = $name;
            fn encode_data(&$this) -> Bytes32 $encode_data
            #[inline(always)]
            fn add_members(&self, _builder: &mut TypeHashBuilder) {}
        }
        impl AtomicType for $T {}
    };
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Address(pub Bytes20);
#[derive(Default, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct U256(pub Bytes32);

impl_atomic!(Address, "address", self { self.0.encode_data() });
impl_atomic!(U256, "uint256", self { self.0.encode_data() });

macro_rules! impl_bytes {
    ($($T:ident: $size:expr => $name:expr,)+) => {
        $(
            pub type $T = [u8; $size];
            impl_atomic!($T, $name, self {
                let mut padded = [0u8; 32];
                let section = &mut padded[32 - $size..];
                section.copy_from_slice(&self[..]);
                padded
            });
        )+
    }
}

impl_bytes! {
    Bytes1: 1 => "bytes1",
    Bytes2: 2 => "bytes2",
    Bytes3: 3 => "bytes3",
    Bytes4: 4 => "bytes4",
    Bytes5: 5 => "bytes5",
    Bytes6: 6 => "bytes6",
    Bytes7: 7 => "bytes7",
    Bytes8: 8 => "bytes8",
    Bytes9: 9 => "bytes9",
    Bytes10: 10 => "bytes10",
    Bytes11: 11 => "bytes11",
    Bytes12: 12 => "bytes12",
    Bytes13: 13 => "bytes13",
    Bytes14: 14 => "bytes14",
    Bytes15: 15 => "bytes15",
    Bytes16: 16 => "bytes16",
    Bytes17: 17 => "bytes17",
    Bytes18: 18 => "bytes18",
    Bytes19: 19 => "bytes19",
    Bytes20: 20 => "bytes20",
    Bytes21: 21 => "bytes21",
    Bytes22: 22 => "bytes22",
    Bytes23: 23 => "bytes23",
    Bytes24: 24 => "bytes24",
    Bytes25: 25 => "bytes25",
    Bytes26: 26 => "bytes26",
    Bytes27: 27 => "bytes27",
    Bytes28: 28 => "bytes28",
    Bytes29: 29 => "bytes29",
    Bytes30: 30 => "bytes30",
    Bytes31: 31 => "bytes31",
    Bytes32: 32 => "bytes32",
}
