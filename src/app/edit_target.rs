#[derive(PartialEq, Eq, Debug)]
pub enum EditTarget {
    Hex,
    Text,
}

impl EditTarget {
    pub fn switch(&mut self) {
        *self = match self {
            EditTarget::Hex => EditTarget::Text,
            EditTarget::Text => EditTarget::Hex,
        }
    }
}
