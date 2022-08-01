use anyhow::Context;
use directories::ProjectDirs;
use recently_used_list::RecentlyUsedList;
use serde::{Deserialize, Serialize};

use crate::args::Args;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    pub recent: RecentlyUsedList<Args>,
}

impl Config {
    pub fn load_or_default() -> anyhow::Result<Self> {
        let proj_dirs = project_dirs().context("Failed to get project dirs")?;
        let cfg_dir = proj_dirs.config_dir();
        if !cfg_dir.exists() {
            std::fs::create_dir_all(&cfg_dir)?;
        }
        let cfg_file = cfg_dir.join(FILENAME);
        if cfg_file.exists() {
            let result: anyhow::Result<Self> = try {
                let cfg_bytes = std::fs::read(cfg_file)?;
                rmp_serde::from_slice(&cfg_bytes)?
            };
            match result {
                Ok(cfg) => Ok(cfg),
                Err(e) => if rfd::MessageDialog::new().set_buttons(
                    rfd::MessageButtons::OkCancelCustom("Overwrite".into(), "Quit".into()),
                ).set_description(&format!("Failed to load config: {:?}\n Create a new default config and overwrite, or quit?", e)).show() {
                    Ok(Self::default())
                } else {
                    anyhow::bail!("Couldn't create config");
                },
            }
        } else {
            Ok(Self::default())
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
