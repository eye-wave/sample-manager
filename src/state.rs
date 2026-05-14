use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Arc;

use crate::LogErrorExt;
use crate::audio::AudioPlayer;
use crate::plugins::{PluginId, PluginInfo, PluginRuntimeHandle};
use crate::state::config::{ConfigData, ConfigDataPatch};

pub mod app_paths;
pub mod config;
pub mod samples;

use config::AppConfig;
use samples::FsSample;

pub struct AppState {
    pub sample_registry: HashMap<Arc<Path>, FsSample>,
    pub audio_player: AudioPlayer,
    pub favorite_samples: HashSet<String>,
    pub plugin_handle: PluginRuntimeHandle,

    app_config: AppConfig,
}

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error")]
    Toml(#[from] toml::de::Error),
}

impl AppState {
    pub fn new() -> Self {
        let favorite_samples: HashSet<_> = fs::read_to_string(app_paths::favorites_file())
            .map(|f| f.lines().map(|l| l.into()).collect())
            .unwrap_or_default();

        let conf = AppConfig::load(app_paths::config_file());
        let plugin_handle = PluginRuntimeHandle::spawn();

        for name in conf.plugins.iter() {
            let plugin_name = name.to_string() + ".zip";
            let path = app_paths::plugin_config_path().join(plugin_name);

            match fs::read(path) {
                Ok(bytes) => plugin_handle.load(name, bytes),
                Err(err) => tracing::error!(
                    plugin = %name,
                    error = %err,
                    "failed to load plugin"
                ),
            }
        }

        Self {
            sample_registry: HashMap::new(),
            audio_player: AudioPlayer::new(),
            favorite_samples,
            plugin_handle,

            app_config: conf,
        }
    }

    pub fn update_config<R, F: FnMut(&mut AppConfig) -> R>(&mut self, mut cb: F) -> R {
        let result = cb(&mut self.app_config);

        if let Ok(contents) = toml::to_string::<ConfigData>(&self.app_config) {
            fs::write(app_paths::config_file(), contents).sure("Failed to write config file");
        }

        result
    }

    fn flush_favorites(&mut self) -> std::io::Result<()> {
        let file = File::create(app_paths::favorites_file())?;
        let mut writer = BufWriter::new(file);

        for f in self.favorite_samples.iter() {
            writeln!(writer, "{f}")?;
        }

        writer.flush()?;

        Ok(())
    }

    pub fn add_sample_to_fav(&mut self, path: &str) {
        self.favorite_samples.insert(path.to_string());
        let _ = self.flush_favorites();
    }

    pub fn remove_sample_from_fav(&mut self, path: &str) {
        self.favorite_samples.remove(path);
        let _ = self.flush_favorites();
    }

    pub fn get_config(&self) -> &AppConfig {
        &self.app_config
    }

    pub fn patch_config(&mut self, patch: ConfigDataPatch) {
        self.app_config.patch(patch);
    }

    pub fn mutate_config<F>(&mut self, cb: F)
    where
        F: FnMut(&mut ConfigData),
    {
        self.app_config.mutate(cb);
    }

    pub fn get_plugin_info(&self, id: PluginId) -> Option<PluginInfo> {
        self.plugin_handle.get_plugin_info(id)
    }
}
