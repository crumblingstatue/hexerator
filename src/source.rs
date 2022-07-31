use std::{
    fs::File,
    io::{Read, Stdin},
};

#[derive(Debug)]
pub enum Source {
    File(File),
    Stdin(Stdin),
}

impl Clone for Source {
    #[expect(
        clippy::unwrap_used,
        reason = "Can't really do much else in clone impl"
    )]
    fn clone(&self) -> Self {
        match self {
            Self::File(file) => Self::File(file.try_clone().unwrap()),
            Self::Stdin(_) => Self::Stdin(std::io::stdin()),
        }
    }
}

impl Read for Source {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::File(f) => f.read(buf),
            Self::Stdin(stdin) => stdin.read(buf),
        }
    }
}
