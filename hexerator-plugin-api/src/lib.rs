pub trait Plugin {
    fn name(&self) -> &str;
    fn desc(&self) -> &str;
    fn methods(&self) -> Vec<PluginMethod>;
    fn on_method_called(
        &mut self,
        name: &str,
        params: &[Option<Value>],
        hx: &mut dyn HexeratorHandle,
    ) -> MethodResult;
}

pub type MethodResult = Result<Option<Value>, String>;

pub struct PluginMethod {
    pub method_name: &'static str,
    pub human_name: Option<&'static str>,
    pub desc: &'static str,
    pub params: &'static [MethodParam],
}

pub struct MethodParam {
    pub name: &'static str,
    pub ty: ValueTy,
}

pub enum ValueTy {
    U64,
    String,
}

pub enum Value {
    U64(u64),
    F64(f64),
    String(String),
}

impl ValueTy {
    pub fn label(&self) -> &'static str {
        match self {
            ValueTy::U64 => "u64",
            ValueTy::String => "string",
        }
    }
}

pub trait HexeratorHandle {
    fn selection_range(&self) -> Option<(usize, usize)>;
    fn get_data(&self, start: usize, end: usize) -> Option<&[u8]>;
    fn get_data_mut(&mut self, start: usize, end: usize) -> Option<&mut [u8]>;
    fn debug_log(&self, msg: &str);
}
