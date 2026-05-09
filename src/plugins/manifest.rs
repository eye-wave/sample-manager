use std::{collections::HashMap, io::Read, ops::Deref, str::FromStr, sync::Arc};

use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use ts_rs::TS;

use crate::schema::{SchemaField, SchemaFieldWithValue};
use crate::{
    AStr,
    plugins::{
        host::{HostState, StorageKey},
        icon::SVGIcon,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("invalid plugin id: {0}")]
    InvalidPluginId(String),

    #[error("plugin '{0}' is missing a runtime entry (wasm file not declared in manifest)")]
    MissingPluginEntry(AStr),

    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("zip error")]
    Zip(#[from] zip::result::ZipError),

    #[error("TOML parse error")]
    Toml(#[from] toml::de::Error),
}

// -- PluginId -----------------------------------------------------------------

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq, Hash)]
pub struct PluginId(AStr);

impl PluginId {
    pub fn new(str: impl AsRef<str>) -> Result<Self, ManifestError> {
        const FORBIDDEN: &[char] = &['<', '>', ':'];
        let s = str.as_ref();

        if s == "__APP_SETTINGS__"
            || s.chars()
                .any(|c| c.is_whitespace() || FORBIDDEN.contains(&c))
        {
            return Err(ManifestError::InvalidPluginId(str.as_ref().to_owned()));
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

fn fetch<T>(bytes: Option<Vec<u8>>, default: impl Fn() -> T, validate: impl Fn(&T) -> bool) -> T
where
    T: for<'a> Deserialize<'a>,
{
    bytes
        .and_then(|b| postcard::from_bytes(&b).ok())
        .filter(validate)
        .unwrap_or_else(default)
}

#[rustfmt::skip]
impl SchemaField {
    pub fn with_fetched_value(&self, plugin_id: PluginId, key: &str, state: &HostState) -> SchemaFieldWithValue {
        let bytes = state.get_item(config_key(&plugin_id, key));
        let b = || bytes.clone();

        match self {
            Self::Number { label, default } =>
                SchemaFieldWithValue::Number { label: label.clone(), default: *default, value: fetch(b(), || *default, |_| true) },
            Self::Bool { label, default } =>
                SchemaFieldWithValue::Bool { label: label.clone(), default: *default, value: fetch(b(), || *default, |_| true) },
            Self::String { label, default, is_password } =>
                SchemaFieldWithValue::String { label: label.clone(), default: default.clone(), is_password: *is_password, value: fetch(b(), || default.clone(), |_| true) },
            Self::Select { label, options, default } =>
                SchemaFieldWithValue::Select { label: label.clone(), options: options.clone(), default: default.clone(), value: fetch(b(), || default.clone(), |v| options.has_field(v)) },
            Self::NumberList { label, separator, count, default } =>
                SchemaFieldWithValue::NumberList { label: label.clone(), separator: separator.clone(), count: *count, default: default.clone(), value: fetch(b(), || default.clone(), |v: &Vec<f64>| v.len() == *count as usize) },
        }
    }
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
    pub fn load_from_bytes(bytes: &[u8]) -> Result<(Self, Vec<u8>), ManifestError> {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;

        let raw: RawPluginManifest = {
            let mut f = zip.by_name("Manifest.toml")?;
            let mut s = String::new();

            f.read_to_string(&mut s)?;

            toml::from_str(&s).map_err(|e| {
                tracing::error!(
                    error = %e,
                    manifest = %s,
                    "Failed to parse Manifest.toml"
                );

                e
            })?
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
            None => return Err(ManifestError::MissingPluginEntry(manifest.name.clone())),
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
                Arc::from(icon.to_string().as_ref())
            }),
            config: self
                .config_schema
                .iter()
                .map(|(key, field)| {
                    (
                        key.clone(),
                        field.with_fetched_value(self.id.clone(), key, state),
                    )
                })
                .collect(),
        }
    }
}

// -- PluginInfo (serializable view) -------------------------------------------

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginInfo {
    pub is_enabled: bool,
    #[serde(flatten)]
    pub meta: PluginMetadata,
    pub icon: Option<AStr>,
    pub capabilities: PluginCapabilities,
    pub config: HashMap<AStr, SchemaFieldWithValue>,
}

impl PluginInfo {
    pub fn get_field(&self, name: &AStr) -> Option<SchemaFieldWithValue> {
        self.config.get(name).cloned()
    }
}

// -- Raw deserialization types (internal) -------------------------------------

#[derive(Deserialize)]
struct RawPluginManifest {
    #[serde(flatten)]
    pub meta: PluginMetadata,
    pub assets: RawPluginAssets,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
    #[serde(default, deserialize_with = "deserialize_schema_map")]
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

fn deserialize_schema_map<'de, D>(deserializer: D) -> Result<HashMap<AStr, SchemaField>, D::Error>
where
    D: Deserializer<'de>,
{
    struct MapVisitor;

    impl<'de> Visitor<'de> for MapVisitor {
        type Value = HashMap<AStr, SchemaField>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "a map of schema fields")
        }

        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut out = HashMap::new();

            while let Some((key, value)) = map.next_entry::<AStr, toml::Value>()? {
                match value.try_into::<SchemaField>() {
                    Ok(field) => {
                        out.insert(key, field);
                    }
                    Err(e) => {
                        tracing::warn!(key = %key, error = %e, "Skipping invalid config_schema entry");
                    }
                }
            }

            Ok(out)
        }
    }

    deserializer.deserialize_map(MapVisitor)
}
