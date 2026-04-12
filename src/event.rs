use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use tao::{event_loop::EventLoopBuilder, window::Window};
use wry::WebView;

use crate::commands::IPCCommand;
use crate::ipc::{commands_iter, ipc_strip_name};
use crate::state::AppState;

pub struct EventSystem {
    event_loop: EventLoopProxy,
    app_state: Arc<RwLock<AppState>>,
    ipc_commands: HashMap<&'static str, &'static dyn IPCCommand>,
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

        let mut app_state = AppState::default();
        app_state.load().ok();

        (
            EventRunner { event_loop },
            Self {
                event_loop: proxy,
                app_state: Arc::new(RwLock::new(app_state)),
                ipc_commands: commands_iter().map(|cmd| (cmd.name(), cmd)).collect(),
            },
        )
    }

    pub fn receive(&self, req: wry::http::Request<String>, window_handle: Arc<Window>) {
        let body = req.body();

        if let Some((fn_name, call_id, payload)) = ipc_strip_name(body)
            && let Some(cmd) = self.ipc_commands.get(fn_name)
        {
            if let Some(bytes) = cmd.respond(payload, &window_handle, Arc::clone(&self.app_state)) {
                self.send(call_id, bytes).ok();
            } else {
                self.send_empty(call_id).ok();
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

    pub fn send_empty(
        &self,
        call_id: u32,
    ) -> Result<(), tao::event_loop::EventLoopClosed<LoopEvent>> {
        self.event_loop
            .send_event(LoopEvent::JS(call_id, Cow::Borrowed(&[])))
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

        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            if let Event::WindowEvent { event, .. } = &event
                && event == &WindowEvent::CloseRequested
            {
                std::process::exit(0);
            }

            if let Event::UserEvent(LoopEvent::JS(call_id, bytes)) = event {
                let payload = unsafe { str::from_utf8_unchecked(&bytes) };
                let code = format!("_r({call_id},`{}`)", payload.replace("`", "\\`"));

                webview.evaluate_script(&code).ok();
            }
        });
    }
}
