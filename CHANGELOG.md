# Changelog

## Unreleased

### Added
- Context menu option to copy a bookmark's offset
- Jump-to button in detail view for bookmarks
- Value edit input in detail view for bookmarks
- Update egui to 0.21


### Changed

- The virtual memory map window now makes it more clear that you're no longer
  looking at the list of processes, but the maps for a process.

### Fixed

- Loading process memory on windows now correctly sets relative offset

## [0.2.0] - 2023-01-27

### Added

- Support for common value types in find dialog, in addition to u8
- About dialog with version info + links
- Clickable file size label in bottom right corner
- Functionality to change the length of the data (truncate/extend)
- Context menus in process open menu to copy addresses/sizes/etc. to clipboard
- Right click context menu option on a view to remove it from the current layout
- Layout properties is accessible from right click context menu on the layout
- Error reporting message dialog if the program panics
- Each file can set a metafile association to always load that meta when loaded
- Vsync and fps limit settings in preferences window
- Bookmark names are displayed when mouse hovers over a bookmarked offset
- "Open bookmark" context menu option in hex view for existing bookmarks
- "Save as" action
- Hex string search in find dialog (de ad be ef)
- Window title now includes filename of opened file
- Ability to save/load scripts in lua execute dialog
- `app:bookmark_set_int(name, value)` lua method to set integer value of a bookmark
- `app:region_pattern_fill(name, pattern)` lua method to fill a region
- Context menu to copy bookmark names in bookmarks window
- Make the offsets in the find dialog copiable/pasteable
- Add x86 disassembly

### Changed

- Update to egui 0.20
- Open file dialog opens same directory as current file, if available
- Replace most native message boxes with egui ones
- Inspect panel shows value at edit cursor if mouse pointer is over a window that covers the hex view.
- Make path label in top right corner click-to-copy
- Process name filter in process open dialog is now case-insensitive
- "Diff with file" file prompt will now open in same directory as current file
- Don't insert a tab character for text views in edit mode when tab is pressed to switch focus
- Active selection actions in edit menu are now in a submenu named "Selection"
- "Copy as hex" is now known as "Copy as hex text"
- Bookmarks table is now resizable horizontally
- Bookmarks table is now scrollable vertically
- Native dialog boxes now have a title, and their text is selectable and copyable!
- Bookmarks window name filter is now case insensitive
- Bookmarks window description editor is now monospace
- Bookmark description is now in a scroll area
- Bookmarks window "add new at cursor" button selects newly added bookmark automatically
- Create default metadata for empty documents, allowing creation of binary files from scratch with Hexerator
- File path label has context menu for various options, left clicking opens the file in default application

### Fixed

- Show error message box instead of panic when failing to allocate textures
- Prevent fill dialog and Jump dialog from constantly stealing focus when they are open
- Certain dialog types no longer erroneusly stack on top of themselves if opened multiple times.
- Lua fill dialog with empty selection now has a close button.
- Make regions window scroll properly
- Pattern fill dialog is now closeable
- "Select all" action now doesn't select more data than is available, even if region is bigger than data.

## [0.1.0] - 2022-09-16

Initial release.

[0.1.0]: https://github.com/crumblingstatue/hexerator/releases/tag/v0.1.0
