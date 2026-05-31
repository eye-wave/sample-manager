use std::path::PathBuf;

use crate::plugins::PluginId;
use crate::state::app_paths;
use crate::window::PROTOCOL;

pub fn hash_path(plugin_id: Option<&PluginId>, path: &str) -> String {
    use base64::Engine;

    let mut hasher = blake3::Hasher::new();
    if let Some(id) = plugin_id {
        hasher.update(id.as_ref().as_bytes());
    }
    hasher.update(path.as_bytes());

    base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(hasher.finalize().as_bytes())
}

pub fn thumbnail_path(hashed: &str) -> PathBuf {
    app_paths::thumbnail_cache_path().join(hashed)
}

pub fn sync_path(id: &PluginId) -> PathBuf {
    app_paths::plugin_sync_path().join(format!("plug_{id}"))
}

pub fn thumbnail_uri(id: Option<PluginId>, hashed: &str) -> String {
    let base = if cfg!(target_os = "windows") {
        format!("https://{PROTOCOL}._")
    } else {
        format!("{PROTOCOL}://_")
    };

    match id {
        Some(id) => format!("{base}/thumb/plugin:{id}/{hashed}"),
        None => format!("{base}/thumb/native/{hashed}"),
    }
}
