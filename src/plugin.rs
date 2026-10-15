//! The CLAP plugin: no audio, no params, just the editor window (a child window under the host's
//! parent, rendered by render.rs on its own thread). The window is resizable: the resize handle
//! drawn in the corner is dragged with the mouse, which is polled on a host timer (the host owns
//! the event loop), and the new size is requested from the host.
use crate::icons::in_grip;
use crate::native::{API, EXTS, Native};
use crate::render::{H, MIN_H, MIN_W, W, pack_size, render};
use clack_extensions::gui::*;
use clack_extensions::timer::*;
use clack_plugin::plugin::features::UTILITY;
use clack_plugin::prelude::*;
use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::thread::JoinHandle;

pub struct Editor;
pub struct Shared;
impl PluginShared<'_> for Shared {}

impl Plugin for Editor {
    type AudioProcessor<'a> = Idle;
    type Shared<'a> = Shared;
    type MainThread<'a> = Main<'a>;

    fn declare_extensions(b: &mut PluginExtensions<Self>, _: Option<&Shared>) {
        b.register::<PluginGui>().register::<PluginTimer>();
    }
}

impl DefaultPluginFactory for Editor {
    fn get_descriptor() -> PluginDescriptor {
        PluginDescriptor::new("org.example.sample-manager", "Sample Manager")
            .with_features([UTILITY])
    }

    fn new_shared(_: HostSharedHandle<'_>) -> Result<Shared, PluginError> {
        Ok(Shared)
    }

    fn new_main_thread<'a>(
        host: HostMainThreadHandle<'a>,
        _: &'a Shared,
    ) -> Result<Main<'a>, PluginError> {
        Ok(Main {
            host,
            size: Cell::new((W, H)),
            open: RefCell::new(None),
        })
    }
}

/// Does nothing: the hosts still need an audio processor to instantiate the plugin.
pub struct Idle;

impl<'a> PluginAudioProcessor<'a, Shared, Main<'a>> for Idle {
    fn activate(
        _: HostAudioProcessorHandle<'a>,
        _: &Main<'a>,
        _: &'a Shared,
        _: PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        Ok(Idle)
    }

    fn process(&mut self, _: Process, _: Audio, _: Events) -> Result<ProcessStatus, PluginError> {
        Ok(ProcessStatus::Sleep)
    }
}

/// The open editor: exists between `set_parent` and `destroy`.
struct Open {
    native: Arc<Native>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    /// Only here because `render` wants it; X11 and Win32 report the window size themselves.
    size: Arc<AtomicU64>,
    /// Running while shown, to poll the mouse.
    timer: Option<TimerId>,
    /// A resize drag: the mouse position on the screen and the window size when it started.
    drag: Option<(i32, i32, u32, u32)>,
    was_down: bool,
}

pub struct Main<'a> {
    host: HostMainThreadHandle<'a>,
    /// The current window size, kept even while there is no window.
    size: Cell<(u32, u32)>,
    open: RefCell<Option<Open>>,
}

impl<'a> PluginMainThread<'a, Shared> for Main<'a> {}

impl Main<'_> {
    fn stop_render(&self) {
        if let Some(o) = self.open.borrow_mut().as_mut() {
            o.stop.store(true, Relaxed);
            if let Some(t) = o.thread.take() {
                let _ = t.join();
            }
        }
    }

    fn stop_timer(&self) {
        if let Some(o) = self.open.borrow_mut().as_mut()
            && let Some(id) = o.timer.take()
            && let Some(t) = self.host.get_extension::<HostTimer>()
        {
            let _ = t.unregister_timer(&self.host, id);
        }
    }

    /// Makes the window `w` x `h`; the renderer follows by itself.
    fn apply_size(&self, w: u32, h: u32) {
        self.size.set((w, h));
        if let Some(o) = self.open.borrow().as_ref() {
            o.native.resize(w, h);
            o.size.store(pack_size(w, h), Relaxed);
        }
    }
}

impl PluginTimerImpl for Main<'_> {
    fn on_timer(&self, _: TimerId) {
        let (w, h) = self.size.get();
        let target = {
            let mut g = self.open.borrow_mut();
            let Some(o) = g.as_mut() else { return };
            let p = o.native.pointer();
            // a drag starts with a press on the handle (not a button already held while moving in)
            if o.drag.is_none() && p.down && !o.was_down && in_grip(p.x, p.y, w, h) {
                o.drag = Some((p.sx, p.sy, w, h));
            }
            if !p.down {
                o.drag = None;
            }
            o.was_down = p.down;
            o.drag.map(|(sx, sy, w0, h0)| {
                (
                    (w0 as i32 + p.sx - sx).max(MIN_W as i32) as u32,
                    (h0 as i32 + p.sy - sy).max(MIN_H as i32) as u32,
                )
            })
        };
        if let Some((nw, nh)) = target
            && (nw, nh) != (w, h)
            // the host does not have to call set_size() after accepting, so apply it ourselves
            && let Some(gui) = self.host.get_extension::<HostGui>()
            && gui.request_resize(self.host.as_shared(), nw, nh).is_ok()
        {
            self.apply_size(nw, nh);
        }
    }
}

impl PluginGuiImpl for Main<'_> {
    fn is_api_supported(&self, c: GuiConfiguration) -> bool {
        c.api_type == API && !c.is_floating
    }

    fn get_preferred_api(&self) -> Option<GuiConfiguration<'_>> {
        Some(GuiConfiguration {
            api_type: API,
            is_floating: false,
        })
    }

    // The window is created in set_parent(): Win32 child windows need their parent up front.
    fn create(&self, _: GuiConfiguration) -> Result<(), PluginError> {
        Ok(())
    }

    fn destroy(&self) {
        self.stop_timer();
        self.stop_render();
        self.open.borrow_mut().take(); // drops Native -> destroys the window
    }

    fn set_scale(&self, _: f64) -> Result<(), PluginError> {
        Ok(())
    }

    fn get_size(&self) -> Option<GuiSize> {
        let (width, height) = self.size.get();
        Some(GuiSize { width, height })
    }

    fn can_resize(&self) -> bool {
        true
    }

    fn adjust_size(&self, s: GuiSize) -> Option<GuiSize> {
        Some(GuiSize {
            width: s.width.max(MIN_W),
            height: s.height.max(MIN_H),
        })
    }

    fn set_size(&self, s: GuiSize) -> Result<(), PluginError> {
        self.apply_size(s.width, s.height);
        Ok(())
    }

    fn set_parent(&self, window: Window) -> Result<(), PluginError> {
        self.destroy();
        let (w, h) = self.size.get();
        *self.open.borrow_mut() = Some(Open {
            native: Arc::new(Native::new(&window, w, h)?),
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
            size: Arc::new(AtomicU64::new(pack_size(w, h))),
            timer: None,
            drag: None,
            was_down: false,
        });
        Ok(())
    }

    fn set_transient(&self, _: Window) -> Result<(), PluginError> {
        Ok(())
    }

    fn show(&self) -> Result<(), PluginError> {
        let mut g = self.open.borrow_mut();
        let o = g
            .as_mut()
            .ok_or(PluginError::Message("set_parent not called"))?;
        o.native.show()?;
        if o.thread.is_none() {
            o.stop.store(false, Relaxed);
            let (native, stop, size) = (o.native.clone(), o.stop.clone(), o.size.clone());
            o.thread = Some(std::thread::spawn(move || unsafe {
                render(
                    &EXTS.map(|e| e.as_ptr()),
                    |e, i| native.surface(e, i),
                    &size,
                    &stop,
                )
            }));
        }
        if o.timer.is_none()
            && let Some(t) = self.host.get_extension::<HostTimer>()
        {
            // 60 Hz is the render rate; hosts may slow it down, the handle then just feels laggier
            o.timer = t.register_timer(&self.host, 16).ok();
        }
        Ok(())
    }

    fn hide(&self) -> Result<(), PluginError> {
        self.stop_timer();
        self.stop_render();
        if let Some(o) = self.open.borrow_mut().as_mut() {
            o.drag = None;
            o.native.hide()?;
        }
        Ok(())
    }
}

clack_export_entry!(SinglePluginEntry<Editor>);
