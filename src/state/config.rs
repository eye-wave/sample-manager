use std::{collections::HashSet, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

mod color;
mod theme;

pub use theme::{Theme, ThemeType};

use crate::state::app_paths;

#[derive(Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub tracked_dirs: HashSet<PathBuf>,
    pub color_theme: Option<String>,
}

impl AppConfig {
    pub fn get_current_theme(&self) -> Option<Theme> {
        let theme_path = app_paths::themes_path().join(self.color_theme.as_ref()?);
        let file = fs::read_to_string(theme_path).ok()?;

        toml::from_str(&file).ok()
    }

    pub fn get_theme(&self, theme_name: &str) -> Option<Theme> {
        let theme_path = app_paths::themes_path().join(theme_name);
        let file = fs::read_to_string(theme_path).ok()?;

        toml::from_str(&file).ok()
    }

    pub fn update_theme(&mut self, theme_name: &str) -> Option<Theme> {
        let theme_path = app_paths::themes_path().join(theme_name);
        let file = fs::read_to_string(theme_path).ok()?;

        toml::from_str(&file).ok().inspect(|_| {
            self.color_theme = Some(theme_name.to_string());
        })
    }
}
