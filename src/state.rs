use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use crate::audio::AudioPlayer;

pub mod config;
pub mod samples;

use config::AppConfig;
use samples::FsSample;

pub const APP_NAME: &str = "SampleVault";

pub struct AppDirs;

impl AppDirs {
    fn _cache_path() -> PathBuf {
        dirs::cache_dir().unwrap().join(APP_NAME)
    }

    fn _config_path() -> PathBuf {
        dirs::config_local_dir().unwrap().join(APP_NAME)
    }

    pub fn themes_path() -> PathBuf {
        Self::_config_path().join("themes")
    }

    pub fn thumbnail_cache_path() -> PathBuf {
        Self::_cache_path().join(".waves")
    }

    pub fn create_all_dirs() -> io::Result<()> {
        std::fs::create_dir_all(Self::thumbnail_cache_path())?;
        std::fs::create_dir_all(Self::themes_path())?;

        Ok(())
    }
}

pub struct AppState {
    pub sample_registry: Arc<[FsSample]>,
    pub audio_player: AudioPlayer,

    app_config: AppConfig,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            sample_registry: Arc::new([]),
            audio_player: AudioPlayer::new(),

            app_config: AppConfig::default(),
        }
    }
}

impl AppState {
    pub fn load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let conf = std::fs::read(self.config_file())?;
        let conf: AppConfig = toml::from_slice(&conf)?;

        self.app_config = conf;

        Ok(())
    }

    pub fn update_config<R, F: FnMut(&mut AppConfig) -> R>(&mut self, mut cb: F) -> R {
        let result = cb(&mut self.app_config);

        if let Ok(contents) = toml::to_string(&self.app_config) {
            std::fs::write(self.config_file(), contents).ok();
        }

        result
    }

    pub fn get_config(&self) -> &AppConfig {
        &self.app_config
    }

    fn config_file(&self) -> PathBuf {
        const CONFIG_NAME: &str = "config.toml";
        AppDirs::_config_path().join(CONFIG_NAME)
    }
}
