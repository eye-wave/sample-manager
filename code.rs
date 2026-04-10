#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
use crate::window::App;
mod commands {
    use std::{borrow::Cow, sync::Arc};
    use crate::commands::fs::IPC_FS;
    mod fs {
        use crate::ipc_commands;
        use std::borrow::Cow;
        use std::sync::Arc;
        fn open_folder(
            _r: &str,
            _w: &Arc<tao::window::Window>,
        ) -> Option<std::borrow::Cow<'static, [u8]>> {
            let folder = tinyfiledialogs::select_folder_dialog("Select folder", "");
            folder.map(|f| Cow::Owned(f.into_bytes()))
        }
        fn search_path(
            path: &str,
            _w: &Arc<tao::window::Window>,
        ) -> Option<std::borrow::Cow<'static, [u8]>> {
            let files: Vec<_> = std::fs::read_dir(path)
                .ok()?
                .filter_map(Result::ok)
                .filter_map(|e| {
                    let is_dir = e.path().is_dir() as u8;
                    e.path()
                        .strip_prefix(path)
                        .ok()
                        .map(|p| is_dir.to_string() + &p.display().to_string())
                })
                .collect();
            if files.is_empty() {
                None
            } else {
                Some(Cow::Owned(files.join("\n").into_bytes()))
            }
        }
        pub(super) static IPC_FS: &[&dyn crate::commands::IPCCommand] = &[
            &SearchPath,
            &OpenFolder,
        ];
        pub struct SearchPath;
        impl crate::commands::IPCCommand for SearchPath {
            fn name(&self) -> &'static str {
                "search_path"
            }
            fn respond(
                &self,
                req: &str,
                window_handle: &Arc<tao::window::Window>,
            ) -> Option<std::borrow::Cow<'static, [u8]>> {
                search_path(req, window_handle)
            }
        }
        pub struct OpenFolder;
        impl crate::commands::IPCCommand for OpenFolder {
            fn name(&self) -> &'static str {
                "open_folder"
            }
            fn respond(
                &self,
                req: &str,
                window_handle: &Arc<tao::window::Window>,
            ) -> Option<std::borrow::Cow<'static, [u8]>> {
                open_folder(req, window_handle)
            }
        }
    }
    mod window {
        use crate::ipc_commands;
        use std::sync::Arc;
        fn close_window(
            _req: &str,
            _w: &Arc<tao::window::Window>,
        ) -> Option<std::borrow::Cow<'static, [u8]>> {
            std::process::exit(0);
        }
        fn minimize_window(
            _req: &str,
            w: &Arc<tao::window::Window>,
        ) -> Option<std::borrow::Cow<'static, [u8]>> {
            w.set_minimized(true);
            None
        }
        fn maximize_window(
            _req: &str,
            w: &Arc<tao::window::Window>,
        ) -> Option<std::borrow::Cow<'static, [u8]>> {
            w.set_maximized(!w.is_maximized());
            None
        }
        fn drag_window(
            _req: &str,
            w: &Arc<tao::window::Window>,
        ) -> Option<std::borrow::Cow<'static, [u8]>> {
            w.drag_window().ok();
            None
        }
        pub(super) static IPC_WINDOW: &[&dyn crate::commands::IPCCommand] = &[
            &CloseWindow,
            &MinimizeWindow,
            &MaximizeWindow,
            &DragWindow,
        ];
        pub struct CloseWindow;
        impl crate::commands::IPCCommand for CloseWindow {
            fn name(&self) -> &'static str {
                "close_window"
            }
            fn respond(
                &self,
                req: &str,
                window_handle: &Arc<tao::window::Window>,
            ) -> Option<std::borrow::Cow<'static, [u8]>> {
                close_window(req, window_handle)
            }
        }
        pub struct MinimizeWindow;
        impl crate::commands::IPCCommand for MinimizeWindow {
            fn name(&self) -> &'static str {
                "minimize_window"
            }
            fn respond(
                &self,
                req: &str,
                window_handle: &Arc<tao::window::Window>,
            ) -> Option<std::borrow::Cow<'static, [u8]>> {
                minimize_window(req, window_handle)
            }
        }
        pub struct MaximizeWindow;
        impl crate::commands::IPCCommand for MaximizeWindow {
            fn name(&self) -> &'static str {
                "maximize_window"
            }
            fn respond(
                &self,
                req: &str,
                window_handle: &Arc<tao::window::Window>,
            ) -> Option<std::borrow::Cow<'static, [u8]>> {
                maximize_window(req, window_handle)
            }
        }
        pub struct DragWindow;
        impl crate::commands::IPCCommand for DragWindow {
            fn name(&self) -> &'static str {
                "drag_window"
            }
            fn respond(
                &self,
                req: &str,
                window_handle: &Arc<tao::window::Window>,
            ) -> Option<std::borrow::Cow<'static, [u8]>> {
                drag_window(req, window_handle)
            }
        }
    }
    pub(super) trait IPCCommand: Send + Sync {
        fn name(&self) -> &'static str;
        fn respond(
            &self,
            req: &str,
            window_handle: &Arc<tao::window::Window>,
        ) -> Option<Cow<'static, [u8]>>;
        fn is_this(&self, req: &str) -> bool {
            req.starts_with(self.name())
        }
        fn strip_name<'a>(&self, req: &'a str) -> Option<(u32, &'a str)> {
            let mut parts = req.splitn(3, ':');
            let _fn_name = parts.next()?;
            let id_str = parts.next()?;
            let payload = parts.next().unwrap_or("");
            let id = id_str.parse::<u32>().ok()?;
            Some((id, payload))
        }
    }
    pub fn commands_iter<'a>() -> impl Iterator<Item = &'a dyn IPCCommand> {
        use crate::commands::window::IPC_WINDOW;
        IPC_WINDOW.iter().chain(IPC_FS.iter()).copied()
    }
}
mod event {
    use std::{borrow::Cow, rc::Rc, sync::Arc};
    use tao::{event_loop::EventLoopBuilder, window::Window};
    use wry::WebView;
    use crate::commands::commands_iter;
    pub struct EventSystem {
        event_loop: EventLoopProxy,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for EventSystem {
        #[inline]
        fn clone(&self) -> EventSystem {
            EventSystem {
                event_loop: ::core::clone::Clone::clone(&self.event_loop),
            }
        }
    }
    pub type EventLoop = tao::event_loop::EventLoop<LoopEvent>;
    pub type EventLoopProxy = tao::event_loop::EventLoopProxy<LoopEvent>;
    pub(super) enum LoopEvent {
        JS(u32, Cow<'static, [u8]>),
    }
    impl EventSystem {
        pub fn build() -> (EventRunner, Self) {
            let event_loop = EventLoopBuilder::<LoopEvent>::with_user_event().build();
            let proxy = event_loop.create_proxy();
            (EventRunner { event_loop }, Self { event_loop: proxy })
        }
        pub fn receive(
            &self,
            req: wry::http::Request<String>,
            window_handle: Arc<Window>,
        ) {
            let body = req.body();
            for cmd in commands_iter() {
                if !cmd.is_this(body) {
                    continue;
                }
                if let Some((call_id, body)) = cmd.strip_name(body)
                    && let Some(bytes) = cmd.respond(body, &window_handle)
                {
                    self.send(call_id, bytes).ok();
                }
            }
        }
        pub fn send(
            &self,
            call_id: u32,
            message: Cow<'static, [u8]>,
        ) -> Result<(), tao::event_loop::EventLoopClosed<LoopEvent>> {
            self.event_loop.send_event(LoopEvent::JS(call_id, message))
        }
    }
    pub struct EventRunner {
        event_loop: EventLoop,
    }
    impl EventRunner {
        pub(super) fn inner(&self) -> &EventLoop {
            &self.event_loop
        }
        pub fn run(self, webview: &Rc<WebView>) {
            use tao::event::{Event, WindowEvent};
            use tao::event_loop::ControlFlow;
            let webview = webview.clone();
            self.event_loop
                .run(move |event, _, control_flow| {
                    *control_flow = ControlFlow::Wait;
                    if let Event::WindowEvent { event, .. } = &event
                        && event == &WindowEvent::CloseRequested
                    {
                        std::process::exit(0);
                    }
                    if let Event::UserEvent(LoopEvent::JS(call_id, bytes)) = event {
                        let payload = unsafe { str::from_utf8_unchecked(&bytes) };
                        let code = ::alloc::__export::must_use({
                            ::alloc::fmt::format(
                                format_args!(
                                    "_r({1},`{0}`)",
                                    payload.replace("`", "\\`"),
                                    call_id,
                                ),
                            )
                        });
                        webview.evaluate_script(&code).ok();
                    }
                });
        }
    }
}
mod http {
    use std::borrow::Cow;
    use rust_embed::RustEmbed;
    use wry::http::{Request, Response};
    #[folder = "client"]
    struct Assets;
    impl Assets {
        fn matcher() -> rust_embed::utils::PathMatcher {
            const INCLUDES: &[&str] = &[];
            const EXCLUDES: &[&str] = &[];
            static PATH_MATCHER: ::std::sync::OnceLock<rust_embed::utils::PathMatcher> = ::std::sync::OnceLock::new();
            PATH_MATCHER
                .get_or_init(|| rust_embed::utils::PathMatcher::new(INCLUDES, EXCLUDES))
                .clone()
        }
        /// Get an embedded file and its metadata.
        pub fn get(file_path: &str) -> ::std::option::Option<rust_embed::EmbeddedFile> {
            let rel_file_path = file_path.replace("\\", "/");
            let file_path = ::std::path::Path::new(
                    "/home/eyewave/Documents/Rust/sample-manager/client",
                )
                .join(&rel_file_path);
            let canonical_file_path = file_path.canonicalize().ok()?;
            if !canonical_file_path
                .starts_with("/home/eyewave/Documents/Rust/sample-manager/client")
            {
                let metadata = ::std::fs::symlink_metadata(&file_path).ok()?;
                if !metadata.is_symlink() {
                    return ::std::option::Option::None;
                }
            }
            let path_matcher = Self::matcher();
            if path_matcher.is_path_included(&rel_file_path) {
                rust_embed::utils::read_file_from_fs(&canonical_file_path).ok()
            } else {
                ::std::option::Option::None
            }
        }
        /// Iterates over the file paths in the folder.
        pub fn iter() -> impl ::std::iter::Iterator<
            Item = ::std::borrow::Cow<'static, str>,
        > + 'static {
            use ::std::path::Path;
            rust_embed::utils::get_files(
                    ::std::string::String::from(
                        "/home/eyewave/Documents/Rust/sample-manager/client",
                    ),
                    Self::matcher(),
                )
                .map(|e| ::std::borrow::Cow::from(e.rel_path))
        }
    }
    impl rust_embed::RustEmbed for Assets {
        fn get(file_path: &str) -> ::std::option::Option<rust_embed::EmbeddedFile> {
            Assets::get(file_path)
        }
        fn iter() -> rust_embed::Filenames {
            rust_embed::Filenames::Dynamic(::std::boxed::Box::new(Assets::iter()))
        }
    }
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
}
mod window {
    use std::{rc::Rc, sync::Arc};
    use tao::window::Window;
    use wry::{WebView, WebViewBuilder};
    use crate::http::request_handler;
    use super::event::{EventRunner, EventSystem};
    pub struct App {
        _webview: WebView,
        runner: EventRunner,
    }
    impl App {
        pub fn build() -> Self {
            let (event_runner, event_system) = EventSystem::build();
            let event_handle = Arc::new(event_system);
            let window = tao::window::WindowBuilder::new()
                .with_title("My app")
                .with_decorations(false)
                .with_inner_size(tao::dpi::LogicalSize::new(800.0, 600.0))
                .build(event_runner.inner())
                .unwrap();
            const PROTOCOL: &str = "sampols";
            let window_handle = Arc::new(window);
            let window_handle_cloned = window_handle.clone();
            let webview = WebViewBuilder::new()
                .with_custom_protocol(PROTOCOL.into(), request_handler)
                .with_ipc_handler(move |req| {
                    event_handle.receive(req, window_handle_cloned.clone())
                })
                .with_url(PROTOCOL.to_string() + "://_/app.html")
                .with_devtools(true);
            let _webview = finish_webview(window_handle.clone(), webview);
            App {
                _webview,
                runner: event_runner,
            }
        }
        pub fn run(self) {
            let webview = Rc::new(self._webview);
            self.runner.run(&webview);
        }
    }
    fn finish_webview(window: Arc<Window>, webview: WebViewBuilder<'_>) -> wry::WebView {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().expect("Failed to get vbox");
        webview.build_gtk(vbox).expect("Failed to build WebView")
    }
}
fn main() {
    App::build().run();
}
