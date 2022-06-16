use std::{
    fs::File,
    io::{Read, Stdin},
};

#[derive(Debug)]
pub enum Source {
    File(File),
    Stdin(Stdin),
}

impl Read for Source {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Source::File(f) => f.read(buf),
            Source::Stdin(stdin) => stdin.read(buf),
        }
    }
}
