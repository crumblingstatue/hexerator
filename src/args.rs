use {
    crate::parse_radix::parse_guess_radix,
    clap::Parser,
    serde::{Deserialize, Serialize},
    std::path::PathBuf,
};

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
}

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
