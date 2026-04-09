use std::{rc::Rc, sync::Arc};

use tao::window::Window;
use wry::{WebView, WebViewBuilder};

use crate::http::request_handler;

use super::event::{EventRunner, EventSystem};

pub struct App {
    window: Window,
    _webview: WebView,
    runner: EventRunner,
}

impl App {
    pub fn build() -> Self {
        let (event_runner, event_system) = EventSystem::build();
        let event_handle = Arc::new(event_system);

        let window = tao::window::WindowBuilder::new()
            .with_title("My app")
            .with_inner_size(tao::dpi::LogicalSize::new(800.0, 600.0))
            .build(event_runner.inner())
            .unwrap();

        const PROTOCOL: &str = "sampols";

        let webview = WebViewBuilder::new()
            .with_custom_protocol(PROTOCOL.into(), request_handler)
            .with_ipc_handler(move |req| event_handle.receive(req))
            .with_url(PROTOCOL.to_string() + "://_/app.html")
            .with_devtools(cfg!(debug_assertions));

        let _webview = finish_webview(&window, webview);

        App {
            window,
            _webview,
            runner: event_runner,
        }
    }

    pub fn run(self) {
        let webview = Rc::new(self._webview);

        self.window.set_visible(true);
        self.runner.run(&webview);
    }
}

#[cfg(target_os = "linux")]
fn finish_webview(window: &Window, webview: WebViewBuilder<'_>) -> wry::WebView {
    use tao::platform::unix::WindowExtUnix;
    use wry::WebViewBuilderExtUnix;

    let vbox = window.default_vbox().expect("Failed to get vbox");
    webview.build_gtk(vbox).expect("Failed to build WebView")
}

#[cfg(not(target_os = "linux"))]
fn finish_webview(window: &Window, webview: WebViewBuilder<'_>) -> wry::WebView {
    webview.build(window).expect("Failed to build WebView")
}
