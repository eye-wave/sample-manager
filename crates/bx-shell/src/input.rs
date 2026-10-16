//! Clicks and typing from a window, for the renderer to pick up (`gpu::Content::events`). The
//! window thread pushes, the render thread takes; pushing also makes it draw again.
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

/// A key that is not a character.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Key {
    Backspace,
    Delete,
    CtrlBackspace,
    CtrlDelete,
    CtrlA,
    Left,
    Right,
    Home,
    End,
    Enter,
    Escape,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Event {
    /// Left button pressed at x, y (px from the top left of the window).
    Click(i32, i32),
    /// Typed text, one character at a time (never a control character).
    Char(char),
    Key(Key),
    /// The wheel: this many rows down (negative: up).
    Scroll(i32),
    /// The left button went up (or was taken away), wherever the mouse is.
    Release,
}

pub struct Events {
    queue: Mutex<Vec<Event>>,
    dirty: AtomicBool,
}

impl Events {
    pub const fn new() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            dirty: AtomicBool::new(false),
        }
    }
    pub fn push(&self, e: Event) {
        self.queue.lock().unwrap().push(e);
        self.dirty.store(true, Relaxed);
    }
    /// Makes the next frame draw again without an event (something else changed, like a scan).
    pub fn wake(&self) {
        self.dirty.store(true, Relaxed);
    }
    /// Everything pushed since the last call, oldest first.
    pub fn take(&self) -> Vec<Event> {
        std::mem::take(&mut *self.queue.lock().unwrap())
    }
    /// Whether something was pushed since the last call (what the renderer polls).
    pub fn dirty(&self) -> bool {
        self.dirty.swap(false, Relaxed)
    }
}

impl Default for Events {
    fn default() -> Self {
        Self::new()
    }
}
