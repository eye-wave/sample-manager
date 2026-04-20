use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use wry::http::{Request, Response, StatusCode, Uri};

pub fn thumbnail_handler(
    cache_path: Arc<Path>,
    req: Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    match try_read_file(&cache_path, req.uri()) {
        Some(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", "*")
            .header("Content-Type", "image/webp")
            .header("Content-Length", bytes.len().to_string())
            .body(Cow::Owned(bytes))
            .unwrap(),
        _ => Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Cow::Borrowed(b"invalid path".as_ref()))
            .unwrap(),
    }
}

fn try_read_file(base_path: &Path, uri: &Uri) -> Option<Vec<u8>> {
    let req_path = PathBuf::from(uri.path());
    let file_norm = req_path.file_name()?;

    let path = base_path.join(file_norm);

    fs::read(&path).ok()
}
