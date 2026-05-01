use std::{collections::HashSet, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

mod color;
mod ffmpeg;
mod theme;

pub use ffmpeg::*;
pub use theme::{Theme, ThemeType};

use crate::state::app_paths;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub tracked_dirs: HashSet<PathBuf>,
    pub ffmpeg_path: Option<PathBuf>,

    pub plugins: HashSet<String>,
    pub color_theme: Option<String>,
}

#[derive(Debug, Deserialize)]
pub enum ListModification {
    Add(String),
    Remove(String),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ConfigField {
    TrackedDirs(ListModification),
    FFMpegPath(String),
    Plugins(ListModification),
    ColorTheme(String),
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

    pub fn mutate_config_field(&mut self, m: ConfigField) {
        use ListModification as M;

        match m {
            ConfigField::TrackedDirs(s) => {
                match s {
                    M::Add(s) => self.tracked_dirs.insert(PathBuf::from(s)),
                    M::Remove(s) => self.tracked_dirs.remove(&PathBuf::from(s)),
                };
            }
            ConfigField::FFMpegPath(s) => {
                if s.is_empty() {
                    self.ffmpeg_path = None;
                }

                self.ffmpeg_path = Some(PathBuf::from(s));
            }
            ConfigField::Plugins(s) => {
                match s {
                    M::Add(s) => self.plugins.insert(s),
                    M::Remove(s) => self.plugins.remove(&s),
                };
            }
            ConfigField::ColorTheme(s) => self.color_theme = Some(s),
        }
    }
}
