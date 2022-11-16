# Changelog

## Unreleased

### Added

- About dialog with version info + links
- Clickable file size label in bottom right corner
- Functionality to change the length of the data (truncate/extend)
- Context menus in process open menu to copy addresses/sizes/etc. to clipboard
- Right click context menu option on a view to remove it from the current layout

### Changed

- Open file dialog opens same directory as current file, if available
- Replace most native message boxes with egui ones
- Inspect panel shows value at edit cursor if mouse pointer is over a window that covers the hex view.
- Make path label in top right corner click-to-copy
- Process name filter in process open dialog is now case-insensitive
- "Diff with file" file prompt will now open in same directory as current file
- Don't insert a tab character for text views in edit mode when tab is pressed to switch focus

### Fixed

- Show error message box instead of panic when failing to allocate textures
- Prevent fill dialog and Jump dialog from constantly stealing focus when they are open
- Certain dialog types no longer erroneusly stack on top of themselves if opened multiple times.
- Lua fill dialog with empty selection now has a close button.
- Make regions window scroll properly

## [0.1.0] - 2022-09-16

Initial release.

[0.1.0]: https://github.com/crumblingstatue/hexerator/releases/tag/v0.1.0
