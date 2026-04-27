use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};

use wry::http::{Request, Response, StatusCode, Uri};

use crate::state::{AppDirs, config::Theme};

pub fn app_handler(theme: Theme, req: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = req.uri().path();

    if path.starts_with("/thumb/") {
        return thumbnail_handler(&AppDirs::thumbnail_cache_path(), req.uri());
    }

    html_handler(theme)
}

fn html_handler(theme: Theme) -> Response<Cow<'static, [u8]>> {
    const HTML: &str = if cfg!(debug_assertions) {
        ""
    } else {
        include_str!("../client/dist/index.html")
    };

    let html = HTML
        .replace(
            "<theme></theme>",
            &format!("<style>{}</style>", theme.to_css()),
        )
        .into_bytes();

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html;charset=utf-8")
        .body(Cow::Owned(html))
        .unwrap()
}

fn thumbnail_handler(cache_path: &Path, uri: &Uri) -> Response<Cow<'static, [u8]>> {
    match try_read_file(cache_path, uri) {
        Some(bytes) => {
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "image/webp")
                .header("Content-Length", bytes.len().to_string());

            #[cfg(debug_assertions)]
            let response = response.header("Access-Control-Allow-Origin", "*");

            response.body(Cow::Owned(bytes)).unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Cow::Borrowed(b"not found".as_ref()))
            .unwrap(),
    }
}

fn try_read_file(base_path: &Path, uri: &Uri) -> Option<Vec<u8>> {
    let req_path = PathBuf::from(uri.path());
    let file_name = req_path.file_name()?;

    let path = base_path.join(file_name);
    fs::read(path).ok()
}
