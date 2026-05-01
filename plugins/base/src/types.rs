use crate::json::{self, JsonValue};
use std::collections::HashMap;

#[derive(Debug)]
pub struct SearchRequest {
    pub query: String,
    pub tags: Vec<String>,
    pub limit: u32,
}

impl SearchRequest {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let s = std::str::from_utf8(bytes).ok()?;
        let val = json::parse(s)?;
        Some(Self {
            query: json::get_str(&val, "query")?.to_owned(),
            limit: json::get_f64(&val, "limit").unwrap_or(50.0) as u32,
            tags: json::get_array(&val, "tags")
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| match v {
                            JsonValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Default)]
pub struct SampleResult {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub tags: Vec<String>,
    pub duration: Option<f32>,
    pub sample_rate: Option<u32>,
    pub metadata: HashMap<String, String>,
}

impl SampleResult {
    /// Serialize a slice of results to JSON bytes for `write_response`.
    pub fn serialize_all(results: &[SampleResult]) -> Vec<u8> {
        let mut out = String::from("[");
        for (i, r) in results.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&r.to_json());
        }
        out.push(']');
        out.into_bytes()
    }

    fn to_json(&self) -> String {
        let tags = self
            .tags
            .iter()
            .map(|t| format!("\"{}\"", escape_json(t)))
            .collect::<Vec<_>>()
            .join(",");

        let metadata = self
            .metadata
            .iter()
            .map(|(k, v)| format!("\"{}\":\"{}\"", escape_json(k), escape_json(v)))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            r#"{{"id":"{id}","name":"{name}","uri":"{uri}","tags":[{tags}],"duration":{dur},"sample_rate":{sr},"metadata":{{{meta}}},"source_plugin":""}}"#,
            id = escape_json(&self.id),
            name = escape_json(&self.name),
            uri = escape_json(&self.uri),
            tags = tags,
            dur = self
                .duration
                .map(|d| d.to_string())
                .unwrap_or_else(|| "null".into()),
            sr = self
                .sample_rate
                .map(|s| s.to_string())
                .unwrap_or_else(|| "null".into()),
            meta = metadata,
        )
    }
}

fn escape_json(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            '\n' => vec!['\\', 'n'],
            '\r' => vec!['\\', 'r'],
            '\t' => vec!['\\', 't'],
            c => vec![c],
        })
        .collect()
}
