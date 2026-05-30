use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use plugin_wire::WireEntry;

use rayon::iter::ParallelBridge;
use serde::{Serialize, de::DeserializeOwned};

use crate::LogErrorExt;
use crate::schema::SchemaField;
use crate::state::samples::{PluginSample, SearchRequest, filter_samples};
use crate::{AStr, plugins::PluginId, state::app_paths};

pub type StorageKey = (PluginId, AStr);

pub struct PendingDownload {
    pub bytes: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("postcard serialization/deserialization error")]
    Postcard(#[from] postcard::Error),
}

#[derive(Default)]
pub struct HostState {
    pub storage: HashMap<StorageKey, Vec<u8>>,
    pub secrets: HashMap<StorageKey, Vec<u8>>,
    pub pending_download: Option<PendingDownload>,

    entry_cache: HashMap<PluginId, HashSet<WireEntry>>,
    local_cache: HashMap<AStr, PluginSample>,
}

impl Drop for HostState {
    fn drop(&mut self) {
        let _ = self.flush_storage();
        let _ = self.flush_secret();
        let _ = self.flush_cache();
    }
}

impl HostState {
    pub(super) fn new() -> Self {
        let mut state = Self::default();
        let _ = state.load_from_disk();

        state
    }

    fn load<T: DeserializeOwned>(target: &mut T, path: &Path) -> Result<(), HostError> {
        let bytes = fs::read(path)?;
        *target = postcard::from_bytes(&bytes)?;

        Ok(())
    }

    fn load_from_disk(&mut self) {
        Self::load(&mut self.storage, app_paths::plugin_storage_file())
            .sure("Failed to load plugin storage");
        Self::load(&mut self.secrets, app_paths::plugin_secret_storage_file())
            .sure("Failed to load plugin secret storage");
        Self::load(&mut self.entry_cache, app_paths::plugin_entry_cache_file())
            .sure("Failed to load plugin entry cache");
    }

    fn flush<T: Serialize>(&self, target: &T, path: &Path) -> Result<(), HostError> {
        let bytes = postcard::to_allocvec(target)?;
        println!("flushing... {}", bytes.len());

        let tmp = path.with_extension("tmp");
        fs::write(&tmp, &bytes)?;
        fs::rename(&tmp, path)?;

        Ok(())
    }

    fn flush_storage(&self) -> Result<(), HostError> {
        self.flush(&self.storage, app_paths::plugin_storage_file())
    }

    fn flush_secret(&self) -> Result<(), HostError> {
        self.flush(&self.secrets, app_paths::plugin_secret_storage_file())
    }

    fn flush_cache(&self) -> Result<(), HostError> {
        self.flush(&self.entry_cache, app_paths::plugin_entry_cache_file())
    }

    pub fn get_item(&self, key: StorageKey) -> Option<Vec<u8>> {
        self.storage.get(&key).cloned()
    }

    pub fn set_item(&mut self, key: StorageKey, data: Vec<u8>) {
        self.storage.insert(key, data);
        let _ = self.flush_storage();
    }

    pub fn get_secret_item(&self, key: StorageKey) -> Option<Vec<u8>> {
        self.secrets.get(&key).cloned()
    }

    pub fn set_secret_item(&mut self, key: StorageKey, data: Vec<u8>) {
        self.secrets.insert(key, data);
    }

    pub fn insert_search_cache<'a>(&mut self, items: impl Iterator<Item = &'a PluginSample>) {
        for item in items {
            self.local_cache
                .insert(Arc::from(item.url.as_str()), item.clone());
        }
    }

    pub fn insert_cached_sample(&mut self, url: AStr) {
        println!("{url}");

        let Some(sample) = self.local_cache.get(&url) else {
            return;
        };

        let plug_id = &sample.plugin_id;

        let container = match self.entry_cache.get_mut(plug_id) {
            Some(container) => container,
            None => {
                self.entry_cache.insert(plug_id.clone(), HashSet::new());
                self.entry_cache.get_mut(plug_id).unwrap()
            }
        };

        let mut sample = sample.entry.clone();

        sample.clear_url();
        container.insert(sample);

        let _ = self.flush_cache();
    }

    pub fn search_local_registry_for(
        &self,
        req: &SearchRequest,
        plugin_id: &PluginId,
    ) -> Arc<Vec<PluginSample>> {
        let empty_set: std::collections::HashSet<WireEntry> = HashSet::new();

        let entries = self
            .local_cache
            .iter()
            .filter(|s| s.1.plugin_id == *plugin_id)
            .map(|s| &s.1.entry)
            .chain(self.entry_cache.get(plugin_id).unwrap_or(&empty_set).iter())
            .par_bridge();

        let results = filter_samples(entries, req);

        Arc::new(
            results
                .1
                .iter()
                .map(|wire_ref| PluginSample::new((*wire_ref).clone(), plugin_id.clone()))
                .collect(),
        )
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
                let value = String::from_utf8(bytes).sure("Invalid string")?;
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
