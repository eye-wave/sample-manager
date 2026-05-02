use std::{collections::HashSet, fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use struct_patch::Patch;

mod color;
mod ffmpeg;
mod theme;

pub use ffmpeg::*;
pub use theme::{Theme, ThemeType};
use ts_rs::TS;

use crate::state::app_paths;

#[derive(Debug, Default, Serialize, Deserialize, Patch, TS)]
#[patch(attribute(derive(Debug, Deserialize)))]
#[ts(export)]
pub struct AppConfig {
    pub tracked_dirs: HashSet<PathBuf>,
    pub ffmpeg_path: Option<PathBuf>,
    #[serde(default)]
    pub sidebar_width: u16,

    pub plugins: HashSet<String>,
    pub color_theme: Option<String>,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfigField {
    TrackedDirs,
    FfmpegPath,
    SidebarWidth,
    Plugins,
    ColorTheme,
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

    pub fn get_field_as_json(&self, field: ConfigField) -> Result<String, serde_json::Error> {
        match field {
            ConfigField::TrackedDirs => serde_json::to_string(&self.tracked_dirs),
            ConfigField::FfmpegPath => serde_json::to_string(&self.ffmpeg_path),
            ConfigField::SidebarWidth => serde_json::to_string(&self.sidebar_width),
            ConfigField::Plugins => serde_json::to_string(&self.plugins),
            ConfigField::ColorTheme => serde_json::to_string(&self.color_theme),
        }
    }

    pub fn mutate_config_field(&mut self, patch: AppConfigPatch) {
        self.apply(patch);
    }
}
