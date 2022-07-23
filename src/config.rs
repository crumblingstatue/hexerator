use directories::ProjectDirs;
use recently_used_list::RecentlyUsedList;
use serde::{Deserialize, Serialize};

use crate::args::Args;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    pub recent: RecentlyUsedList<Args>,
}

impl Config {
    pub fn load_or_default() -> Self {
        let proj_dirs = project_dirs();
        let cfg_dir = proj_dirs.config_dir();
        if !cfg_dir.exists() {
            std::fs::create_dir_all(&cfg_dir).unwrap();
        }
        let cfg_file = cfg_dir.join(FILENAME);
        if !cfg_file.exists() {
            Config::default()
        } else {
            let result: anyhow::Result<_> = try {
                let cfg_bytes = std::fs::read(cfg_file)?;
                rmp_serde::from_slice(&cfg_bytes)?
            };
            match result {
                Ok(cfg) => cfg,
                Err(e) => if rfd::MessageDialog::new().set_buttons(
                    rfd::MessageButtons::OkCancelCustom("Overwrite".into(), "Quit".into()),
                ).set_description(&format!("Failed to load config: {:?}\n Create a new default config and overwrite, or quit?", e)).show() {
                    Config::default()
                } else {
                    panic!("Couldn't create config");
                },
            }
        }
    }
    pub fn save(&self) {
        let bytes = rmp_serde::to_vec(self).unwrap();
        let proj_dirs = project_dirs();
        let cfg_dir = proj_dirs.config_dir();
        std::fs::write(cfg_dir.join(FILENAME), &bytes).unwrap();
    }
}

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("", "crumblingstatue", "hexerator").unwrap()
}

const FILENAME: &str = "hexerator.cfg";
