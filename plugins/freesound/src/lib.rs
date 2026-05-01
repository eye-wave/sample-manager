use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use plugin_base as host;

#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: u32) -> *mut u8 {
    let mut buf = Vec::with_capacity(len as usize);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn free(ptr: *mut u8, len: u32) {
    unsafe { drop(Vec::from_raw_parts(ptr, len as usize, len as usize)) }
}

static PLUGIN_NAME: &str = "Freesound";
static PLUGIN_DESCRIPTION: &str = "Search freesound.org for audio samples";

#[unsafe(no_mangle)]
pub static plugin_name_ptr: u32 = 0;
#[unsafe(no_mangle)]
pub static plugin_name_len: u32 = PLUGIN_NAME.len() as u32;
#[unsafe(no_mangle)]
pub static plugin_description_ptr: u32 = 0;
#[unsafe(no_mangle)]
pub static plugin_description_len: u32 = PLUGIN_DESCRIPTION.len() as u32;

#[derive(Deserialize)]
struct FreesoundResponse {
    results: Vec<FreesoundSound>,
}

#[derive(Deserialize)]
struct FreesoundSound {
    id: u64,
    name: String,
    tags: Vec<String>,
    duration: f32,
    #[serde(rename = "previews")]
    previews: FreesoundPreviews,
    samplerate: Option<u32>,
    description: Option<String>,
    username: String,
    license: String,
}

#[derive(Deserialize)]
struct FreesoundPreviews {
    #[serde(rename = "preview-hq-mp3")]
    hq_mp3: String,
}

#[derive(Deserialize)]
struct SearchRequest {
    query: String,
    tags: Vec<String>,
    limit: u32,
}

#[derive(Serialize)]
struct SampleResult {
    id: String,
    name: String,
    uri: String,
    tags: Vec<String>,
    duration: Option<f32>,
    sample_rate: Option<u32>,
    metadata: HashMap<String, String>,
    source_plugin: String,
}

#[unsafe(no_mangle)]
pub extern "C" fn search(req_ptr: u32, req_len: u32) -> u32 {
    let req_bytes = unsafe { std::slice::from_raw_parts(req_ptr as *const u8, req_len as usize) };

    let results = match do_search(req_bytes) {
        Ok(r) => r,
        Err(e) => {
            host::host_log(&format!("search error: {e}"));
            vec![]
        }
    };

    write_response(&results)
}

fn do_search(req_bytes: &[u8]) -> Result<Vec<SampleResult>, &'static str> {
    let req: SearchRequest = serde_json::from_slice(req_bytes).map_err(|_| "bad request json")?;

    let api_key = host::host_secret_get("api_key")
        .and_then(|b| String::from_utf8(b).ok())
        .ok_or("no api_key in secret storage — configure it first")?;

    let tag_filter = if req.tags.is_empty() {
        String::new()
    } else {
        format!(" tag:{}", req.tags.join(" tag:"))
    };

    let url = format!(
        "https://api.freesound.org/v2/search/text/?query={}{}&page_size={}&fields=id,name,tags,duration,previews,samplerate,description,username,license",
        urlencoding(req.query.trim()),
        urlencoding(&tag_filter),
        req.limit.min(150),
    );

    let body = host::host_http_fetch(&url, &[("Authorization", &format!("Token {api_key}"))])
        .ok_or("http_fetch failed")?;

    let response: FreesoundResponse =
        serde_json::from_slice(&body).map_err(|_| "failed to parse freesound response")?;

    Ok(response
        .results
        .into_iter()
        .map(|s| {
            let mut metadata = HashMap::new();
            metadata.insert("username".into(), s.username);
            metadata.insert("license".into(), s.license);
            if let Some(desc) = s.description {
                metadata.insert("description".into(), desc);
            }

            SampleResult {
                id: format!("freesound:{}", s.id),
                name: s.name,
                uri: s.previews.hq_mp3,
                tags: s.tags,
                duration: Some(s.duration),
                sample_rate: s.samplerate,
                metadata,
                source_plugin: String::new(),
            }
        })
        .collect())
}

fn write_response(results: &[SampleResult]) -> u32 {
    let json = serde_json::to_vec(results).unwrap_or_else(|_| b"[]".to_vec());
    let total = 4 + json.len();

    let mut buf = Vec::with_capacity(total);
    buf.extend_from_slice(&(json.len() as u32).to_le_bytes());
    buf.extend_from_slice(&json);

    let ptr = buf.as_ptr() as u32;
    std::mem::forget(buf);
    ptr
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                vec![c]
            }
            c => format!("%{:02X}", c as u32).chars().collect(),
        })
        .collect()
}
