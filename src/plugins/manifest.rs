use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::AStr;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PluginManifest {
    pub id: AStr,
    pub name: AStr,
    pub description: AStr,
    pub version: AStr,

    #[serde(skip_serializing)]
    #[ts(skip)]
    pub assets: PluginAssets,

    #[serde(default)]
    pub capabilities: PluginCapabilities,
    #[serde(default)]
    pub config_schema: HashMap<AStr, SchemaField>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PluginAssets {
    pub icon: Option<Vec<u8>>,
    pub entry: Option<AStr>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export)]
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
#[ts(export)]
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
