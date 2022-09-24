use {
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct U8;
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct U16Le;
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct U16Be;
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct U32Le;
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct U32Be;
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct U64Le;
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
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
