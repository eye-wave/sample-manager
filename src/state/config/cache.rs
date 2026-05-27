use std::{collections::HashMap, fs, io, path::Path};

use serde::Serialize;
use ts_rs::TS;

use crate::state::app_paths;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppCacheSize {
    waveforms: Option<CacheWithCount>,
    plugin_storage: u64,
    plugin_data: Option<HashMap<String, u64>>,
}

#[derive(Serialize, Default, TS)]
#[serde(rename_all = "camelCase")]
pub struct CacheWithCount {
    size: u64,
    count: usize,
}

impl AppCacheSize {
    pub fn read() -> Self {
        Self {
            waveforms: read_waveforms_cache_size().ok(),
            plugin_storage: read_plugin_storage_cache(),
            plugin_data: read_plugin_data_cache_size().ok(),
        }
    }
}

fn read_waveforms_cache_size() -> io::Result<CacheWithCount> {
    let (size, count) = fs::read_dir(app_paths::thumbnail_cache_path())?
        .filter_map(Result::ok)
        .filter_map(|e| e.metadata().ok())
        .fold((0, 0), |(sum, count), meta| (sum + meta.len(), count + 1));

    Ok(CacheWithCount { count, size })
}

fn read_plugin_storage_cache() -> u64 {
    [
        app_paths::plugin_storage_file(),
        app_paths::plugin_secret_storage_file(),
    ]
    .iter()
    .map(std::fs::metadata)
    .filter_map(Result::ok)
    .map(|m| m.len())
    .sum()
}

fn read_plugin_data_cache_size() -> io::Result<HashMap<String, u64>> {
    let mut sizes = HashMap::new();

    for entry in fs::read_dir(app_paths::plugin_cache_path())? {
        let entry = entry?;
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let size = read_dir_size(&entry.path())?;
            sizes.insert(name, size);
        }
    }

    Ok(sizes)
}

fn read_dir_size(path: &Path) -> io::Result<u64> {
    let mut total_size = 0;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            total_size += read_dir_size(&entry.path())?;
        } else if file_type.is_file() {
            total_size += entry.metadata()?.len();
        }
    }

    Ok(total_size)
}
