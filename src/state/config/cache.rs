use std::{collections::HashMap, fs, io, path::Path};

use byte_unit::Byte;
use serde::Deserialize;

use crate::state::app_paths;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppCacheSize {
    waveforms: Option<CacheWithCount>,
    plugin_storage: Option<Byte>,
    plugin_data: Option<HashMap<String, Byte>>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CacheWithCount {
    size: Byte,
    count: usize,
}

impl AppCacheSize {
    pub fn read() -> Self {
        Self {
            waveforms: read_waveforms_cache_size().ok(),
            plugin_storage: read_plugin_storage_cache().map(Byte::from_u64).ok(),
            plugin_data: read_plugin_data_cache_size().ok(),
        }
    }
}

fn read_waveforms_cache_size() -> io::Result<CacheWithCount> {
    let (size, count) = fs::read_dir(app_paths::thumbnail_cache_path())?
        .filter_map(Result::ok)
        .filter_map(|e| e.metadata().ok())
        .fold((0, 0), |(sum, count), meta| (sum + meta.len(), count + 1));

    let size = Byte::from_u64(size);

    Ok(CacheWithCount { count, size })
}

fn read_plugin_storage_cache() -> io::Result<u64> {
    [
        app_paths::plugin_storage_file(),
        app_paths::plugin_secret_storage_file(),
    ]
    .iter()
    .map(std::fs::metadata)
    .try_fold(0u64, |acc, meta| Ok(acc + meta?.len()))
}

fn read_plugin_data_cache_size() -> io::Result<HashMap<String, Byte>> {
    let mut sizes = HashMap::new();

    for entry in fs::read_dir(app_paths::plugin_cache_path())? {
        let entry = entry?;
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let size = read_dir_size(&entry.path())?;
            sizes.insert(name, Byte::from_u64(size));
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
