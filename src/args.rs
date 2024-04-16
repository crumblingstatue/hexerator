use {
    crate::parse_radix::parse_guess_radix,
    clap::Parser,
    serde::{Deserialize, Serialize},
    std::path::PathBuf,
};

/// Arguments given to hexerator on startup
#[derive(Parser, Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Args {
    /// Arguments relating to the source to open
    #[clap(flatten)]
    pub src: SourceArgs,
    /// Open most recently used file
    #[arg(long)]
    pub recent: bool,
    /// Load this metafile
    #[arg(long)]
    pub meta: Option<PathBuf>,
    /// Show version information and exit
    #[arg(long)]
    pub version: bool,
    /// Start with debug logging enabled
    #[arg(long)]
    pub debug: bool,
    /// Spawn and open memory of a command with arguments (must be last option)
    #[arg(long, allow_hyphen_values=true, num_args=1..)]
    pub spawn_command: Vec<String>,
}

/// Arguments for opening a source (file/stream/process/etc)
#[derive(Parser, Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct SourceArgs {
    /// The file to read
    pub file: Option<PathBuf>,
    /// Jump to offset on startup
    #[arg(short = 'j', value_parser = parse_guess_radix::<usize>)]
    pub jump: Option<usize>,
    /// Seek to offset, consider it beginning of the file in the editor
    #[arg(long, value_parser = parse_guess_radix::<usize>)]
    pub hard_seek: Option<usize>,
    /// Read only this many bytes
    #[arg(long, value_parser = parse_guess_radix::<usize>)]
    pub take: Option<usize>,
    /// Open file as read-only, without writing privileges
    #[arg(long)]
    pub read_only: bool,
    #[arg(long)]
    /// Specify source as a streaming source (for example, standard streams).
    /// Sets read-only attribute.
    pub stream: bool,
}
