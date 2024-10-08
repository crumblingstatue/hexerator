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
            ValueType::F32Le(v) => v.label(),
            ValueType::F32Be(v) => v.label(),
            ValueType::F64Le(v) => v.label(),
            ValueType::F64Be(v) => v.label(),
            ValueType::StringMap(v) => v.label(),
        }
    }

    pub(crate) fn byte_len(&self) -> usize {
        match self {
            ValueType::None => 1,
            ValueType::I8(_) => 1,
            ValueType::U8(_) => 1,
            ValueType::I16Le(_) => 2,
            ValueType::U16Le(_) => 2,
            ValueType::I16Be(_) => 2,
            ValueType::U16Be(_) => 2,
            ValueType::I32Le(_) => 4,
            ValueType::U32Le(_) => 4,
            ValueType::I32Be(_) => 4,
            ValueType::U32Be(_) => 4,
            ValueType::I64Le(_) => 8,
            ValueType::U64Le(_) => 8,
            ValueType::I64Be(_) => 8,
            ValueType::U64Be(_) => 8,
            ValueType::F32Le(_) => 4,
            ValueType::F32Be(_) => 4,
            ValueType::F64Le(_) => 8,
            ValueType::F64Be(_) => 8,
            ValueType::StringMap(_) => 1,
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
            ValueType::None => r!(U8),
            ValueType::I8(_) => r!(I8),
            ValueType::U8(_) => r!(U8),
            ValueType::I16Le(_) => r!(I16 Le),
            ValueType::U16Le(_) => r!(U16 Le),
            ValueType::I16Be(_) => r!(I16 Be),
            ValueType::U16Be(_) => r!(U16 Be),
            ValueType::I32Le(_) => r!(I32 Le),
            ValueType::U32Le(_) => r!(U32 Le),
            ValueType::I32Be(_) => r!(I32 Be),
            ValueType::U32Be(_) => r!(U32 Be),
            ValueType::I64Le(_) => r!(I64 Le),
            ValueType::U64Le(_) => r!(U64 Le),
            ValueType::I64Be(_) => r!(I64 Be),
            ValueType::U64Be(_) => r!(U64 Be),
            ValueType::F32Le(_) => r!(F32 Le),
            ValueType::F32Be(_) => r!(F32 Be),
            ValueType::F64Le(_) => r!(F64 Le),
            ValueType::F64Be(_) => r!(F64 Be),
            ValueType::StringMap(_) => r!(U8),
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
            ReadValue::U8(v) => v.fmt(f),
            ReadValue::I8(v) => v.fmt(f),
            ReadValue::I16(v) => v.fmt(f),
            ReadValue::U16(v) => v.fmt(f),
            ReadValue::I32(v) => v.fmt(f),
            ReadValue::U32(v) => v.fmt(f),
            ReadValue::I64(v) => v.fmt(f),
            ReadValue::U64(v) => v.fmt(f),
            ReadValue::F32(v) => v.fmt(f),
            ReadValue::F64(v) => v.fmt(f),
        }
    }
}

pub trait EndianedPrimitive {
    const BYTE_LEN: usize = std::mem::size_of::<Self::Primitive>();
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
