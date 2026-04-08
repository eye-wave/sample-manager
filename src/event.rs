use std::rc::Rc;

use tao::event_loop::EventLoopBuilder;
use wry::WebView;

#[derive(Clone)]
pub struct EventSystem {
    event_loop: EventLoopProxy,
}

pub type EventLoop = tao::event_loop::EventLoop<UserEvent>;
pub type EventLoopProxy = tao::event_loop::EventLoopProxy<UserEvent>;

pub(super) enum UserEvent {
    JS(String),
}

impl EventSystem {
    pub fn build() -> (EventRunner, Self) {
        let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
        let proxy = event_loop.create_proxy();

        (EventRunner { event_loop }, Self { event_loop: proxy })
    }

    pub fn receive(
        &self,
        req: wry::http::Request<String>,
    ) -> Result<(), tao::event_loop::EventLoopClosed<UserEvent>> {
        if req.body().as_str() == "ping" {
            self.send("pong!")?;
        }

        Ok(())
    }
    pub fn send(&self, message: &str) -> Result<(), tao::event_loop::EventLoopClosed<UserEvent>> {
        self.event_loop
            .send_event(UserEvent::JS(message.to_string()))
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

            if let Event::UserEvent(UserEvent::JS(code)) = event {
                webview.evaluate_script(&code).ok();
            }
        });
    }
}
