use std::sync::Arc;
use std::{path::Path, rc::Rc};

use tao::window::Window;
use wry::{WebView, WebViewBuilder};

use super::event::{EventRunner, EventSystem};

pub struct App {
    _webview: WebView,
    runner: EventRunner,
}

impl App {
    pub fn build() -> Self {
        let (event_runner, event_system) = EventSystem::build();
        let event_handle = Arc::new(event_system);

        let cache_path: Arc<Path> = {
            let guard = event_handle.app_state.read().unwrap();
            Arc::from(guard.cache_path.as_ref())
        };

        let window = tao::window::WindowBuilder::new()
            .with_title("My app")
            .with_decorations(false)
            .with_inner_size(tao::dpi::LogicalSize::new(920.0, 720.0))
            .build(event_runner.inner())
            .unwrap();

        let window_handle = Arc::new(window);
        let window_handle_cloned = window_handle.clone();

        let webview = if cfg!(debug_assertions) {
            WebViewBuilder::new()
                .with_url("http://localhost:5173/app")
                .with_devtools(true)
        } else {
            use crate::http::html_handler;

            let protocol = "sampols".to_string();

            WebViewBuilder::new()
                .with_custom_protocol(protocol.clone(), html_handler)
                .with_url(protocol + "://_")
                .with_devtools(true)
        }
        .with_custom_protocol("athumb".to_string(), move |_, req| {
            crate::http::thumbnail_handler(cache_path.clone(), req)
        })
        .with_ipc_handler(move |req| event_handle.receive(req, window_handle_cloned.clone()));

        let _webview = finish_webview(window_handle.clone(), webview);

        App {
            _webview,
            runner: event_runner,
        }
    }

    pub fn run(self) {
        self.runner.run(&Rc::new(self._webview));
    }
}

#[cfg(target_os = "linux")]
fn finish_webview(window: Arc<Window>, webview: WebViewBuilder<'_>) -> wry::WebView {
    use tao::platform::unix::WindowExtUnix;
    use wry::WebViewBuilderExtUnix;

    let vbox = window.default_vbox().expect("Failed to get vbox");
    webview.build_gtk(vbox).expect("Failed to build WebView")
}

#[cfg(not(target_os = "linux"))]
fn finish_webview(window: Arc<Window>, webview: WebViewBuilder<'_>) -> wry::WebView {
    webview.build(&window).expect("Failed to build WebView")
}
