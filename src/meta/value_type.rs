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
    U64Le(U64Le),
    StringMap(StringMap),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct U8;
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct U16Le;
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct U64Le;

pub type StringMap = HashMap<u8, String>;

impl ValueType {
    pub fn label(&self) -> &str {
        match self {
            ValueType::None => "none",
            ValueType::U8(_) => "u8",
            ValueType::U16Le(_) => "u16-le",
            ValueType::U64Le(_) => "u64-le",
            ValueType::StringMap(_) => "string list",
        }
    }
}
