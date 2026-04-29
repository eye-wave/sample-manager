use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::{fs, io};

use crate::audio::AudioPlayer;
use crate::ipc::IPCMessage;

pub mod config;
pub mod samples;

use config::AppConfig;
use samples::FsSample;

pub mod app_paths {
    use super::*;

    pub const APP_NAME: &str = "SampleVault";

    fn cache_path() -> PathBuf {
        dirs::cache_dir().unwrap().join(APP_NAME)
    }

    fn config_path() -> PathBuf {
        dirs::config_local_dir().unwrap().join(APP_NAME)
    }

    pub fn config_file() -> PathBuf {
        config_path().join("config.toml")
    }

    pub fn favorites_file() -> PathBuf {
        cache_path().join(".favorites")
    }

    pub fn themes_path() -> PathBuf {
        config_path().join("themes")
    }

    pub fn thumbnail_cache_path() -> PathBuf {
        cache_path().join(".waves")
    }

    pub fn create_all_dirs() -> io::Result<()> {
        fs::create_dir_all(thumbnail_cache_path())?;
        fs::create_dir_all(themes_path())?;

        Ok(())
    }
}

pub struct AppState {
    pub sample_registry: HashMap<Arc<Path>, FsSample>,
    pub audio_player: AudioPlayer,
    pub favorite_samples: HashSet<PathBuf>,

    app_config: AppConfig,
}

impl AppState {
    pub fn new(rx: mpsc::Sender<IPCMessage>) -> Self {
        Self {
            sample_registry: HashMap::new(),
            audio_player: AudioPlayer::new(rx),

            favorite_samples: HashSet::new(),
            app_config: AppConfig::default(),
        }
    }

    pub fn load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let conf = fs::read(app_paths::config_file())?;
        let conf: AppConfig = toml::from_slice(&conf)?;

        let favorite_samples: HashSet<PathBuf> = fs::read_to_string(app_paths::favorites_file())
            .map(|f| f.lines().map(|l| l.into()).collect())
            .unwrap_or_default();

        self.app_config = conf;
        self.favorite_samples = favorite_samples;

        Ok(())
    }

    pub fn update_config<R, F: FnMut(&mut AppConfig) -> R>(&mut self, mut cb: F) -> R {
        let result = cb(&mut self.app_config);

        if let Ok(contents) = toml::to_string(&self.app_config) {
            fs::write(app_paths::config_file(), contents).ok();
        }

        result
    }

    fn rewrite_favorites(&mut self) {
        let _ = fs::write(
            app_paths::favorites_file(),
            self.favorite_samples
                .iter()
                .map(|f| f.to_string_lossy())
                .intersperse("\n".into())
                .collect::<String>(),
        );
    }

    pub fn add_sample_to_fav(&mut self, path: PathBuf) {
        self.favorite_samples.insert(path);
        self.rewrite_favorites();
    }

    pub fn remove_sample_from_fav(&mut self, path: &Path) {
        self.favorite_samples.remove(path);
        self.rewrite_favorites();
    }

    pub fn is_sample_fav(&self, path: &Path) -> bool {
        self.favorite_samples.contains(path)
    }

    pub fn get_config(&self) -> &AppConfig {
        &self.app_config
    }
}
