use gamedebug_core::imm_msg;
use sfml::graphics::{Font, Vertex};

use crate::{
    app::{edit_target::EditTarget, interact_mode::InteractMode, App},
    lens::{draw_cursor, draw_glyph},
};

use super::Lens;

pub fn ascii(lens: &Lens, app: &mut App, font: &Font, vertex_buffer: &mut Vec<Vertex>) {}
