use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use plugin_wire::WireEntry;

use rayon::iter::ParallelBridge;
use serde::{Serialize, de::DeserializeOwned};

use crate::state::samples::{SearchRequest, filter_samples};
use crate::{
    AStr, AnyResult,
    plugins::{PluginId, manifest::SchemaField},
    state::app_paths,
};

pub type StorageKey = (PluginId, AStr);

pub struct PendingDownload {
    pub bytes: Vec<u8>,
}

pub struct HostState {
    pub storage: HashMap<StorageKey, Vec<u8>>,
    pub secrets: HashMap<StorageKey, Vec<u8>>,
    pub pending_download: Option<PendingDownload>,

    entry_cache: HashMap<PluginId, HashSet<WireEntry>>,
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
        let mut state = Self {
            storage: HashMap::new(),
            secrets: HashMap::new(),
            entry_cache: HashMap::new(),
            pending_download: None,
        };
        // Best-effort load — if it fails we start fresh
        let _ = state.load_from_disk();
        state
    }

    fn load<T: DeserializeOwned>(target: &mut T, path: &Path) -> AnyResult<()> {
        let bytes = fs::read(path)?;
        *target = postcard::from_bytes(&bytes)?;

        Ok(())
    }

    fn load_from_disk(&mut self) -> AnyResult<()> {
        Self::load(&mut self.storage, app_paths::plugin_storage_file())?;
        Self::load(&mut self.secrets, app_paths::plugin_secret_storage_file())?;
        Self::load(&mut self.entry_cache, app_paths::plugin_entry_cache_file())?;

        Ok(())
    }

    fn flush<T: Serialize>(&self, target: &T, path: &Path) -> AnyResult<()> {
        let bytes = postcard::to_allocvec(target)?;
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, &bytes)?;
        fs::rename(&tmp, path)?;

        Ok(())
    }

    fn flush_storage(&self) -> AnyResult<()> {
        self.flush(&self.storage, app_paths::plugin_storage_file())
    }

    fn flush_secret(&self) -> AnyResult<()> {
        self.flush(&self.secrets, app_paths::plugin_secret_storage_file())
    }

    fn flush_cache(&self) -> AnyResult<()> {
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

    pub fn write_sample_cache<'a>(
        &mut self,
        plugin_id: &PluginId,
        results: impl Iterator<Item = &'a WireEntry>,
    ) {
        if let Some(container) = self.entry_cache.get_mut(plugin_id) {
            for r in results {
                container.insert(r.clone());
            }
        }
    }

    pub fn search_local_registry(&self, req: &SearchRequest) -> Arc<Vec<WireEntry>> {
        let entries = self
            .entry_cache
            .values()
            .flat_map(|c| c.iter())
            .par_bridge();

        let results = filter_samples(entries, req);

        Arc::new(results.iter().map(|c| (*c).clone()).collect())
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
