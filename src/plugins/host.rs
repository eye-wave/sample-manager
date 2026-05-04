use std::{collections::HashMap, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    AStr, AnyResult,
    plugins::{PluginId, manifest::SchemaField},
};

pub type StorageKey = (PluginId, AStr);

#[derive(Serialize, Deserialize)]
pub struct HostState {
    pub storage: HashMap<StorageKey, Vec<u8>>,
    pub secrets: HashMap<StorageKey, Vec<u8>>,
    pub pending_download_path: Option<PathBuf>,
    #[serde(skip, default)]
    storage_path: PathBuf,
}

impl HostState {
    pub(super) fn new(storage_path: PathBuf) -> Self {
        let mut state = Self {
            storage: HashMap::new(),
            secrets: HashMap::new(),
            pending_download_path: None,
            storage_path,
        };
        // Best-effort load — if it fails we start fresh
        let _ = state.load_from_disk();
        state
    }

    fn load_from_disk(&mut self) -> AnyResult<()> {
        if !self.storage_path.exists() {
            return Ok(());
        }

        let bytes = std::fs::read(&self.storage_path)?;
        let host: Self = postcard::from_bytes(&bytes)?;

        self.storage = host.storage;
        self.secrets = host.secrets;
        Ok(())
    }

    pub fn flush(&self) -> AnyResult<()> {
        let bytes = postcard::to_allocvec(&self)?;
        let tmp = self.storage_path.with_extension("tmp");
        fs::write(&tmp, &bytes)?;
        fs::rename(&tmp, &self.storage_path)?;
        Ok(())
    }

    pub fn get_item(&self, key: StorageKey) -> Option<Vec<u8>> {
        self.storage.get(&key).cloned()
    }

    pub fn set_item(&mut self, key: StorageKey, data: Vec<u8>) {
        self.storage.insert(key, data);
        let _ = self.flush();
    }

    pub fn get_secret_item(&self, key: StorageKey) -> Option<Vec<u8>> {
        self.secrets.get(&key).cloned()
    }

    pub fn set_secret_item(&mut self, key: StorageKey, data: Vec<u8>) {
        self.secrets.insert(key, data);
        let _ = self.flush();
    }

    /// Assembles the current stored config values for a plugin as a
    /// plain string map. Used when passing config into `get_index`.
    pub fn get_plugin_config(
        &self,
        plugin_id: &PluginId,
        schema: &HashMap<AStr, SchemaField>,
    ) -> HashMap<String, String> {
        schema
            .keys()
            .filter_map(|key| {
                let storage_key = (plugin_id.clone(), key.clone());
                let bytes = self.get_item(storage_key)?;
                let value = String::from_utf8(bytes).ok()?;
                Some((key.to_string(), value))
            })
            .collect()
    }

    pub(super) fn is_url_allowed(&self, url: &str, allowlist: &[String]) -> bool {
        let Ok(parsed) = url::Url::parse(url) else {
            return false;
        };
        let Some(host) = parsed.host_str() else {
            return false;
        };
        allowlist
            .iter()
            .any(|allowed| host == allowed || host.ends_with(&format!(".{allowed}")))
    }
}
