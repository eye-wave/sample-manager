use std::borrow::Cow;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, RwLock};

use tao::window::ResizeDirection;
use tao::{event_loop::EventLoopBuilder, window::Window};
use wry::WebView;

use crate::LogErrorExt;
use crate::ipc::{
    IPC_ID_BASE, IPCBody, IPCCommand, IPCMessage, IPCSenderUI, commands_iter, ipc_strip_cmd_id,
};
use crate::state::AppState;

pub struct EventSystem {
    pub webview_tx: IPCSenderUI,
    event_loop: EventLoopProxy,
    pub app_state: Arc<RwLock<AppState>>,
    ipc_commands: Vec<&'static dyn IPCCommand>,
}

pub struct EventRunner {
    webview_rx: mpsc::Receiver<IPCMessage>,
    event_loop: EventLoop,
    window_handle: Option<Arc<Window>>,
}

pub type EventLoop = tao::event_loop::EventLoop<LoopEvent>;
pub type EventLoopProxy = tao::event_loop::EventLoopProxy<LoopEvent>;

pub(super) enum LoopEvent {
    JS(u32, Cow<'static, [u8]>),
    Resize(ResizeDirection),
}

impl EventSystem {
    pub fn build() -> (EventRunner, Self) {
        let (tx, rx) = mpsc::channel();
        let tx = IPCSenderUI(tx);

        let event_loop = EventLoopBuilder::<LoopEvent>::with_user_event().build();
        let proxy = event_loop.create_proxy();

        let app_state = AppState::new(tx.clone());

        (
            EventRunner {
                event_loop,
                webview_rx: rx,
                window_handle: None,
            },
            Self {
                webview_tx: tx,
                event_loop: proxy,
                app_state: Arc::new(RwLock::new(app_state)),
                ipc_commands: commands_iter().collect(),
            },
        )
    }

    pub fn receive(&self, req: wry::http::Request<String>, window_handle: Arc<Window>) {
        let body = req.body();

        if let Some(dir_str) = body.strip_prefix("0:") {
            let dir = match dir_str {
                "0" => ResizeDirection::SouthEast,
                "1" => ResizeDirection::NorthEast,
                "2" => ResizeDirection::SouthWest,
                "3" => ResizeDirection::NorthWest,
                "4" => ResizeDirection::East,
                "5" => ResizeDirection::West,
                "6" => ResizeDirection::North,
                "7" => ResizeDirection::South,
                _ => return,
            };
            self.event_loop
                .send_event(LoopEvent::Resize(dir))
                .sure("Failed to send the 'resize' event");
            return;
        }

        if let Some((cmd_id, call_id, payload)) = ipc_strip_cmd_id(body)
            && let Some(cmd) = self.ipc_commands.get(cmd_id - IPC_ID_BASE)
        {
            let body = IPCBody {
                webview_sender: self.webview_tx.clone(),
                req: Arc::from(payload),
                window_handle: window_handle.clone(),
                app_state: self.app_state.clone(),
            };

            match cmd.respond(body) {
                Ok(bytes) => {
                    self.send(call_id, bytes)
                        .sure("Failed to respond to IPC command");
                }
                Err(err) => {
                    tracing::error!(kind="ipc", error = %err);
                    self.send_empty(call_id)
                        .sure("Failed to respond to IPC command");
                }
            };
        }
    }

    pub fn send(
        &self,
        call_id: u32,
        message: Cow<'static, [u8]>,
    ) -> Result<(), tao::event_loop::EventLoopClosed<LoopEvent>> {
        self.event_loop.send_event(LoopEvent::JS(call_id, message))
    }

    pub fn send_empty(
        &self,
        call_id: u32,
    ) -> Result<(), tao::event_loop::EventLoopClosed<LoopEvent>> {
        self.event_loop
            .send_event(LoopEvent::JS(call_id, Cow::Borrowed(&[])))
    }
}

fn escape_for_template_literal(s: &str) -> String {
    s.replace('\\', "\\\\").replace('`', "\\`")
}

impl EventRunner {
    pub(super) fn inner(&self) -> &EventLoop {
        &self.event_loop
    }

    pub fn attach_handle(&mut self, handle: &Arc<Window>) {
        self.window_handle = Some(handle.clone());
    }

    pub fn run(self, webview: &Rc<WebView>) {
        use tao::event::{Event, WindowEvent};
        use tao::event_loop::ControlFlow;

        let webview = webview.clone();

        self.event_loop.run(move |event, _, control_flow| {
            let window = self.window_handle.as_ref().unwrap();

            *control_flow = ControlFlow::Poll;

            while let Ok(msg) = self.webview_rx.try_recv() {
                let payload = escape_for_template_literal(&msg.payload);
                let code = format!("_s('{}',`{}`)", msg.id, payload);
                webview
                    .evaluate_script(&code)
                    .sure("Failed to evauate webview script");
            }

            match &event {
                Event::WindowEvent { event, .. } => {
                    if event == &WindowEvent::CloseRequested {
                        *control_flow = ControlFlow::Exit;
                    }
                }

                Event::UserEvent(LoopEvent::JS(call_id, bytes)) => {
                    if let Ok(payload_str) = str::from_utf8(bytes) {
                        let payload = escape_for_template_literal(payload_str);
                        let code = format!("_r({call_id},`{}`)", payload);
                        webview
                            .evaluate_script(&code)
                            .sure("Failed to evauate webview script");
                    }
                }
                Event::UserEvent(LoopEvent::Resize(dir)) => {
                    window
                        .drag_resize_window(*dir)
                        .sure("Failed to start resizing app window");
                }

                _ => {}
            }
        });
    }
}
