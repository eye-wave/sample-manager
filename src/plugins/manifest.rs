use base64::{Engine as _, engine::general_purpose};
use std::{collections::HashMap, ops::Deref};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{AStr, plugins::host::HostState};

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct PluginMetadata {
    pub id: AStr,
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
}

impl Deref for PluginManifest {
    type Target = PluginMetadata;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PluginInfo {
    #[serde(flatten)]
    pub meta: PluginMetadata,
    pub icon: Option<String>,

    pub capabilities: PluginCapabilities,
    pub config: HashMap<String, SchemaFieldWithValue>,
}

impl PluginManifest {
    pub fn to_plugin_info(&self, state: &HostState) -> PluginInfo {
        PluginInfo {
            meta: self.meta.clone(),
            capabilities: self.capabilities.clone(),
            icon: self.assets.icon.as_ref().map(|s| s.to_string()),
            config: self
                .config_schema
                .iter()
                .map(|(key, field)| {
                    (
                        key.to_string(),
                        field.with_fetched_value(self.id.clone(), key.clone(), state),
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
pub struct PluginCapabilities {
    #[serde(default)]
    pub storage: bool,
    #[serde(default)]
    pub encrypted_storage: bool,
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub network_allowlist: Vec<String>,
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
    fn with_fetched_value(
        &self,
        plugin_id: AStr,
        key: AStr,
        state: &HostState,
    ) -> SchemaFieldWithValue {
        SchemaFieldWithValue {
            field_type: self.clone(),
            value: state.get_item(plugin_id, key).map(stringify_bytes),
        }
    }
}

#[derive(Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct SchemaFieldWithValue {
    field_type: SchemaField,
    value: Option<String>,
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
