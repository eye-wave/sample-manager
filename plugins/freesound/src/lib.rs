use plugin_base::{json, prelude::*};

use std::collections::HashMap;

plugin_base::export_allocator!();

#[unsafe(no_mangle)]
pub extern "C" fn search(req_ptr: u32, req_len: u32) -> u32 {
    let req_bytes = unsafe { read_request(req_ptr, req_len) };

    let results = match do_search(req_bytes) {
        Ok(r) => r,
        Err(e) => {
            plugin_base::log(&format!("Freesound error: {e}"));
            vec![]
        }
    };

    let response_bytes = SampleResult::serialize_all(&results);
    write_response(&response_bytes)
}

fn do_search(req_bytes: &[u8]) -> Result<Vec<SampleResult>, &'static str> {
    let req = SearchRequest::from_bytes(req_bytes).ok_or("bad request json")?;

    let api_key =
        secret_get_str("api_key").ok_or("no api_key in secret storage — configure it first")?;

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

    let json_resp = Request::get(&url)
        .header("Authorization", &format!("Token {api_key}"))
        .send_json()
        .ok_or("http_fetch failed or returned invalid JSON")?;

    let results_array =
        json::get_array(&json_resp, "results").ok_or("invalid response: missing results array")?;

    let mut samples = Vec::with_capacity(results_array.len());

    for item in results_array {
        let id = json::get_f64(item, "id")
            .map(|n| n.to_string())
            .unwrap_or_default();
        let name = json::get_str(item, "name").unwrap_or("Unknown").to_owned();
        let tags = json::get_array(item, "tags")
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| match v {
                        JsonValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let previews = json::get_obj(item, "previews");
        let hq_mp3 = previews
            .and_then(|p| match p.get("preview-hq-mp3") {
                Some(JsonValue::String(s)) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();

        let mut metadata = HashMap::new();
        if let Some(u) = json::get_str(item, "username") {
            metadata.insert("username".into(), u.into());
        }
        if let Some(l) = json::get_str(item, "license") {
            metadata.insert("license".into(), l.into());
        }
        if let Some(d) = json::get_str(item, "description") {
            metadata.insert("description".into(), d.into());
        }

        samples.push(SampleResult {
            id: format!("freesound:{}", id),
            name,
            uri: hq_mp3,
            tags,
            duration: json::get_f64(item, "duration").map(|f| f as f32),
            sample_rate: json::get_f64(item, "samplerate").map(|f| f as u32),
            metadata,
        });
    }

    Ok(samples)
}
