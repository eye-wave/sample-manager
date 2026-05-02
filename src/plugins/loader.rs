use std::sync::Arc;

use crate::{AnyResult, plugins::manifest::PluginManifest};

pub fn unpack_plugin_zip(bytes: &[u8]) -> AnyResult<(PluginManifest, Vec<u8>)> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor)?;

    let mut manifest: PluginManifest = {
        let mut f = zip.by_name("Manifest.toml")?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        toml::from_str(&s)?
    };

    let svg_icon = manifest.assets.icon.as_ref().and_then(|path| {
        let mut f = zip.by_name(path.as_ref()).ok()?;
        let mut s = String::new();
        f.read_to_string(&mut s).ok()?;

        Some(Arc::from(s))
    });

    manifest.assets.icon = svg_icon;

    let _wasm_path = &manifest.assets.entry;

    let wasm_bytes = if let Some(path) = &manifest.assets.entry {
        let mut f = zip.by_name(path)?;
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf)?;
        buf
    } else {
        return Err("Manifest missing assets.entry wasm path".into());
    };

    Ok((manifest, wasm_bytes))
}

pub fn unpack_plugin_metadata(bytes: &[u8]) -> AnyResult<PluginManifest> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor)?;

    let mut manifest: PluginManifest = {
        let mut f = zip.by_name("Manifest.toml")?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        toml::from_str(&s)?
    };

    let svg_icon = manifest.assets.icon.as_ref().and_then(|path| {
        let mut f = zip.by_name(path.as_ref()).ok()?;
        let mut s = String::new();
        f.read_to_string(&mut s).ok()?;

        Some(Arc::from(s))
    });

    manifest.assets.icon = svg_icon;

    Ok(manifest)
}
