Legend:
- [ ] = Unimplemented
- [x] = Implemented

# Features
- [x] [Colorized values](#colorized-values)
  - [x] [Custom color palettes](#custom-color-palettes)
- [x] [Easy data alignment](#easy-data-alignment)
- [x] Suitable for [process memory editing](#process-memory-editing)
- [x] [Multiple source types](#multiple-source-types) (file/streamed sources)
- [x] [Rich command line options](commandline.md)
- [ ] [Bookmarks](#bookmarks)
- [ ] [Multiple configurable views](#multiple-configurable-views)
- [ ] [Huge file support through memory mapped files](#huge-file-support-through-memory-mapped-files)

# Non-features
- [Insertion](#insertion)
- [Memory holes support, generic support for huge data](#memory-holes-support-generic-support-for-huge-data)

# Features

## Colorized values
Colorizing values helps a lot with human pattern recognition.
![Colorized values](screenshots/color.png)

### Custom color palettes
Custom color palettes can be saved and loaded, and generated through various means.
![Custom palettes](screenshots/custom-palette.png)

## Easy data alignment
Hexerator considers it important to easily align data with shortcut keys. Proper alignment can make a lot of difference
with pattern recognition.

You can see a YouTube video of it in action here:

[![Video](https://img.youtube.com/vi/GhPh_y1PjTU/0.jpg)](https://www.youtube.com/watch?v=GhPh_y1PjTU)

## Process Memory editing
Hexerator is able to be used for viewing and editing process memory.
It also only saves regions that have been edited, to prevent
old memory from being saved over new updated memory.

You can see a YouTube video of it in action here:

[![Video](https://img.youtube.com/vi/W8ab3-Hp-f0/0.jpg)](https://www.youtube.com/watch?v=W8ab3-Hp-f0)

## Multiple source types
Hexerator supports opening both files and streamed sources like standard input or character devices like `/dev/urandom`.


## Bookmarks

Quickly and easily save and access regions of interest in the file.
To be implemented.

## Multiple configurable views

Each bookmarked region can have multiple views of different configuration, like different column size.
To be implemented.

## Huge file support through memory mapped files

Huge files that couldn't fit in memory can be opened as memory mapped files through a command line
option.
To be implemented.

# Non-features

## Insertion
Insertion would complicate implementation, and for most binary data, including process memory, it will just mess up the data.

## Memory holes support, generic support for huge data
Originally, I wanted to have a generic mechanism for loading only parts of files, but I found that
it would make the implementation way more complex, and possibly inefficient, so I dropped the idea.
Huge files will be eventually supported through memory mapped file support.