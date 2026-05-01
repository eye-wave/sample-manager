use serde::Deserialize;

use crate::AStr;

#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub id: AStr,
    pub name: AStr,
    pub description: AStr,
    pub version: AStr,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
}

#[derive(Debug, Default, Deserialize)]
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
