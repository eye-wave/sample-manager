use std::{collections::HashSet, path::PathBuf};

use serde::{Deserialize, Serialize};

pub struct AppState {
    _config_path: PathBuf,
    _cache_path: PathBuf,
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
        let _cache_path = dirs::cache_dir().unwrap().join(APP_NAME);

        Self {
            _config_path,
            _cache_path,
            app_config: AppConfig::default(),
        }
    }
}

impl AppState {
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
