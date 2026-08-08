use std::{
    fs::File,
    io::{Read, Stdin},
};

#[derive(Debug)]
pub enum SourceProvider {
    File(File),
    Stdin(Stdin),
}

#[derive(Debug)]
pub struct Source {
    pub provider: SourceProvider,
    pub attr: SourceAttributes,
    pub state: SourceState,
}

impl Source {
    pub fn file(f: File) -> Self {
        Self {
            provider: SourceProvider::File(f),
            attr: SourceAttributes {
                stream: false,
                permissions: SourcePermissions { write: true },
            },
            state: SourceState::default(),
        }
    }
}

#[derive(Debug)]
pub struct SourceAttributes {
    /// Whether reading should be done by streaming
    pub stream: bool,
    pub permissions: SourcePermissions,
}

#[derive(Debug, Default)]
pub struct SourceState {
    /// Whether streaming has finished
    pub stream_end: bool,
}

#[derive(Debug)]
pub struct SourcePermissions {
    pub write: bool,
}

impl Clone for SourceProvider {
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

impl Read for SourceProvider {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::File(f) => f.read(buf),
            Self::Stdin(stdin) => stdin.read(buf),
        }
    }
}
