//! Hexerator hello world example plugin

use hexerator_plugin_api::{
    HexeratorHandle, MethodParam, MethodResult, Plugin, PluginMethod, Value, ValueTy,
};

struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn name(&self) -> &str {
        "Hello world plugin"
    }

    fn desc(&self) -> &str {
        "Hi! I'm an example plugin for Hexerator"
    }

    fn methods(&self) -> Vec<hexerator_plugin_api::PluginMethod> {
        vec![
            PluginMethod {
                method_name: "say_hello",
                human_name: Some("Say hello"),
                desc: "Write 'hello' to debug log.",
                params: &[],
            },
            PluginMethod {
                method_name: "fill_selection",
                human_name: Some("Fill selection"),
                desc: "Fills the selection with 0x42",
                params: &[],
            },
            PluginMethod {
                method_name: "sum_range",
                human_name: None,
                desc: "Sums up the values in the provided range",
                params: &[
                    MethodParam {
                        name: "from",
                        ty: ValueTy::U64,
                    },
                    MethodParam {
                        name: "to",
                        ty: ValueTy::U64,
                    },
                ],
            },
        ]
    }

    fn on_method_called(
        &mut self,
        name: &str,
        params: &[Option<Value>],
        hx: &mut dyn HexeratorHandle,
    ) -> MethodResult {
        match name {
            "say_hello" => {
                hx.debug_log("Hello world!");
                Ok(None)
            }
            "fill_selection" => match hx.selection_range() {
                Some((start, end)) => match hx.get_data_mut(start, end) {
                    Some(data) => {
                        data.fill(0x42);
                        Ok(None)
                    }
                    None => Err("Selection out of bounds".into()),
                },
                None => Err("Selection unavailable".into()),
            },
            "sum_range" => {
                let &[Some(Value::U64(from)), Some(Value::U64(to))] = params else {
                    return Err("Invalid params".into());
                };
                match hx.get_data_mut(from as usize, to as usize) {
                    Some(data) => {
                        let sum: u64 = data.iter().map(|b| *b as u64).sum();
                        Ok(Some(Value::U64(sum)))
                    }
                    None => Err("Out of bounds".into()),
                }
            }
            _ => Err(format!("Unknown method: {name}")),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "Rust" fn hexerator_plugin_new() -> Box<dyn Plugin> {
    Box::new(HelloPlugin)
}
