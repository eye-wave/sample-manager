use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use struct_patch::Patch;

mod color;
mod theme;

pub use theme::{Theme, ThemeType};
use ts_rs::TS;

use crate::state::app_paths;

#[derive(Clone, Debug, Default, Serialize, Patch, PartialEq, TS)]
pub struct FFPaths {
    pub ffmpeg_path: Option<PathBuf>,
    pub ffprobe_path: Option<PathBuf>,
}

impl<'de> serde::Deserialize<'de> for FFPaths {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Raw {
            ffmpeg: Option<PathBuf>,
            ffprobe: Option<PathBuf>,
        }

        let raw = Raw::deserialize(deserializer)?;

        let resolve = |p: Option<PathBuf>, name: &str| {
            p.or_else(|| find_executable(name))
                .filter(|p| p.is_file() && is_executable(p))
        };

        Ok(Self {
            ffmpeg_path: resolve(raw.ffmpeg, "ffmpeg"),
            ffprobe_path: resolve(raw.ffprobe, "ffprobe"),
        })
    }
}

impl FFPaths {
    pub fn flatten<'a>(&'a self) -> Option<FFPathsRef<'a>> {
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

#[derive(Debug, Default, Serialize, Deserialize, Patch, TS)]
#[patch(attribute(derive(Debug, Deserialize)))]
#[serde(default)]
#[ts(export)]
pub struct AppConfig {
    pub tracked_dirs: HashSet<PathBuf>,
    #[serde(flatten)]
    pub ffpaths: FFPaths,
    pub sidebar_width: u16,

    pub plugins: HashSet<String>,
    pub color_theme: Option<String>,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigField {
    TrackedDirs,
    FfmpegPath,
    FfprobePath,
    SidebarWidth,
    Plugins,
    ColorTheme,
}

struct SerPtr {
    ptr: *const (),
    serialize: fn(*const ()) -> Result<String, serde_json::Error>,
}

impl<T: Serialize> From<&T> for SerPtr {
    fn from(value: &T) -> Self {
        Self::new(value)
    }
}

impl SerPtr {
    pub fn new<T: Serialize>(value: &T) -> Self {
        Self {
            ptr: value as *const T as *const (),
            serialize: |ptr| {
                let value = unsafe { &*(ptr as *const T) };
                serde_json::to_string(value)
            },
        }
    }

    fn to_json(&self) -> Result<String, serde_json::Error> {
        (self.serialize)(self.ptr)
    }
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
        use ConfigField as C;

        let ptr: SerPtr = match field {
            C::TrackedDirs => (&self.tracked_dirs).into(),
            C::FfmpegPath => (&self.ffpaths.ffmpeg_path).into(),
            C::FfprobePath => (&self.ffpaths.ffprobe_path).into(),
            C::SidebarWidth => (&self.sidebar_width).into(),
            C::Plugins => (&self.plugins).into(),
            C::ColorTheme => (&self.color_theme).into(),
        };

        ptr.to_json()
    }

    pub fn mutate_config_field(&mut self, patch: AppConfigPatch) {
        self.apply(patch);
    }
}

fn find_executable(cmd: &str) -> Option<PathBuf> {
    which::which(cmd).ok()
}

fn is_executable(path: &impl AsRef<Path>) -> bool {
    let metadata = match std::fs::metadata(path.as_ref()) {
        Ok(m) => m,
        Err(_) => return false,
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
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "exe")
            .unwrap_or(false)
    }
}
