use std::path::PathBuf;

use directories::ProjectDirs;
use recently_used_list::RecentlyUsedList;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    pub recent: RecentlyUsedList<PathBuf>,
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
            let cfg_bytes = std::fs::read(cfg_file).unwrap();
            rmp_serde::from_slice(&cfg_bytes).unwrap()
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
