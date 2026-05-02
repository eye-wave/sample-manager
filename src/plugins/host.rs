use std::collections::HashMap;

use crate::{AStr, plugins::PluginId};

pub struct HostState {
    pub storage: HashMap<(PluginId, AStr), Vec<u8>>,
}

impl HostState {
    pub(super) fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub fn get_item(&self, plugin_id: PluginId, key: AStr) -> Option<Vec<u8>> {
        self.storage.get(&(plugin_id, key)).cloned()
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
