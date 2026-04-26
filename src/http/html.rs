use std::borrow::Cow;

use wry::http::Response;

use crate::state::config::Theme;

pub fn html_handler(theme: Theme) -> Response<Cow<'static, [u8]>> {
    const HTML: &str = include_str!("../../client/dist/index.html");

    let html = HTML
        .replace(
            "<theme></theme>",
            &format!("<style>{}</style>", theme.to_css()),
        )
        .into_bytes();

    Response::builder()
        .status(200)
        .header("Content-Type", "text/html;charset=utf-8")
        .body(Cow::Owned(html))
        .unwrap()
}
