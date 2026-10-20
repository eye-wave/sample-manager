//! Splice: the samples splice.com's search finds (their mp3 previews, which are scrambled: `unpack`
//! undoes it when the app has downloaded one). It asks the same GraphQL endpoint the website does (`query.graphql`
//! is the website's query), with no account. Remembering what was found is the host's job (it
//! caches every hit and fuzzy-matches the query against that too, offline included).
mod scramble;

wit_bindgen::generate!({ path: "../../wit", world: "plugin" });

use bx::search::host;
use exports::bx::search::search::{Guest, Hit, Info};
use serde_json::Value as Json;

const ENDPOINT: &str = "https://surfaces-graphql.splice.com/graphql";

/// The body of the POST that searches `query`.
fn request_body(q: &str) -> String {
    const Q1: &str = r#"{"operationName":"SamplesSearch","variables":{"order":"DESC","sort":"popularity","limit":10,"page":1,"tags":[],"key":null,"chord_type":null,"bpm":null,"min_bpm":null,"max_bpm":null,"asset_category_slug":null,"random_seed":null,"attributes":[],"filepath":""#;
    const Q2: &str = r#"","query":""#;
    const Q3: &str = r#"","ac_uuid":null},"extensions":{"clientLibrary":{"name":"@apollo/client","version":"4.2.10"}},"query":"query SamplesSearch($attributes: [AssetAttributeSlug!], $parent_asset_uuid: GUID, $query: String, $order: SortOrder = DESC, $sort: AssetSortType = popularity, $random_seed: String, $tags: [ID], $key: String, $chord_type: String, $bpm: String, $min_bpm: Int, $max_bpm: Int, $limit: Int = 50, $asset_category_slug: AssetCategorySlug, $page: Int = 1, $ac_uuid: String, $parent_asset_type: AssetTypeSlug, $licensed: Boolean, $liked: Boolean, $filepath: String, $query_strategy: AssetSearchQueryStrategy, $popularity_weight: Float) {\n  assetsSearch(\n    filter: {attributes: $attributes, legacy: true, published: true, asset_type_slug: sample, query: $query, filepath: $filepath, tag_ids: $tags, key: $key, chord_type: $chord_type, bpm: $bpm, min_bpm: $min_bpm, max_bpm: $max_bpm, asset_category_slug: $asset_category_slug, ac_uuid: $ac_uuid, licensed: $licensed, liked: $liked, query_strategy: $query_strategy, popularity_weight: $popularity_weight}\n    children: {parent_asset_uuid: $parent_asset_uuid}\n    pagination: {page: $page, limit: $limit}\n    sort: {sort: $sort, order: $order, random_seed: $random_seed}\n    legacy: {parent_asset_type: $parent_asset_type}\n  ) {\n    ...assetDetails\n    __typename\n  }\n}\n\nfragment assetDetails on AssetPage {\n  ...assetPageItems\n  ...assetTagSummaries\n  pagination_metadata {\n    currentPage\n    totalPages\n    __typename\n  }\n  response_metadata {\n    records\n    __typename\n  }\n  __typename\n}\n\nfragment assetPageItems on AssetPage {\n  items {\n    ... on IAsset {\n      asset_type_slug\n      liked\n      licensed\n      asset_prices {\n        amount\n        currency\n        __typename\n      }\n      uuid\n      name\n      tags {\n        uuid\n        label\n        __typename\n      }\n      files {\n        uuid\n        name\n        hash\n        path\n        asset_file_type_slug\n        url\n        __typename\n      }\n      __typename\n    }\n    ... on IAssetChild {\n      parents(filter: {asset_type_slug: pack}) {\n        items {\n          ... on PackAsset {\n            permalink_slug\n            permalink_base_url\n            uuid\n            name\n            provider {\n              permalink_slug\n              __typename\n            }\n            files {\n              uuid\n              path\n              asset_file_type_slug\n              url\n              __typename\n            }\n            __typename\n          }\n          __typename\n        }\n        __typename\n      }\n      __typename\n    }\n    ... on SampleAsset {\n      bpm\n      chord_type\n      key\n      duration\n      uuid\n      name\n      asset_category_slug\n      catalog_uuid\n      legacy_uuid\n      attributes\n      __typename\n    }\n    ... on GeneratedSampleAsset {\n      bpm\n      chord_type\n      key\n      duration\n      uuid\n      name\n      asset_category_slug\n      catalog_uuid\n      attributes\n      created_at\n      sampleParent: parents(filter: {asset_type_slug: sample}) {\n        items {\n          ... on SampleAsset {\n            catalog_uuid\n            parents(filter: {asset_type_slug: pack}) {\n              items {\n                ... on PackAsset {\n                  uuid\n                  name\n                  permalink_slug\n                  permalink_base_url\n                  files {\n                    uuid\n                    path\n                    asset_file_type_slug\n                    url\n                    __typename\n                  }\n                  __typename\n                }\n                __typename\n              }\n              __typename\n            }\n            __typename\n          }\n          __typename\n        }\n        __typename\n      }\n      __typename\n    }\n    ... on PresetAsset {\n      uuid\n      name\n      catalog_uuid\n      attributes\n      asset_devices {\n        uuid\n        device {\n          name\n          uuid\n          minimum_device_version\n          __typename\n        }\n        __typename\n      }\n      __typename\n    }\n    ... on PackAsset {\n      uuid\n      name\n      catalog_uuid\n      provider {\n        name\n        permalink_slug\n        __typename\n      }\n      provider_uuid\n      uuid\n      permalink_slug\n      permalink_base_url\n      main_genre\n      __typename\n    }\n    ... on MidiAsset {\n      catalog_uuid\n      description\n      key\n      bpm\n      __typename\n    }\n    __typename\n  }\n  __typename\n}\n\nfragment assetTagSummaries on AssetPage {\n  tag_summary {\n    count\n    tag {\n      uuid\n      label\n      taxonomy {\n        uuid\n        name\n        __typename\n      }\n      __typename\n    }\n    __typename\n  }\n  __typename\n}"}"#;

    const SIZE: usize = combined_len(&[Q1, Q2, Q3]);

    let mut body = String::with_capacity(SIZE + q.len());

    body.push_str(Q1);
    body.push_str(q);
    body.push_str(Q2);
    body.push_str(q);
    body.push_str(Q3);

    body
}

const fn combined_len(strings: &[&str]) -> usize {
    let mut total = 0;
    let mut i = 0;

    while i < strings.len() {
        total += strings[i].len();
        i += 1;
    }

    total
}

/// `Pack/One-Shots/Kicks/Kick_Power.wav` -> `Kick_Power`
fn title(name: &str) -> &str {
    let file = name.rsplit('/').next().unwrap_or(name);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

/// The previews in an answer; samples without one are left out.
fn parse(body: &[u8]) -> Result<Vec<Hit>, String> {
    let v: Json = serde_json::from_slice(body).map_err(|e| format!("not JSON: {e}"))?;
    if let Some(errors) = v.get("errors").and_then(Json::as_array) {
        let first = errors
            .first()
            .and_then(|e| e.get("message"))
            .and_then(Json::as_str)
            .unwrap_or("unknown error");
        return Err(format!("Splice: {first}"));
    }
    let items = v
        .pointer("/data/assetsSearch/items")
        .and_then(Json::as_array)
        .ok_or("no samples in the answer")?;
    Ok(items
        .iter()
        .filter_map(|i| {
            let name = i.get("name")?.as_str()?;
            let url = i
                .get("files")?
                .as_array()?
                .iter()
                .find(|f| {
                    f.get("asset_file_type_slug").and_then(Json::as_str) == Some("preview_mp3")
                })?
                .get("url")?
                .as_str()
                .filter(|u| !u.is_empty())?;
            Some(Hit {
                title: title(name).into(),
                url: url.into(),
                ext: Some("mp3".into()),
                unpack: true,
                skip_frames: scramble::MP3_START_DELAY,
            })
        })
        .collect())
}

/// What Splice finds for `query`.
fn from_network(query: &str) -> Result<Vec<Hit>, String> {
    let headers = [
        ("content-type", "application/json"),
        (
            "accept",
            "application/graphql-response+json,application/json;q=0.9",
        ),
        ("origin", "https://splice.com"),
        ("referer", "https://splice.com/"),
        ("accept-language", "en-US,en"),
        ("alt-used", "surfaces-graphql.splice.com"),
        ("sec-fetch-dest", "empty"),
        ("sec-fetch-mode", "cors"),
        ("sec-fetch-site", "same-site"),
        ("priority", "u=4"),
        ("pragma", "no-cache"),
        ("cache-control", "no-cache"),
    ]
    .map(|(k, v)| (k.to_string(), v.to_string()));
    let r = host::http_request("POST", ENDPOINT, &headers, request_body(query).as_bytes())?;
    if r.status >= 400 {
        return Err(format!(
            "Splice answered HTTP {}: {}",
            r.status,
            String::from_utf8_lossy(&r.body)
                .chars()
                .take(300)
                .collect::<String>()
        ));
    }
    parse(&r.body)
}

struct Splice;

impl Guest for Splice {
    fn describe() -> Info {
        Info {
            name: "Splice".into(),
            description: "Samples from splice.com".into(),
        }
    }

    fn unpack(_url: String, data: Vec<u8>) -> Result<Vec<u8>, String> {
        scramble::unscramble(data)
    }

    fn search(query: String) -> Result<Vec<Hit>, String> {
        from_network(&query)
    }
}

export!(Splice);
