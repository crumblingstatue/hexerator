#[derive(Default, Debug)]
pub struct EditState {
    // The editing byte offset
    pub cursor: usize,
    // The half digit when the user begins to type into a hex view
    pub hex_edit_half_digit: Option<u8>,
}
