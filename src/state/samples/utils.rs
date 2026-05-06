use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use ahash::AHasher;

use crate::AnyResult;
#[cfg(not(target_os = "windows"))]
use crate::plugins::PluginId;
use crate::state::app_paths;
use crate::window::PROTOCOL;

pub fn hash_path(path: &str) -> String {
    use base64::Engine;

    let mut hasher = AHasher::default();
    path.hash(&mut hasher);

    base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(hasher.finish().to_be_bytes())
}

pub fn thumbnail_path(hashed: &str) -> PathBuf {
    app_paths::thumbnail_cache_path().join(hashed)
}

pub fn sync_path(id: &PluginId, hashed: &str) -> AnyResult<PathBuf> {
    let parent = app_paths::plugin_sync_path().join(format!("plug_{id}"));

    fs::create_dir_all(&parent)?;

    Ok(parent.join(format!("{hashed}.wav")))
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
