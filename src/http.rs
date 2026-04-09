use std::borrow::Cow;

use rust_embed::RustEmbed;
use wry::http::{Request, Response};

#[derive(RustEmbed)]
#[folder = "client"]
struct Assets;

pub(super) fn request_handler(
    _webview_id: &str,
    req: Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    let path = &req.uri().path().to_string()[1..];

    if let Some(file) = Assets::get(path) {
        Response::builder()
            .status(200)
            .body(Cow::Owned(file.data.into_owned()))
            .unwrap()
    } else {
        Response::builder()
            .status(404)
            .body(Cow::Borrowed(b"Not Found".as_ref()))
            .unwrap()
    }
}
