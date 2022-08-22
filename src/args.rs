use std::{path::PathBuf, str::FromStr};

use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Args {
    /// The file to read
    pub file: Option<PathBuf>,
    /// Jump to offset on startup
    #[clap(short = 'j', value_parser = parse_offset)]
    pub jump: Option<usize>,
    /// Seek to offset, consider it beginning of the file in the editor
    #[clap(long, value_parser = parse_offset)]
    pub hard_seek: Option<usize>,
    /// Read only this many bytes
    #[clap(long, value_parser = parse_offset)]
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

fn parse_offset(arg: &str) -> Result<usize, <usize as FromStr>::Err> {
    if let Some(stripped) = arg.strip_prefix("0x") {
        usize::from_str_radix(stripped, 16)
    } else if arg.contains(['a', 'b', 'c', 'd', 'e', 'f']) {
        usize::from_str_radix(arg, 16)
    } else {
        arg.parse()
    }
}
