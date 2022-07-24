use std::path::PathBuf;

use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Args {
    /// The file to read
    pub file: Option<PathBuf>,
    /// Jump to offset on startup
    #[clap(short = 'j')]
    pub jump: Option<usize>,
    /// Seek to offset, consider it beginning of the file in the editor
    #[clap(long)]
    pub hard_seek: Option<usize>,
    /// Read only this many bytes
    #[clap(long)]
    pub take: Option<usize>,
    /// Open file as read-only, without writing privileges
    #[clap(long)]
    pub read_only: bool,
    #[clap(long)]
    /// Specify source as a streaming source (for example, standard streams).
    /// Sets read-only attribute.
    pub stream: bool,
    /// Open content in existing instance
    #[clap(long)]
    pub instance: bool,
    /// Load most recently used file
    #[clap(long)]
    pub load_recent: bool,
}
