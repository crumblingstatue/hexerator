pub trait StrExt {
    fn is_empty_or_ws_only(&self) -> bool;
}

impl StrExt for str {
    fn is_empty_or_ws_only(&self) -> bool {
        self.trim().is_empty()
    }
}
