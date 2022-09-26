use {
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
};

#[derive(Serialize, Deserialize, Clone, Default)]
pub enum ValueType {
    #[default]
    None,
    U8(U8),
    U16Le(U16Le),
    U16Be(U16Be),
    U32Le(U32Le),
    U32Be(U32Be),
    U64Le(U64Le),
    U64Be(U64Be),
    StringMap(StringMap),
}

impl PartialEq for ValueType {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct U8;
#[derive(Serialize, Deserialize, Clone)]
pub struct U16Le;
#[derive(Serialize, Deserialize, Clone)]
pub struct U16Be;
#[derive(Serialize, Deserialize, Clone)]
pub struct U32Le;
#[derive(Serialize, Deserialize, Clone)]
pub struct U32Be;
#[derive(Serialize, Deserialize, Clone)]
pub struct U64Le;
#[derive(Serialize, Deserialize, Clone)]
pub struct U64Be;

pub type StringMap = HashMap<u8, String>;

impl ValueType {
    pub fn label(&self) -> &str {
        match self {
            ValueType::None => "none",
            ValueType::U8(_) => "u8",
            ValueType::U16Le(_) => "u16-le",
            ValueType::U64Le(_) => "u64-le",
            ValueType::StringMap(_) => "string list",
            ValueType::U16Be(_) => "u16-be",
            ValueType::U32Le(_) => "u32-le",
            ValueType::U32Be(_) => "u32-be",
            ValueType::U64Be(_) => "u64-be",
        }
    }
}

pub trait EndianedPrimitive {
    const BYTE_LEN: usize = std::mem::size_of::<Self::Primitive>();
    type Primitive: egui::emath::Numeric + std::fmt::Display + TryInto<usize>;
    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive;
    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN];
}

impl EndianedPrimitive for U8 {
    type Primitive = u8;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        bytes[0]
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        [prim]
    }
}

impl EndianedPrimitive for U16Le {
    type Primitive = u16;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        u16::from_le_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_le_bytes()
    }
}

impl EndianedPrimitive for U16Be {
    type Primitive = u16;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        u16::from_be_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_be_bytes()
    }
}

impl EndianedPrimitive for U32Le {
    type Primitive = u32;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        u32::from_le_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_le_bytes()
    }
}

impl EndianedPrimitive for U32Be {
    type Primitive = u32;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        u32::from_be_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_be_bytes()
    }
}

impl EndianedPrimitive for U64Le {
    type Primitive = u64;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        u64::from_le_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_le_bytes()
    }
}

impl EndianedPrimitive for U64Be {
    type Primitive = u64;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        u64::from_be_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_be_bytes()
    }
}
