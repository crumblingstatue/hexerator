use {
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
};

#[derive(Serialize, Deserialize, Clone, Default)]
pub enum ValueType {
    #[default]
    None,
    I8(I8),
    U8(U8),
    I16Le(I16Le),
    U16Le(U16Le),
    I16Be(I16Be),
    U16Be(U16Be),
    I32Le(I32Le),
    U32Le(U32Le),
    I32Be(I32Be),
    U32Be(U32Be),
    I64Le(I64Le),
    U64Le(U64Le),
    I64Be(I64Be),
    U64Be(U64Be),
    StringMap(StringMap),
}

impl PartialEq for ValueType {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

pub type StringMap = HashMap<u8, String>;

impl ValueType {
    pub fn label(&self) -> &str {
        match self {
            ValueType::None => "none",
            ValueType::I8(v) => v.label(),
            ValueType::U8(v) => v.label(),
            ValueType::I16Le(v) => v.label(),
            ValueType::U16Le(v) => v.label(),
            ValueType::I16Be(v) => v.label(),
            ValueType::U16Be(v) => v.label(),
            ValueType::I32Le(v) => v.label(),
            ValueType::U32Le(v) => v.label(),
            ValueType::I32Be(v) => v.label(),
            ValueType::U32Be(v) => v.label(),
            ValueType::I64Le(v) => v.label(),
            ValueType::U64Le(v) => v.label(),
            ValueType::I64Be(v) => v.label(),
            ValueType::U64Be(v) => v.label(),
            ValueType::StringMap(v) => v.label(),
        }
    }
}

pub trait EndianedPrimitive {
    const BYTE_LEN: usize = std::mem::size_of::<Self::Primitive>();
    type Primitive: egui::emath::Numeric + std::fmt::Display + TryInto<usize>;
    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive;
    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN];
    fn label(&self) -> &'static str;
}

impl_for_int! {
    I8 => i8 ne,
    U8 => u8 ne,
    I16Le => i16 le,
    U16Le => u16 le,
    I16Be => i16 be,
    U16Be => u16 be,
    I32Le => i32 le,
    U32Le => u32 le,
    I32Be => i32 be,
    U32Be => u32 be,
    I64Le => i64 le,
    U64Le => u64 le,
    I64Be => i64 be,
    U64Be => u64 be,
}

macro impl_for_int($($wrap:ident => $prim:ident $en:ident,)*) {
    $(
        #[derive(Serialize, Deserialize, Clone)]
        pub struct $wrap;

        impl EndianedPrimitive for $wrap {
            type Primitive = $prim;

            paste::paste! {
                fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
                    $prim::[<from_ $en _bytes>](bytes)
                }

                fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
                    prim.[<to_ $en _bytes>]()
                }

                fn label(&self) -> &'static str {
                    concat!(stringify!($prim), "-", stringify!($en))
                }
            }
        }
    )*
}
