use std::path::PathBuf;

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::parse_radix::parse_guess_radix;

#[derive(Parser, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Args {
    /// Arguments relating to the source to open
    #[clap(flatten)]
    pub src: SourceArgs,
    /// Open most recently used file
    #[clap(long)]
    pub recent: bool,
    /// Load this metafile
    #[clap(long)]
    pub meta: Option<PathBuf>,
}

#[derive(Parser, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct SourceArgs {
    /// The file to read
    pub file: Option<PathBuf>,
    /// Jump to offset on startup
    #[clap(short = 'j', value_parser = parse_guess_radix::<usize>)]
    pub jump: Option<usize>,
    /// Seek to offset, consider it beginning of the file in the editor
    #[clap(long, value_parser = parse_guess_radix::<usize>)]
    pub hard_seek: Option<usize>,
    /// Read only this many bytes
    #[clap(long, value_parser = parse_guess_radix::<usize>)]
    pub take: Option<usize>,
    /// Open file as read-only, without writing privileges
    #[clap(long)]
    pub read_only: bool,
    #[clap(long)]
    /// Specify source as a streaming source (for example, standard streams).
    /// Sets read-only attribute.
    pub stream: bool,
}
