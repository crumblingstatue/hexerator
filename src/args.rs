use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    /// The file to read
    pub file: PathBuf,
    /// Jump to offset on startup
    #[clap(short = 'j')]
    pub jump: Option<usize>,
}
