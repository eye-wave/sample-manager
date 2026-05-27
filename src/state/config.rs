use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::ser::Error;
use serde::{Deserialize, Serialize};
use struct_patch::Patch;

mod cache;
mod color;
mod theme;

pub use theme::Theme;
use ts_rs::TS;

use crate::schema::{SchemaFieldOptions, SchemaFieldWithValue};
use crate::state::config::theme::list_themes;
use crate::state::{StateError, app_paths};
use crate::{AStr, LogErrorExt};

// --- FFPaths ---

#[derive(Clone, Debug, Default, Serialize, Patch, PartialEq, TS)]
pub struct FFPaths {
    pub ffmpeg_path: Option<PathBuf>,
    pub ffprobe_path: Option<PathBuf>,
}

impl<'de> Deserialize<'de> for FFPaths {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            ffmpeg: Option<PathBuf>,
            ffprobe: Option<PathBuf>,
        }

        let Raw { ffmpeg, ffprobe } = Raw::deserialize(deserializer)?;

        Ok(Self {
            ffmpeg_path: resolve_executable(ffmpeg, "ffmpeg"),
            ffprobe_path: resolve_executable(ffprobe, "ffprobe"),
        })
    }
}

impl FFPaths {
    pub fn flatten(&self) -> Option<FFPathsRef<'_>> {
        Some(FFPathsRef {
            ffmpeg: self.ffmpeg_path.as_ref()?.to_str()?,
            ffprobe: self.ffprobe_path.as_ref()?.to_str()?,
        })
    }
}

#[derive(Clone)]
pub struct FFPathsRef<'a> {
    pub ffmpeg: &'a str,
    pub ffprobe: &'a str,
}

// --- AppConfig ---

pub struct AppConfig {
    inner: ConfigData,
    path: &'static Path,
}

impl AppConfig {
    pub fn load(path: &'static Path) -> Self {
        if let Ok(inner) = fs::read(path)
            .map_err(StateError::from)
            .and_then(|bytes| toml::from_slice::<ConfigData>(&bytes).map_err(StateError::from))
        {
            Self { inner, path }
        } else {
            Self {
                path,
                inner: ConfigData::default(),
            }
        }
    }

    pub fn mutate<R>(&mut self, f: impl FnOnce(&mut ConfigData) -> R) -> R {
        let result = f(&mut self.inner);
        self.flush();
        result
    }

    pub fn patch(&mut self, patch: ConfigDataPatch) {
        self.mutate(|c| c.apply(patch));
    }

    fn flush(&self) {
        if let Ok(contents) = toml::to_string(&self.inner) {
            let _ = fs::write(self.path, contents);
        }
    }
}

impl std::ops::Deref for AppConfig {
    type Target = ConfigData;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for AppConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Patch, TS)]
#[patch(attribute(derive(Clone, Debug, Deserialize)))]
#[serde(default)]
#[ts(export)]
pub struct ConfigData {
    pub tracked_dirs: HashSet<PathBuf>,
    #[serde(flatten)]
    pub ffpaths: FFPaths,
    pub sidebar_width: u16,
    pub plugins: HashSet<String>,
    pub color_theme: Option<String>,
}

impl ConfigData {
    pub fn get_current_theme(&self) -> Option<Theme> {
        self.color_theme.as_deref().and_then(|t| self.get_theme(t))
    }

    pub fn get_theme(&self, theme_name: &str) -> Option<Theme> {
        let path = theme_path(theme_name);
        toml::from_str(&fs::read_to_string(path).sure("Failed to read config")?)
            .sure("Failed to parse config")
    }

    pub fn update_theme(&mut self, theme_name: &str) -> Option<Theme> {
        let theme = self.get_theme(theme_name)?;
        self.color_theme = Some(theme_name.to_string());
        Some(theme)
    }

    pub fn get_field_as_json(&self, field: &str) -> Result<String, serde_json::Error> {
        match field {
            "tracked_dirs" => serde_json::to_string(&self.tracked_dirs),
            "ffmpeg_path" => serde_json::to_string(&self.ffpaths.ffmpeg_path),
            "ffprobe_path" => serde_json::to_string(&self.ffpaths.ffprobe_path),
            "sidebar_width" => serde_json::to_string(&self.sidebar_width),
            "plugins" => serde_json::to_string(&self.plugins),
            "color_theme" => serde_json::to_string(&self.color_theme),
            _ => Err(serde_json::Error::custom(format!(
                "Unknown config field: {field}"
            ))),
        }
    }

    pub fn as_fields(&self) -> HashMap<AStr, SchemaFieldWithValue> {
        HashMap::from([
            (
                "tracked_dirs".into(),
                SchemaFieldWithValue::StringList {
                    label: "Tracked directories".into(),
                    default: Vec::new(),
                    value: self
                        .tracked_dirs
                        .iter()
                        .filter_map(|f| f.to_str())
                        .map(Arc::from)
                        .collect(),
                },
            ),
            (
                "ffmpeg_path".into(),
                SchemaFieldWithValue::String {
                    label: "FFmpeg path".into(),
                    default: "/usr/bin/ffmpeg".into(),
                    is_password: false,
                    value: self
                        .ffpaths
                        .ffmpeg_path
                        .as_deref()
                        .and_then(|p| p.to_str())
                        .unwrap_or_default()
                        .into(),
                },
            ),
            (
                "ffprobe_path".into(),
                SchemaFieldWithValue::String {
                    label: "FFprobe path".into(),
                    default: "/usr/bin/ffprobe".into(),
                    is_password: false,
                    value: self
                        .ffpaths
                        .ffprobe_path
                        .as_deref()
                        .and_then(|p| p.to_str())
                        .unwrap_or_default()
                        .into(),
                },
            ),
            (
                "sidebar_width".into(),
                SchemaFieldWithValue::Number {
                    label: "Sidebar width".into(),
                    default: 270.0,
                    value: self.sidebar_width as f64,
                },
            ),
            (
                "color_theme".into(),
                SchemaFieldWithValue::Select {
                    label: "Color Theme".into(),
                    options: SchemaFieldOptions::Grouped {
                        groups: list_themes().unwrap_or_default(),
                    },
                    default: "valentine".into(),
                    value: self.color_theme.as_deref().unwrap_or_default().into(),
                },
            ),
        ])
    }
}

// --- Helpers ---

fn theme_path(name: &str) -> PathBuf {
    app_paths::themes_path().join(format!("{name}.toml"))
}

fn resolve_executable(path: Option<PathBuf>, name: &str) -> Option<PathBuf> {
    path.or_else(|| which::which(name).sure("exec path not found"))
        .filter(|p| p.is_file() && is_executable(p))
}

fn is_executable(path: &impl AsRef<Path>) -> bool {
    let Ok(metadata) = fs::metadata(path.as_ref()) else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(windows)]
    {
        path.as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e == "exe")
    }
}
