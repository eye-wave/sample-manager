use std::borrow::Cow;

use rust_embed::RustEmbed;
use wry::http::{Request, Response};

#[derive(RustEmbed)]
#[folder = "client/dist"]
struct Assets;

fn mime_from_path(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("js") => "application/javascript",
        Some("css") => "text/css",
        Some("html") => "text/html",
        _ => "application/octet-stream",
    }
}

pub(super) fn request_handler(
    _webview_id: &str,
    req: Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    let path = &req.uri().path().to_string()[1..];

    if let Some(file) = Assets::get(path) {
        let mime = mime_from_path(path);

        Response::builder()
            .status(200)
            .header("Content-Type", mime)
            .body(Cow::Owned(file.data.into_owned()))
            .unwrap()
    } else {
        Response::builder()
            .status(404)
            .body(Cow::Borrowed(b"Not Found".as_ref()))
            .unwrap()
    }
}
