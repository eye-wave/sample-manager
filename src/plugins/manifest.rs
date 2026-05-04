use base64::{Engine as _, engine::general_purpose};
use std::{collections::HashMap, ops::Deref, sync::Arc};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    AStr, AnyResult,
    plugins::host::{HostState, StorageKey},
};

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq, Hash)]
pub struct PluginId(AStr);

impl PluginId {
    pub fn new(str: impl AsRef<str>) -> AnyResult<Self> {
        if str
            .as_ref()
            .chars()
            .any(|c| c.is_whitespace() || c == '<' || c == '>')
        {
            return Err("invalid plugin id".into());
        }

        Ok(Self(Arc::from(str.as_ref())))
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PluginId::new(s.as_str()).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct PluginMetadata {
    pub id: PluginId,
    pub name: AStr,
    pub description: AStr,
    pub version: AStr,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PluginManifest {
    #[serde(flatten)]
    pub meta: PluginMetadata,
    pub assets: PluginAssets,

    #[serde(default)]
    pub capabilities: PluginCapabilities,
    #[serde(default)]
    pub config_schema: HashMap<AStr, SchemaField>,

    /// Controls whether the plugin handles search itself (API call inside wasm)
    /// or provides a flat index that the host searches.
    #[serde(default)]
    pub search_mode: SearchMode,
}

impl Deref for PluginManifest {
    type Target = PluginMetadata;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

/// How a plugin surfaces samples to the host.
///
/// `Delegated`  — plugin receives the `SearchRequest` and returns matching
///                samples directly. Suitable for remote API plugins.
///
/// `HostIndexed` — plugin exposes `get_index()` which returns the full
///                 sample list. The host caches it and handles searching
///                 locally with the fuzzy matcher. Suitable for local
///                 file-registry plugins.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SearchMode {
    #[default]
    Delegated,
    HostIndexed {
        /// How many seconds the host index cache is valid for.
        /// After expiry the host calls `get_index()` again.
        #[serde(default = "default_ttl")]
        ttl_secs: u64,
    },
}

fn default_ttl() -> u64 {
    300 // 5 minutes
}

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

impl PluginManifest {
    pub fn to_plugin_info(&self, state: &HostState) -> PluginInfo {
        PluginInfo {
            is_enabled: true,
            meta: self.meta.clone(),
            capabilities: self.capabilities.clone(),
            icon: self.assets.icon.as_ref().map(|s| s.to_string()),
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

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PluginAssets {
    pub icon: Option<AStr>,
    pub entry: Option<AStr>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[serde(default)]
pub struct PluginCapabilities {
    pub storage: bool,
    pub encrypted_storage: bool,
    pub network: bool,
    pub network_allowlist: Vec<String>,
    pub filesystem: bool,
}

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

pub fn config_key(id: &PluginId, key: &str) -> StorageKey {
    (
        id.clone(),
        Arc::from(("CONFIG:".to_string() + key).as_str()),
    )
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct SchemaFieldWithValue {
    pub field_type: SchemaField,
    pub value: Option<String>,
}

pub fn stringify_bytes(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(err) => {
            let original_bytes = err.into_bytes();
            let encoded = general_purpose::STANDARD.encode(original_bytes);
            format!("base64:{}", encoded)
        }
    }
}

pub fn parse_string_to_bytes(s: String) -> Vec<u8> {
    if let Some(stripped) = s.strip_prefix("base64:") {
        general_purpose::STANDARD
            .decode(stripped)
            .unwrap_or_default()
    } else {
        s.into_bytes()
    }
}
