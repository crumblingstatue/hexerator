use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    /// The file to read
    pub file: Option<PathBuf>,
    /// Jump to offset on startup
    #[clap(short = 'j')]
    pub jump: Option<usize>,
    /// Seek to offset, consider it beginning of the file in the editor
    #[clap(long)]
    pub hard_seek: Option<u64>,
    /// Read only this many bytes
    #[clap(long)]
    pub take: Option<u64>,
    /// Open file as read-only, without writing privileges
    #[clap(long)]
    pub read_only: bool,
    #[clap(long)]
    /// Specify source as a streaming source (for example, standard streams).
    /// Sets read-only attribute.
    pub stream: bool,
}
