use {
    crate::args::SourceArgs,
    anyhow::Context as _,
    directories::ProjectDirs,
    egui_fontcfg::CustomFontPaths,
    recently_used_list::RecentlyUsedList,
    serde::{Deserialize, Serialize},
    std::{
        collections::{BTreeMap, HashMap},
        path::PathBuf,
    },
};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub recent: RecentlyUsedList<SourceArgs>,
    pub style: Style,
    /// filepath->meta associations
    #[serde(default)]
    pub meta_assocs: MetaAssocs,
    #[serde(default = "default_vsync")]
    pub vsync: bool,
    #[serde(default)]
    pub fps_limit: u32,
    #[serde(default)]
    pub pinned_dirs: Vec<PathBuf>,
    #[serde(default)]
    pub custom_font_paths: CustomFontPaths,
    #[serde(default)]
    pub font_families: BTreeMap<egui::FontFamily, Vec<String>>,
}

const fn default_vsync() -> bool {
    true
}

pub type MetaAssocs = HashMap<PathBuf, PathBuf>;

#[derive(Serialize, Deserialize, Default)]
pub struct Style {
    pub font_sizes: FontSizes,
}

#[derive(Serialize, Deserialize)]
pub struct FontSizes {
    pub heading: u8,
    pub body: u8,
    pub monospace: u8,
    pub button: u8,
    pub small: u8,
}

impl Default for FontSizes {
    fn default() -> Self {
        Self {
            small: 10,
            body: 14,
            button: 14,
            heading: 16,
            monospace: 14,
        }
    }
}

const DEFAULT_RECENT_CAPACITY: usize = 16;

impl Default for Config {
    fn default() -> Self {
        let mut recent = RecentlyUsedList::default();
        recent.set_capacity(DEFAULT_RECENT_CAPACITY);
        Self {
            recent,
            style: Style::default(),
            meta_assocs: HashMap::default(),
            fps_limit: 0,
            vsync: default_vsync(),
            pinned_dirs: Vec::new(),
            custom_font_paths: Default::default(),
            font_families: Default::default(),
        }
    }
}

pub struct LoadedConfig {
    pub config: Config,
    /// If `Some`, saving this config file will overwrite an old one that couldn't be loaded
    pub old_config_err: Option<anyhow::Error>,
}

impl Config {
    pub fn load_or_default() -> anyhow::Result<LoadedConfig> {
        let proj_dirs = project_dirs().context("Failed to get project dirs")?;
        let cfg_dir = proj_dirs.config_dir();
        if !cfg_dir.exists() {
            std::fs::create_dir_all(cfg_dir)?;
        }
        let cfg_file = cfg_dir.join(FILENAME);
        if !cfg_file.exists() {
            Ok(LoadedConfig {
                config: Config::default(),
                old_config_err: None,
            })
        } else {
            let result: anyhow::Result<Self> = try {
                let cfg_bytes = std::fs::read(cfg_file)?;
                rmp_serde::from_slice(&cfg_bytes)?
            };
            match result {
                Ok(cfg) => Ok(LoadedConfig {
                    config: cfg,
                    old_config_err: None,
                }),
                Err(e) => Ok(LoadedConfig {
                    config: Config::default(),
                    old_config_err: Some(e),
                }),
            }
        }
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(self)?;
        let proj_dirs = project_dirs().context("Failed to get project dirs")?;
        let cfg_dir = proj_dirs.config_dir();
        std::fs::write(cfg_dir.join(FILENAME), bytes)?;
        Ok(())
    }
}

pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "crumblingstatue", "hexerator")
}

pub trait ProjectDirsExt {
    fn color_theme_path(&self) -> PathBuf;
}

impl ProjectDirsExt for ProjectDirs {
    fn color_theme_path(&self) -> PathBuf {
        self.config_dir().join("egui_colors_theme.pal")
    }
}

const FILENAME: &str = "hexerator.cfg";
