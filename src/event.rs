use std::{borrow::Cow, rc::Rc};

use tao::event_loop::EventLoopBuilder;
use wry::WebView;

mod fs;

pub(super) trait IPCCommand: Send + Sync {
    fn name(&self) -> &'static str;
    fn respond(&self, req: &str) -> Option<Cow<'static, [u8]>>;

    fn is_this(&self, req: &str) -> bool {
        req.starts_with(self.name())
    }

    fn strip_name<'a>(&self, req: &'a str) -> (u32, &'a str) {
        let num: String = req
            .chars()
            .skip(self.name().len() + 1)
            .take_while(|c| c.is_ascii_digit())
            .collect();

        let call_id: u32 = num.parse().unwrap_or(0);
        let idx = self.name().len() + num.len() + 2;

        (call_id, &req[idx..])
    }
}

static IPC_COMMANDS: &[&dyn IPCCommand] = &[&fs::SearchPath];

#[derive(Clone)]
pub struct EventSystem {
    event_loop: EventLoopProxy,
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

    pub fn receive(&self, req: wry::http::Request<String>) {
        let body = req.body();

        for cmd in IPC_COMMANDS {
            if !cmd.is_this(body) {
                continue;
            }

            let (call_id, body) = cmd.strip_name(body);

            if let Some(bytes) = cmd.respond(body) {
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
