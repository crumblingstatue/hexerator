use {
    crate::args::SourceArgs,
    anyhow::Context,
    directories::ProjectDirs,
    recently_used_list::RecentlyUsedList,
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub recent: RecentlyUsedList<SourceArgs>,
    pub style: Style,
}

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
            heading: 20,
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
        }
    }
}

impl Config {
    pub fn load_or_default() -> anyhow::Result<Self> {
        let proj_dirs = project_dirs().context("Failed to get project dirs")?;
        let cfg_dir = proj_dirs.config_dir();
        if !cfg_dir.exists() {
            std::fs::create_dir_all(cfg_dir)?;
        }
        let cfg_file = cfg_dir.join(FILENAME);
        if !cfg_file.exists() {
            Ok(Config::default())
        } else {
            let result: anyhow::Result<Self> = try {
                let cfg_bytes = std::fs::read(cfg_file)?;
                rmp_serde::from_slice(&cfg_bytes)?
            };
            match result {
                Ok(cfg) => Ok(cfg),
                Err(e) => if rfd::MessageDialog::new().set_buttons(
                    rfd::MessageButtons::OkCancelCustom("Overwrite".into(), "Quit".into()),
                ).set_description(&format!("Failed to load config: {:?}\n Create a new default config and overwrite, or quit?", e)).show() {
                    Ok(Config::default())
                } else {
                    anyhow::bail!("Couldn't create config");
                },
            }
        }
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let bytes = rmp_serde::to_vec(self)?;
        let proj_dirs = project_dirs().context("Failed to get project dirs")?;
        let cfg_dir = proj_dirs.config_dir();
        std::fs::write(cfg_dir.join(FILENAME), &bytes)?;
        Ok(())
    }
}

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "crumblingstatue", "hexerator")
}

const FILENAME: &str = "hexerator.cfg";
