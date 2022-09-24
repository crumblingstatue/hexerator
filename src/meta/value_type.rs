use {
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub enum ValueType {
    #[default]
    None,
    U8,
    U16Le,
    U64Le,
    StringMap(HashMap<u8, String>),
}

impl ValueType {
    pub fn label(&self) -> &str {
        match self {
            ValueType::None => "none",
            ValueType::U8 => "u8",
            ValueType::U16Le => "u16-le",
            ValueType::U64Le => "u64-le",
            ValueType::StringMap(_) => "string list",
        }
    }
}
