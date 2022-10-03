# Changelog

## Unreleased

### Added

- About dialog with version info + links
- Clickable file size label in bottom right corner

### Changed

- Open file dialog opens same directory as current file, if available
- Replace most native message boxes with egui ones
- Inspect panel shows value at edit cursor if mouse pointer is over a window that covers the hex view.
- Make path label in top right corner click-to-copy

### Fixed

- Show error message box instead of panic when failing to allocate textures
- Prevent fill dialog and Jump dialog from constantly stealing focus when they are open
- Certain dialog types no longer erroneusly stack on top of themselves if opened multiple times.
- Lua fill dialog with empty selection now has a close button.
- Make regions window scroll properly

## [0.1.0] - 2022-09-16

Initial release.

[0.1.0]: https://github.com/crumblingstatue/hexerator/releases/tag/v0.1.0
