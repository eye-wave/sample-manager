use std::{collections::HashSet, fs, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::state::samples::FsSample;

pub mod samples;

pub struct AppState {
    _config_path: PathBuf,
    pub cache_path: PathBuf,
    pub sample_registry: Arc<[FsSample]>,

    app_config: AppConfig,
}

#[derive(Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub tracked_dirs: HashSet<PathBuf>,
}

impl Default for AppState {
    fn default() -> Self {
        const APP_NAME: &str = "SampleVault";

        let _config_path = dirs::config_local_dir().unwrap().join(APP_NAME);
        let cache_path = dirs::cache_dir().unwrap().join(APP_NAME);

        Self {
            _config_path,
            cache_path,
            app_config: AppConfig::default(),
            sample_registry: Arc::new([]),
        }
    }
}

impl AppState {
    pub fn load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let conf = fs::read(self.config_file())?;
        let conf: AppConfig = toml::from_slice(&conf)?;

        self.app_config = conf;

        Ok(())
    }

    pub fn create_dirs(&self) {
        fs::create_dir_all(&self.cache_path).ok();
        fs::create_dir_all(&self._config_path).ok();
    }

    pub fn update_config<F: FnMut(&mut AppConfig)>(&mut self, mut cb: F) {
        cb(&mut self.app_config);

        if let Ok(contents) = toml::to_string(&self.app_config) {
            std::fs::write(self.config_file(), contents).ok();
        }
    }

    pub fn get_config(&self) -> &AppConfig {
        &self.app_config
    }

    fn config_file(&self) -> PathBuf {
        const CONFIG_NAME: &str = "config.toml";
        self._config_path.join(CONFIG_NAME)
    }
}
