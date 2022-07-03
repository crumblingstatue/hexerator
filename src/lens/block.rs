use gamedebug_core::imm_msg;
use sfml::graphics::{RenderTarget, RenderWindow, Sprite, Texture, Transformable, Vertex};

use crate::{
    app::{edit_target::EditTarget, interact_mode::InteractMode, App},
    lens::draw_cursor,
};

use super::Lens;

pub fn block(
    lens: &Lens,
    app: &mut App,
    window: &mut RenderWindow,
    vertex_buffer: &mut Vec<Vertex>,
) {
}
