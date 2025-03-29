# Changelog

## [0.4.0] - TBD

### New features

- [Memory mapped file support][mmap]
- [Allow defining data layouts with Rust struct syntax][struct]
  - `View->Ruler` can use struct definitions
- [Mouse drag selection][mdrag]
  - You can finally select regions by dragging the mouse, rather than having to use shift+1/shift+2
- [Block selection with alt+drag][mblock]
  - You can select non-contiguous sections by holding alt and drawing a rectangle with the mouse

[mmap]: <https://crumblingstatue.github.io/hexerator-book/0.4.0/feature-docs/mmap.html>
[struct]: <https://crumblingstatue.github.io/hexerator-book/0.4.0/feature-docs/structs.html>
[mdrag]: <https://crumblingstatue.github.io/hexerator-book/0.4.0/basic-ops/selecting-data.html#mouse-drag-selection>
[mblock]: <https://crumblingstatue.github.io/hexerator-book/0.4.0/basic-ops/selecting-data.html#mouse-block-multi-selection>

### UI changes

- Add custom right panel to file open dialog
  - Shows information about the highlighted file
  - Allows selecting advanced options
- Backtrace support for error popups
- External command window now provides more options for working directory
- Show information about rows/column positions in more places
- `Home`/`End` now jumps to row begin/end.
  - `ctrl+Home`/`ctrl+End` are now used for view begin/end.
- The selection can now be quickly cleared with a `Clear` button in the top panel
- Add a "quick scroll" slider popup to the bottom panel, to quickly navigate huge files.
- Add Find&Replace for `HexString` find type
- Add a bunch of icons to buttons
- Remove superfluous "Perspectives" menu

### Other Improvements

- Make stream buffer size configurable, use a larger default size
- Hexerator now retries opening a file as read-only if there was a permission error
- Hex strings now accept parsing comma separated, or "packed" (unseparated) hex values
- The command line help on Windows is now functional
- Increase/decrease byte (`ctrl+=`/`ctrl+-`) now works on selections
- Add Windows CI
- Bunch of bug fixes and minor UX improvements, as usual

### CLI
Add `--view` flag to select view to focus on startup

## [0.3.0] - 2024-10-16

### UI changes

**Hex Editor:**

- `Del` key zeroes out the byte at cursor

**Bookmarks window:**

- Jump-to button in detail view
- Value edit input in detail view
- Context menu option to copy a bookmark's offset
- Add right click menu option to reoffset all bookmarks based on a known offset (read help label)

**File diff window:**

- Now takes the value types of bookmarks into account, showing the whole values of
  bookmarks instead of just raw bytes.
- Add "Highlight all" button to highlight all differences
- Add "Open this" and "Diff with..." buttons to speed up diffing
  subsequent versions of a file

**Find dialog:**

- Add help hover popups for the find type dropdown
- Add "string diff" and "pattern equivalence" find types. See the help popups ;)
- Add basic replace functionality to Ascii find

**X86 assembly dialog:**

- Add ability to jump to offset of decoded instructions

**Root context menu:**

- Add "copy selection as utf-8 text"
- Add "zero fill" (Shortcut: `Del`)

**External command window:**

- Now openable with `Ctrl+E`
- Allow closing with `Esc` key
- Add "selection only" toggle to only pass selection to external command

**Open process window:**
- Add UI to launch a child process in order to view its memory (hexerator doesn't have to be root)
- The virtual memory map window now makes it more clear that you're no longer
  looking at the list of processes, but the maps for a process.

**Jump dialog:**

- Replace (broken) "relative" option with "absolute"

**Preferences window:**

- Make the ui tabbed
- Small ui improvements

### Lua scripting

- Replaced LuaJIT with Lua 5.4, because LuaJIT is incompatible with `panic=abort`.
- Add Lua syntax highlighting in most places
- Add Lua API help window (`Scripting - Lua help`)
- Add a bunch more API items (see `Scripting -> Lua help`)
- Allow saving named scripts, and add script manager window to overview them
- Add Lua console window for quick evaluation and "watching" expressions
- Scripts can now take arguments (`args` table, e.g. `args.foo`)

### Plugins

New feature. Allow loading dylib plugins. Documentation to be added.
For now, see the `hexerator_plugin_api` crate inside the repo.

### Command line

- Add `--version` flag
- Add `--debug` flag to start with debug logging enabled and debug window open
- Add `--spawn-command <command>...` flag to spawn a child process and open it in process list (hexerator doesn't have to be root)
- Add `--autosave` and `--autoreload [<interval>]` to enable autosave/autoreaload through CLI
- Add `--layout <name>` to switch to a layout at startup
- Add `--new <length>` option to create a new (zero-filled) buffer

### Fixes

- Loading process memory on windows now correctly sets relative offset
- When failing to load a file via command line arg, error reason is now properly displayed

### Other

- `Analysis -> Zero partition` for "zero-partitioning" files that contain large zeroed out sections (like process memory).
- Add feature to autoreload only visible part (as opposed to whole file)
- Replace blocking file dialog with nonblocking egui file dialog
- Update egui to 0.29
- Experimental support for custom color themes (See `Preferences` -> `Style`)
- Make monochrome and "grayscale" hex text colors customizable
- No more dynamic dependency on SFML. It's statically linked now.
- Various bug fixes and minor improvements, too many to list individually

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
[0.2.0]: https://github.com/crumblingstatue/hexerator/releases/tag/v0.2.0
[0.3.0]: https://github.com/crumblingstatue/hexerator/releases/tag/v0.3.0
