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
    F32Le(F32Le),
    F32Be(F32Be),
    F64Le(F64Le),
    F64Be(F64Be),
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
            Self::None => "none",
            Self::I8(v) => v.label(),
            Self::U8(v) => v.label(),
            Self::I16Le(v) => v.label(),
            Self::U16Le(v) => v.label(),
            Self::I16Be(v) => v.label(),
            Self::U16Be(v) => v.label(),
            Self::I32Le(v) => v.label(),
            Self::U32Le(v) => v.label(),
            Self::I32Be(v) => v.label(),
            Self::U32Be(v) => v.label(),
            Self::I64Le(v) => v.label(),
            Self::U64Le(v) => v.label(),
            Self::I64Be(v) => v.label(),
            Self::U64Be(v) => v.label(),
            Self::F32Le(v) => v.label(),
            Self::F32Be(v) => v.label(),
            Self::F64Le(v) => v.label(),
            Self::F64Be(v) => v.label(),
            Self::StringMap(v) => v.label(),
        }
    }

    pub(crate) fn byte_len(&self) -> usize {
        match self {
            Self::None => 1,
            Self::I8(_) => 1,
            Self::U8(_) => 1,
            Self::I16Le(_) => 2,
            Self::U16Le(_) => 2,
            Self::I16Be(_) => 2,
            Self::U16Be(_) => 2,
            Self::I32Le(_) => 4,
            Self::U32Le(_) => 4,
            Self::I32Be(_) => 4,
            Self::U32Be(_) => 4,
            Self::I64Le(_) => 8,
            Self::U64Le(_) => 8,
            Self::I64Be(_) => 8,
            Self::U64Be(_) => 8,
            Self::F32Le(_) => 4,
            Self::F32Be(_) => 4,
            Self::F64Le(_) => 8,
            Self::F64Be(_) => 8,
            Self::StringMap(_) => 1,
        }
    }

    pub fn read(&self, data: &[u8]) -> anyhow::Result<ReadValue> {
        macro_rules! r {
            ($t:ident $($en:ident)?) => {
                paste::paste! {
                    ReadValue::$t(read::<[<$t $($en)?>]>(data)?)
                }
            }
        }
        Ok(match self {
            Self::None => r!(U8),
            Self::I8(_) => r!(I8),
            Self::U8(_) => r!(U8),
            Self::I16Le(_) => r!(I16 Le),
            Self::U16Le(_) => r!(U16 Le),
            Self::I16Be(_) => r!(I16 Be),
            Self::U16Be(_) => r!(U16 Be),
            Self::I32Le(_) => r!(I32 Le),
            Self::U32Le(_) => r!(U32 Le),
            Self::I32Be(_) => r!(I32 Be),
            Self::U32Be(_) => r!(U32 Be),
            Self::I64Le(_) => r!(I64 Le),
            Self::U64Le(_) => r!(U64 Le),
            Self::I64Be(_) => r!(I64 Be),
            Self::U64Be(_) => r!(U64 Be),
            Self::F32Le(_) => r!(F32 Le),
            Self::F32Be(_) => r!(F32 Be),
            Self::F64Le(_) => r!(F64 Le),
            Self::F64Be(_) => r!(F64 Be),
            Self::StringMap(_) => r!(U8),
        })
    }
}

fn read<P: EndianedPrimitive>(data: &[u8]) -> Result<P::Primitive, anyhow::Error>
where
    [(); P::BYTE_LEN]:,
{
    Ok(P::from_bytes(data[..P::BYTE_LEN].try_into()?))
}

pub enum ReadValue {
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
}

impl std::fmt::Display for ReadValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::U8(v) => v.fmt(f),
            Self::I8(v) => v.fmt(f),
            Self::I16(v) => v.fmt(f),
            Self::U16(v) => v.fmt(f),
            Self::I32(v) => v.fmt(f),
            Self::U32(v) => v.fmt(f),
            Self::I64(v) => v.fmt(f),
            Self::U64(v) => v.fmt(f),
            Self::F32(v) => v.fmt(f),
            Self::F64(v) => v.fmt(f),
        }
    }
}

pub trait EndianedPrimitive {
    const BYTE_LEN: usize = size_of::<Self::Primitive>();
    type Primitive: egui::emath::Numeric + std::fmt::Display + core::str::FromStr;
    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive;
    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN];
    fn label(&self) -> &'static str;
    fn from_byte_slice(slice: &[u8]) -> Option<Self::Primitive>
    where
        [(); Self::BYTE_LEN]:,
    {
        match slice.try_into() {
            Ok(slice) => Some(Self::from_bytes(slice)),
            Err(_) => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct I8;

impl EndianedPrimitive for I8 {
    type Primitive = i8;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        i8::from_ne_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_ne_bytes()
    }

    fn label(&self) -> &'static str {
        "i8"
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct U8;

impl EndianedPrimitive for U8 {
    type Primitive = u8;

    fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
        u8::from_ne_bytes(bytes)
    }

    fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
        prim.to_ne_bytes()
    }

    fn label(&self) -> &'static str {
        "u8"
    }
}

macro_rules! impl_for_num {
    ($($wrap:ident => $prim:ident $en:ident,)*) => {
        $(
            #[derive(Serialize, Deserialize, Clone)]
            pub struct $wrap;

            impl EndianedPrimitive for $wrap {
                type Primitive = $prim;

                fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self::Primitive {
                    $prim::${concat(from_, $en, _bytes)}(bytes)
                }

                fn to_bytes(prim: Self::Primitive) -> [u8; Self::BYTE_LEN] {
                    prim.${concat(to_, $en, _bytes)}()
                }

                fn label(&self) -> &'static str {
                    concat!(stringify!($prim), "-", stringify!($en))
                }
            }
        )*
    }
}

impl_for_num! {
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
    F32Le => f32 le,
    F32Be => f32 be,
    F64Le => f64 le,
    F64Be => f64 be,
}
