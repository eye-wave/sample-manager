use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use wry::http::{Request, Response, StatusCode};

pub(super) fn html_handler(_: &str, _: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    const HTML: &[u8] = include_bytes!("../client/dist/index.html");

    Response::builder()
        .status(200)
        .header("Content-Type", "text/html;charset=utf-8")
        .body(Cow::Borrowed(HTML))
        .unwrap()
}

pub(super) fn thumbnail_handler(
    cache_path: Arc<Path>,
    req: Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    let req_path = req.uri().path().trim_start_matches('/');

    if req_path.contains("..") {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Cow::Borrowed(b"invalid path".as_ref()))
            .unwrap();
    }

    let mut path = PathBuf::from(&*cache_path);
    path.push(req_path);
    path.set_extension("webp");

    match fs::read(&path) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", "*")
            .header("Content-Type", "image/webp")
            .header("Content-Length", bytes.len().to_string())
            .body(Cow::Owned(bytes))
            .unwrap(),
        Err(err) => {
            let status = if err.kind() == std::io::ErrorKind::NotFound {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            Response::builder()
                .status(status)
                .body(Cow::Borrowed(b"".as_ref()))
                .unwrap()
        }
    }
}
