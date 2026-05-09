use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize, ser::Error};
use ts_rs::TS;

use crate::AStr;

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
    NumberList {
        label: AStr,
        separator: Option<AStr>,
        count: u8,
        default: Vec<f64>,
    },
    StringList {
        label: AStr,
        default: Vec<AStr>,
    },
    Select {
        label: AStr,
        options: SchemaFieldOptions,
        default: AStr,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SchemaFieldOptions {
    Grouped { groups: HashMap<AStr, Vec<AStr>> },
    List { values: Vec<AStr> },
}

impl SchemaFieldOptions {
    pub fn has_field(&self, name: &str) -> bool {
        match self {
            Self::List { values: l } => l.iter().any(|f| f.as_ref() == name),
            Self::Grouped { groups } => groups
                .values()
                .any(|v| v.iter().any(|f| f.as_ref() == name)),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SchemaFieldWithValue {
    Number {
        label: AStr,
        #[serde(default)]
        default: f64,
        value: f64,
    },
    String {
        label: AStr,
        #[serde(default)]
        default: AStr,
        #[serde(default)]
        is_password: bool,
        value: AStr,
    },
    Bool {
        label: AStr,
        #[serde(default)]
        default: bool,
        value: bool,
    },
    NumberList {
        label: AStr,
        separator: Option<AStr>,
        count: u8,
        default: Vec<f64>,
        value: Vec<f64>,
    },
    StringList {
        label: AStr,
        value: Vec<AStr>,
        default: Vec<AStr>,
    },
    Select {
        label: AStr,
        options: SchemaFieldOptions,
        default: AStr,
        value: AStr,
    },
}

impl SchemaFieldWithValue {
    pub fn swap_value(mut self, data: &str) -> Result<Self, serde_json::Error> {
        match &mut self {
            Self::Bool { value, .. } => *value = serde_json::from_str(data)?,
            Self::Number { value, .. } => *value = serde_json::from_str(data)?,
            Self::NumberList { value, count, .. } => {
                let list: Vec<_> = serde_json::from_str(data)?;
                for (i, n) in list.iter().enumerate().take(*count as usize) {
                    value[i] = *n;
                }
            }
            Self::StringList { value, .. } => {
                let list: Vec<_> = serde_json::from_str(data)?;

                value.clear();
                value.extend(list);
            }
            Self::String { value, .. } => *value = Arc::from(data),
            Self::Select { value, options, .. } => {
                if !options.has_field(data) {
                    return Err(serde_json::Error::custom(format!(
                        "Unknown variant '{data}', possible options are: {options:?}"
                    )));
                };

                *value = Arc::from(data)
            }
        }

        Ok(self)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        match self {
            Self::Bool { value, .. } => postcard::to_allocvec(value),
            Self::Number { value, .. } => postcard::to_allocvec(value),
            Self::NumberList { value, .. } => postcard::to_allocvec(value),
            Self::StringList { value, .. } => postcard::to_allocvec(value),
            Self::Select { value, .. } => postcard::to_allocvec(value),
            Self::String { value, .. } => postcard::to_allocvec(value),
        }
    }
}
