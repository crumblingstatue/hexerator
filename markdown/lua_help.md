## Methods

`hx:add_region(name, start, end)`

Add a region

`hx:load_file(path)`

Load a file

`hx:bookmark_set_int(name, value)`

Set a bookmark

`hx:region_pattern_fill(name, pattern)`

Pattern fill a region

`hx:find_result_offsets()`

Get access to the find result dialog offsets

`hx:read_u8(offset)`

Read an unsigned 8 bit integer

`hx:read_u32_le(offset)`

Read an unsigned 32 bit integer

`hx:fill_range(start, end, value)`

Fill a range with a value

`hx:set_dirty_region(start, end)`

Set the dirty region of the opened document

`hx:save()`

Save the current document (all the dirty regions).

`hx:bookmark_offset(name)`

Get the offset of a bookmark.

`hx:add_bookmark(offset, name)`

Add a bookmark
