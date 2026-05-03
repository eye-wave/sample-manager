use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};

use crate::AnyResult;
use crate::audio::AudioPlayer;
use crate::ipc::IPCMessage;
use crate::plugins::{PluginInfo, PluginRuntimeHandle};
use crate::state::config::{AppConfigPatch, find_executable, is_executable};

pub mod app_paths;
pub mod config;
pub mod samples;

use config::AppConfig;
use samples::FsSample;

pub struct AppState {
    pub sample_registry: HashMap<Arc<Path>, FsSample>,
    pub audio_player: AudioPlayer,
    pub favorite_samples: HashSet<PathBuf>,
    pub plugin_handle: PluginRuntimeHandle,
    pub loaded_plugins_info: Vec<PluginInfo>,

    app_config: AppConfig,
}

impl AppState {
    pub fn new(rx: mpsc::Sender<IPCMessage>) -> Self {
        Self {
            sample_registry: HashMap::new(),
            audio_player: AudioPlayer::new(rx),
            favorite_samples: HashSet::new(),
            plugin_handle: PluginRuntimeHandle::spawn(),

            app_config: AppConfig::default(),
            loaded_plugins_info: Vec::new(),
        }
    }

    pub fn init(&mut self) -> AnyResult<()> {
        let conf = fs::read(app_paths::config_file())?;
        let conf: AppConfig = toml::from_slice(&conf)?;

        let favorite_samples: HashSet<PathBuf> = fs::read_to_string(app_paths::favorites_file())
            .map(|f| f.lines().map(|l| l.into()).collect())
            .unwrap_or_default();

        self.app_config = conf;
        self.favorite_samples = favorite_samples;

        if self.app_config.ffmpeg_path.is_none() {
            if let Some(path) = find_executable("ffmpeg")
                && is_executable(&path)
            {
                self.app_config.ffmpeg_path = Some(path)
            }
        } else {
            if !is_executable(self.app_config.ffmpeg_path.as_ref().unwrap()) {
                self.app_config.ffmpeg_path = None
            }
        }

        for name in self.app_config.plugins.iter() {
            let plugin_name = name.to_string() + ".zip";
            let path = app_paths::plugin_config_path().join(plugin_name);

            match fs::read(path) {
                Ok(bytes) => self.plugin_handle.load(name, bytes),
                Err(err) => eprintln!("Failed to load plugin '{name}'.\n\t{err}"),
            }
        }

        self.loaded_plugins_info = self.plugin_handle.get_all_plugins_info();

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

    pub fn mutate_config_field(&mut self, patch: AppConfigPatch) {
        self.app_config.mutate_config_field(patch);
        self.update_config(|_| {});
    }
}
