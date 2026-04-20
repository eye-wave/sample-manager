use std::borrow::Cow;

use wry::http::{Request, Response};

pub fn html_handler(_: &str, _: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    const HTML: &[u8] = include_bytes!("../../client/dist/index.html");

    Response::builder()
        .status(200)
        .header("Content-Type", "text/html;charset=utf-8")
        .body(Cow::Borrowed(HTML))
        .unwrap()
}
