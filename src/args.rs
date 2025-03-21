use {
    crate::parse_radix::parse_guess_radix,
    clap::Parser,
    serde::{Deserialize, Serialize},
    std::path::PathBuf,
};

/// Hexerator: Versatile hex editor
#[derive(Parser, Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct Args {
    /// Arguments relating to the source to open
    #[clap(flatten)]
    pub src: SourceArgs,
    /// Open most recently used file
    #[arg(long)]
    pub recent: bool,
    /// Load this metafile
    #[arg(long, value_name = "path")]
    pub meta: Option<PathBuf>,
    /// Show version information and exit
    #[arg(long)]
    pub version: bool,
    /// Start with debug logging enabled
    #[arg(long)]
    pub debug: bool,
    /// Spawn and open memory of a command with arguments (must be last option)
    #[arg(long, value_name="command", allow_hyphen_values=true, num_args=1..)]
    pub spawn_command: Vec<String>,
    #[arg(long, value_name = "name")]
    /// When spawning a command, open the process list with this filter, rather than selecting a pid
    pub look_for_proc: Option<String>,
    /// Automatically reload the source for the current buffer in millisecond intervals (default:250)
    #[arg(long, value_name="interval", default_missing_value="250", num_args=0..=1)]
    pub autoreload: Option<u32>,
    /// Only autoreload the data visible in the current layout
    #[arg(long)]
    pub autoreload_only_visible: bool,
    /// Automatically save if there is an edited region in the file
    #[arg(long)]
    pub autosave: bool,
    /// Open this layout on startup instead of the default
    #[arg(long, value_name = "name")]
    pub layout: Option<String>,
    /// Focus the first instance of this view on startup
    #[arg(long, value_name = "name")]
    pub view: Option<String>,
    #[arg(long)]
    /// Load a dynamic library plugin at startup
    pub load_plugin: Vec<PathBuf>,
    /// Allocate a new (zero-filled) buffer. Also creates the provided file argument if it doesn't exist.
    #[arg(long, value_name = "length")]
    pub new: Option<usize>,
}

/// Arguments for opening a source (file/stream/process/etc)
#[derive(Parser, Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct SourceArgs {
    /// The file to read
    pub file: Option<PathBuf>,
    /// Jump to offset on startup
    #[arg(short = 'j', long="jump", value_name="offset", value_parser = parse_guess_radix::<usize>)]
    pub jump: Option<usize>,
    /// Seek to offset, consider it beginning of the file in the editor
    #[arg(long, value_name="offset", value_parser = parse_guess_radix::<usize>)]
    pub hard_seek: Option<usize>,
    /// Read only this many bytes
    #[arg(long, value_name = "bytes", value_parser = parse_guess_radix::<usize>)]
    pub take: Option<usize>,
    /// Open file as read-only, without writing privileges
    #[arg(long)]
    pub read_only: bool,
    /// Specify source as a streaming source (for example, standard streams).
    /// Sets read-only attribute.
    #[arg(long)]
    pub stream: bool,
    /// The buffer size in bytes to use for reading when streaming
    #[arg(long)]
    #[serde(default)]
    pub stream_buffer_size: Option<usize>,
    /// Try to open the source using mmap rather than load into a buffer
    #[serde(default)]
    #[arg(long, value_name = "mode")]
    pub unsafe_mmap: Option<MmapMode>,
    /// Assume the memory mapped file is of this length (might be needed for looking at block devices, etc.)
    #[serde(default)]
    #[arg(long, value_name = "len")]
    pub mmap_len: Option<usize>,
}

/// How the memory mapping should operate
#[derive(
    Clone,
    Copy,
    clap::ValueEnum,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::IntoStaticStr,
    strum::EnumIter,
    Default,
)]
pub enum MmapMode {
    /// Read-only memory map.
    /// Note: Some features may not work, as Hexerator was designed for a mutable data buffer.
    #[default]
    Ro,
    /// Copy-on-write memory map.
    /// Changes are only visible locally.
    Cow,
    /// Mutable memory map.
    /// *WARNING*: Any edits will immediately take effect. THEY CANNOT BE UNDONE.
    DangerousMut,
}
