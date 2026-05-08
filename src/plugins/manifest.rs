use std::{collections::HashMap, io::Read, ops::Deref, str::FromStr, sync::Arc};

use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    AStr, AnyResult,
    plugins::{
        host::{HostState, StorageKey},
        icon::SVGIcon,
    },
};

// -- PluginId -----------------------------------------------------------------

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq, Hash)]
pub struct PluginId(AStr);

impl PluginId {
    pub fn new(str: impl AsRef<str>) -> AnyResult<Self> {
        const FORBIDDEN: &[char] = &['<', '>', ':'];
        let s = str.as_ref();

        if s.chars()
            .any(|c| c.is_whitespace() || FORBIDDEN.contains(&c))
        {
            return Err("invalid plugin id".into());
        }

        Ok(Self(Arc::from(s)))
    }
}

impl std::fmt::Display for PluginId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for PluginId {
    type Target = AStr;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for PluginId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        PluginId::new(s.as_str()).map_err(serde::de::Error::custom)
    }
}

// -- Metadata & Assets --------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct PluginMetadata {
    pub id: PluginId,
    pub name: AStr,
    pub description: AStr,
    pub version: AStr,
}

#[derive(Clone, Debug, Default)]
pub struct PluginAssets {
    pub icon: Option<SVGIcon>,
    pub entry: Option<AStr>,
}

// -- SearchMode ---------------------------------------------------------------

/// How a plugin surfaces samples to the host.
///
/// `Delegated`   — plugin receives the `SearchRequest` and returns matching
///                 samples directly. Suitable for remote API plugins.
///
/// `HostIndexed` — plugin exposes `get_index()` which returns the full sample
///                 list. The host caches it and handles searching locally with
///                 the fuzzy matcher. Suitable for local file-registry plugins.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SearchMode {
    #[default]
    Delegated,
    HostIndexed {
        /// Seconds the host index cache is valid for before calling
        /// `get_index()` again.
        #[serde(default = "default_ttl")]
        ttl_secs: u64,
    },
}

fn default_ttl() -> u64 {
    300 // 5 minutes
}

// -- Capabilities -------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct PluginCapabilities {
    pub storage: bool,
    pub encrypted_storage: bool,
    pub network: bool,
    pub network_allowlist: Vec<String>,
    pub filesystem: bool,
}

// -- Config schema ------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SchemaField {
    Number {
        label: AStr,
        #[serde(default)]
        default: f64,
    },
    String {
        label: AStr,
        #[serde(default)]
        default: AStr,
        #[serde(default)]
        is_password: bool,
    },
    Bool {
        label: AStr,
        #[serde(default)]
        default: bool,
    },
    Select {
        label: AStr,
        options: Vec<AStr>,
        default: AStr,
    },
}

impl SchemaField {
    pub fn with_fetched_value(
        &self,
        plugin_id: PluginId,
        key: &str,
        state: &HostState,
    ) -> SchemaFieldWithValue {
        SchemaFieldWithValue {
            field_type: self.clone(),
            value: state
                .get_item(config_key(&plugin_id, key))
                .map(stringify_bytes),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct SchemaFieldWithValue {
    pub field_type: SchemaField,
    pub value: Option<String>,
}

pub fn config_key(id: &PluginId, key: &str) -> StorageKey {
    (
        id.clone(),
        Arc::from(("CONFIG:".to_string() + key).as_str()),
    )
}

// -- PluginManifest -----------------------------------------------------------

pub struct PluginManifest {
    pub meta: PluginMetadata,
    pub assets: PluginAssets,
    pub capabilities: PluginCapabilities,
    pub config_schema: HashMap<AStr, SchemaField>,
    pub search_mode: SearchMode,
}

impl Deref for PluginManifest {
    type Target = PluginMetadata;
    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl PluginManifest {
    pub fn load_from_bytes(bytes: &[u8]) -> AnyResult<(Self, Vec<u8>)> {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;

        let raw: RawPluginManifest = {
            let mut f = zip.by_name("Manifest.toml")?;
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            toml::from_str(&s)?
        };

        let icon = raw.assets.icon.as_ref().and_then(|path| {
            let mut f = zip.by_name(path.as_ref()).ok()?;
            let mut s = String::new();
            f.read_to_string(&mut s).ok()?;
            SVGIcon::from_str(&s).ok()
        });

        let manifest = Self {
            meta: raw.meta,
            assets: PluginAssets {
                icon,
                entry: raw.assets.entry,
            },
            capabilities: raw.capabilities,
            config_schema: raw.config_schema,
            search_mode: raw.search_mode,
        };

        let wasm_bytes = match &manifest.assets.entry {
            Some(path) => {
                let mut f = zip.by_name(path)?;
                let mut buf = Vec::with_capacity(f.size() as usize);
                f.read_to_end(&mut buf)?;
                buf
            }
            None => return Err("Manifest missing assets.entry wasm path".into()),
        };

        Ok((manifest, wasm_bytes))
    }

    pub fn to_plugin_info<F>(&self, state: &HostState, icon_cb: F) -> PluginInfo
    where
        F: Fn(&mut SVGIcon),
    {
        PluginInfo {
            is_enabled: true,
            meta: self.meta.clone(),
            capabilities: self.capabilities.clone(),
            icon: self.assets.icon.as_ref().map(|s| {
                let mut icon = s.clone();
                icon_cb(&mut icon);
                icon.to_string()
            }),
            config: self
                .config_schema
                .iter()
                .map(|(key, field)| {
                    (
                        key.to_string(),
                        field.with_fetched_value(self.id.clone(), key, state),
                    )
                })
                .collect(),
        }
    }
}

// -- PluginInfo (serializable view) -------------------------------------------

#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginInfo {
    pub is_enabled: bool,
    #[serde(flatten)]
    pub meta: PluginMetadata,
    pub icon: Option<String>,
    pub capabilities: PluginCapabilities,
    pub config: HashMap<String, SchemaFieldWithValue>,
}

// -- Raw deserialization types (internal) -------------------------------------

#[derive(Deserialize)]
struct RawPluginManifest {
    #[serde(flatten)]
    pub meta: PluginMetadata,
    pub assets: RawPluginAssets,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
    #[serde(default)]
    pub config_schema: HashMap<AStr, SchemaField>,
    #[serde(default)]
    pub search_mode: SearchMode,
}

#[derive(Deserialize)]
struct RawPluginAssets {
    #[serde(default)]
    pub icon: Option<String>,
    pub entry: Option<AStr>,
}

// -- Byte / string helpers -----------------------------------------------------

pub fn stringify_bytes(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(err) => {
            let encoded = general_purpose::STANDARD.encode(err.into_bytes());
            format!("base64:{encoded}")
        }
    }
}

pub fn parse_string_to_bytes(s: String) -> Vec<u8> {
    match s.strip_prefix("base64:") {
        Some(encoded) => general_purpose::STANDARD
            .decode(encoded)
            .unwrap_or_default(),
        None => s.into_bytes(),
    }
}
