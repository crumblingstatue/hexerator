/// User interaction mode
///
/// There are 2 modes: View and Edit
#[derive(PartialEq, Eq, Debug)]
pub enum InteractMode {
    /// Mode optimized for viewing the contents
    ///
    /// For example arrow keys scroll the content
    View,
    /// Mode optimized for editing the contents
    ///
    /// For example arrow keys move the cursor
    Edit,
}
