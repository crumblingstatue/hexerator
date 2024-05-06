use {
    crate::{
        app::App,
        gui::{dialogs::pattern_fill::parse_pattern_string, ConMsg, Gui},
        meta::{
            region::Region,
            value_type::{self, EndianedPrimitive as _, ValueType},
            Bookmark, NamedRegion, ScriptKey,
        },
        slice_ext::SliceExt as _,
    },
    anyhow::Context,
    egui_sfml::sfml::graphics::Font,
    mlua::{ExternalError as _, ExternalResult as _, IntoLuaMulti, Lua, UserData},
    std::collections::HashMap,
};

pub struct LuaExecContext<'app, 'gui, 'font> {
    pub app: &'app mut App,
    pub gui: &'gui mut Gui,
    pub font: &'font Font,
    pub key: Option<ScriptKey>,
}

pub(crate) trait Method<'lua> {
    /// Name of the method
    const NAME: &'static str;
    /// Help text for the method
    const HELP: &'static str;
    /// Stringified API signature for help purposes
    const API_SIG: &'static str;
    /// Arguments the method takes when called
    type Args;
    /// Return type
    type Ret: IntoLuaMulti<'lua>;
    /// The function that gets called
    fn call(lua: &'lua Lua, exec: &mut LuaExecContext, args: Self::Args)
        -> mlua::Result<Self::Ret>;
}

macro_rules! def_method {
    ($help:literal $name:ident($lua:ident, $exec:ident, $($argname:ident: $argty:ty),*) -> $ret:ty $block:block) => {
        #[allow(non_camel_case_types)] pub(crate) enum $name {}
        impl<'lua> Method<'lua> for $name {
            const NAME: &'static str = stringify!($name);
            const HELP: &'static str = $help;
            const API_SIG: &'static str = concat!(stringify!($name), "(", $(stringify!($argname), ": ", stringify!($argty), ", ",)* ")", " -> ", stringify!($ret));
            type Args = ($($argty,)*);
            type Ret = $ret;
            fn call($lua: &'lua Lua, $exec: &mut LuaExecContext, ($($argname,)*): ($($argty,)*)) -> mlua::Result<$ret> $block
        }
    };
}

def_method! {
    "Adds a region to the meta"
    add_region(_lua, exec, name: String, begin: usize, end: usize) -> () {
        exec.app.meta_state.meta.low.regions.insert(NamedRegion {
            name,
            desc: String::new(),
            region: Region { begin, end },
        });
        Ok(())
    }
}

def_method! {
    "Loads a file"
    load_file(_lua, exec, path: String) -> () {
        exec.app
            .load_file(path.into(), true, exec.font, &mut exec.gui.msg_dialog)
            .map_err(|e| e.into_lua_err())?;
        Ok(())
    }
}

def_method! {
    "Sets the value pointed to by the bookmark to an integer value"
    bookmark_set_int(_lua, exec, name: String, val: i64) -> () {
        let bm = exec
            .app
            .meta_state
            .meta
            .bookmark_by_name_mut(&name)
            .ok_or("no such bookmark".into_lua_err())?;
        bm.write_int(&mut exec.app.data[bm.offset..], val).map_err(|e| e.into_lua_err())?;
        Ok(())
    }
}

def_method! {
    "Fills a named region with a pattern"
    region_pattern_fill(_lua, exec, name: String, pattern: String) -> () {
        let reg = exec
            .app
            .meta_state
            .meta
            .region_by_name_mut(&name)
            .ok_or("no such region".into_lua_err())?;
        let pat = parse_pattern_string(&pattern).map_err(|e| e.into_lua_err())?;
        exec.app.data[reg.region.begin..=reg.region.end].pattern_fill(&pat);
        Ok(())
    }
}

def_method! {
    "Returns an array containing the offsets of the find results"
    find_result_offsets(_lua, exec,) -> Vec<usize> {
        Ok(exec.gui.find_dialog.results_vec.clone())
    }
}

def_method! {
    "Reads an unsigned 8 bit integer at `offset`"
    read_u8(_lua, exec, offset: usize) -> u8 {
        match exec.app.data.get(offset) {
            Some(byte) => Ok(*byte),
            None => Err("out of bounds".into_lua_err()),
        }
    }
}

def_method! {
    "Sets unsigned 8 bit integer at `offset` to `value`"
    write_u8(_lua, exec, offset: usize, value: u8) -> () {
        match exec.app.data.get_mut(offset) {
            Some(byte) => {
                *byte = value;
                Ok(())
            }
            None => Err("out of bounds".into_lua_err())
        }
    }
}

def_method! {
    "Reads a little endian unsigned 32 bit integer at `offset`"
    read_u32_le(_lua, exec, offset: usize) -> u32 {
        match exec
        .app
        .data
        .get(offset..offset + 4)
    {
        Some(slice) => value_type::U32Le::from_byte_slice(slice)
            .ok_or_else(|| "Failed to convert".into_lua_err()),
        None => Err("out of bounds".into_lua_err()),
    }
    }
}

def_method! {
    "Fills a range from `start` to `end` with the value `fill`"
    fill_range(_lua, exec, start: usize, end: usize, fill: u8) -> () {
        match exec
              .app
              .data
              .get_mut(start..end) {
            Some(slice) => {
                slice.fill(fill);
                Ok(())
            }
            None => Err("out of bounds".into_lua_err()),
        }
    }
}

def_method! {
    "Sets the dirty region to `begin..=end`"
    set_dirty_region(_lua, exec, begin: usize, end: usize) -> () {
        exec.app.edit_state.dirty_region = Some(Region { begin, end });
        Ok(())
    }
}

def_method! {
    "Save the currently opened document (its dirty ranges)"
    save(_lua, exec,) -> () {
        exec.app.save(&mut exec.gui.msg_dialog).into_lua_err()?;
        Ok(())
    }
}

def_method! {
    "Returns the offset pointed to by the bookmark `name`"
    bookmark_offset(_lua, exec, name: String) -> usize {
        match exec
             .app
             .meta_state
             .meta
             .bookmark_by_name_mut(&name)
        {
            Some(bm) => Ok(bm.offset),
            None => Err(format!("no such bookmark: {name}").into_lua_err()),
        }
    }
}

def_method! {
    "Adds a bookmark with name `name`, pointing at `offset`"
    add_bookmark(_lua, exec, offset: usize, name: String) -> () {
        exec.app.meta_state.meta.bookmarks.push(Bookmark {
            offset,
            label: name,
            desc: String::new(),
            value_type: ValueType::None,
        });
        Ok(())
    }
}

def_method! {
    "Finds a hex string in the format '99 aa bb ...' format, and returns its offset"
    find_hex_string(_lua, exec, hex_string: String) -> Option<usize> {
        let mut offset = None;
        crate::gui::find_dialog::find_hex_string(&hex_string, &exec.app.data, |off| {
            offset = Some(off);
        }).into_lua_err()?;
        Ok(offset)
    }
}

def_method! {
    "Set the cursor to `offset`, center the view on the cursor, and flash the cursor"
    focus_cursor(_lua, exec, offset: usize) -> () {
        exec.app.search_focus(offset);
        Ok(())
    }
}

def_method! {
    "Reoffsets all bookmarks based on the difference between a bookmark's and the cursor's offsets"
    reoffset_bookmarks_cursor_diff(_lua, exec, bookmark_name: String) -> () {
        let bookmark = exec.app.meta_state.meta.bookmark_by_name_mut(&bookmark_name).context("No such bookmark").into_lua_err()?;
        let offset = bookmark.offset;
        exec.app.reoffset_bookmarks_cursor_diff(offset);
        Ok(())
    }
}

def_method! {
    "Prints to the lua console"
    log(_lua, exec, value: String) -> () {
        exec.gui.lua_console_window.open.set(true);
        exec.gui.lua_console_window.active_msg_buf = exec.key;
        exec.gui.lua_console_window.msg_buf_for_key(exec.key).push(ConMsg::Plain(value));
        Ok(())
    }
}

def_method! {
    "Prints a clickable offset link to the lua console with an optional text"
    loffset(_lua, exec, offset: usize, text: Option<String>) -> () {
        exec.gui.lua_console_window.open.set(true);
        exec.gui.lua_console_window.active_msg_buf = exec.key;
        exec.gui.lua_console_window.msg_buf_for_key(exec.key).push(ConMsg::OffsetLink { text: text.map_or(offset.to_string(), |text| format!("{offset}: {text}")), offset });
        Ok(())
    }
}

def_method! {
    "Prints a clickable (inclusive) range link to the lua console with an optional text"
    lrange(_lua, exec, start: usize, end: usize, text: Option<String>) -> () {
        exec.gui.lua_console_window.open.set(true);
        exec.gui.lua_console_window.active_msg_buf = exec.key;
        let fmt = move || { format!("{start}..={end}")};
        exec.gui.lua_console_window.msg_buf_for_key(exec.key).push(ConMsg::RangeLink { text: text.map_or_else(fmt, |text| format!("{}: {text}", fmt())), start, end });
        Ok(())
    }
}

def_method! {
    "Returns the start and end offsets of the selection"
    selection(_lua, exec,) -> (usize, usize) {
        exec.app.hex_ui.selection().map(|reg| (reg.begin, reg.end)).context("Selection is empty").into_lua_err()
    }
}

def_method! {
    "Gets a named script as a callable function. `hx:require('myscript')()`"
    require(lua, exec, name: String) -> mlua::Function<'lua> {
        let s = exec.app.meta_state.meta.scripts.values().find(|scr| scr.name == name).ok_or_else(|| "no such script".into_lua_err())?;
        let chunk = lua.load(&s.content);
        chunk.into_function()
    }
}

def_method! {
    "Executes another script with the provided (optional) arguments"
    exec(lua, exec, name: String, args: Option<String>) -> () {
        let args = args.as_deref().unwrap_or("");
        if let Some((key, scr)) = exec.app.meta_state.meta.scripts.iter().find(|(_key, scr)| scr.name == name) {
            let script = scr.content.clone();
            exec_lua(lua, &script, exec.app, exec.gui, exec.font, args, Some(key)).into_lua_err()?;
        }
        Ok(())
    }
}

macro_rules! for_each_method {
    ($m:ident) => {
        $m!(add_region);
        $m!(load_file);
        $m!(bookmark_set_int);
        $m!(region_pattern_fill);
        $m!(find_result_offsets);
        $m!(read_u8);
        $m!(write_u8);
        $m!(read_u32_le);
        $m!(fill_range);
        $m!(set_dirty_region);
        $m!(save);
        $m!(bookmark_offset);
        $m!(add_bookmark);
        $m!(find_hex_string);
        $m!(focus_cursor);
        $m!(reoffset_bookmarks_cursor_diff);
        $m!(log);
        $m!(loffset);
        $m!(lrange);
        $m!(selection);
        $m!(require);
        $m!(exec);
    };
}
pub(super) use for_each_method;

impl<'app, 'gui, 'font> UserData for LuaExecContext<'app, 'gui, 'font> {
    fn add_methods<'lua, T: mlua::UserDataMethods<'lua, Self>>(methods: &mut T) {
        macro_rules! add_method {
            ($t:ty) => {
                methods.add_method_mut(<$t>::NAME, <$t>::call)
            };
        }
        for_each_method!(add_method);
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ExecLuaError {
    #[error("Failed to parse arguments: {0}")]
    ArgParse(#[from] ArgParseError),
    #[error("Failed to execute lua: {0}")]
    Lua(#[from] mlua::prelude::LuaError),
}

pub fn exec_lua(
    lua: &Lua,
    lua_script: &str,
    app: &mut App,
    gui: &mut Gui,
    font: &Font,
    args: &str,
    key: Option<ScriptKey>,
) -> Result<(), ExecLuaError> {
    let args_table = lua.create_table()?;
    if !args.is_empty() {
        let args = parse_script_args(args)?;
        for (k, v) in args.into_iter() {
            match v {
                ScriptArg::String(s) => args_table.set(k, s)?,
                ScriptArg::Num(n) => args_table.set(k, n)?,
            }
        }
    }
    lua.scope(|scope| {
        let chunk = lua.load(lua_script);
        let fun = chunk.into_function()?;
        let app = scope.create_nonstatic_userdata(LuaExecContext {
            app: &mut *app,
            gui,
            font,
            key,
        })?;
        if let Some(env) = fun.environment() {
            env.set("hx", app)?;
            env.set("args", args_table)?;
        }
        fun.call(())?;
        Ok(())
    })?;
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum ScriptArg {
    String(String),
    Num(f64),
}

pub const SCRIPT_ARG_FMT_HELP_STR: &str = "mynum = 4.5, mystring = \"hello\"";

#[derive(thiserror::Error, Debug)]
pub enum ArgParseError {
    #[error("Argument must be of format 'a=b'")]
    ArgNotAEqB,
    #[error("Unterminated string literal")]
    UnterminatedString,
    #[error("Error parsing number: {0}")]
    NumParse(#[from] std::num::ParseFloatError),
    #[error("Missing value after assignment")]
    MissingValue,
}

/// Parse script arguments
pub fn parse_script_args(s: &str) -> Result<HashMap<String, ScriptArg>, ArgParseError> {
    let mut hm = HashMap::new();
    let assignments = s.split(',');
    for assignment in assignments {
        match assignment.split_once('=') {
            Some((lhs, rhs)) => {
                let key = lhs.trim();
                let strval = rhs.trim();
                let Some(first_byte) = strval.bytes().next() else {
                    return Err(ArgParseError::MissingValue);
                };
                if let Some(strval) = strval.strip_prefix(['"', '\'']) {
                    let Some(end) = strval.find(first_byte as char) else {
                        return Err(ArgParseError::UnterminatedString);
                    };
                    hm.insert(
                        key.to_string(),
                        ScriptArg::String(strval[..end].to_string()),
                    );
                } else {
                    let num: f64 = strval.parse()?;
                    hm.insert(key.to_string(), ScriptArg::Num(num));
                }
            }
            None => {
                return Err(ArgParseError::ArgNotAEqB);
            }
        }
    }
    Ok(hm)
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_parse_script_args() {
    let args = parse_script_args(SCRIPT_ARG_FMT_HELP_STR).unwrap();
    assert_eq!(args.get("mynum"), Some(&ScriptArg::Num(4.5)));
    assert_eq!(
        args.get("mystring"),
        Some(&ScriptArg::String("hello".to_string()))
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_parse_script_args_single_quot() {
    let args = parse_script_args(" myval = 'hello world' ").unwrap();
    assert_eq!(
        args.get("myval"),
        Some(&ScriptArg::String("hello world".to_string()))
    );
}
