//! Clicks and typing from a window, for the renderer to pick up (`gpu::Content::events`). The
//! window thread pushes, the render thread takes; pushing also makes it draw again.
use std::path::PathBuf;
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

/// Pixels the mouse travels with the button held before a press on something draggable is a drag.
pub const DRAG_PX: i32 = 6;

/// Whether the mouse, pressed at (`x0`, `y0`), is now at (`x`, `y`) far enough to be a drag.
pub fn moved(x0: i32, y0: i32, x: i32, y: i32) -> bool {
    (x - x0).abs().max((y - y0).abs()) > DRAG_PX
}

pub struct Events {
    queue: Mutex<Vec<Event>>,
    dirty: AtomicBool,
    /// The file under the mouse that can be dragged out of the window, as the renderer last drew.
    drag: Mutex<Option<PathBuf>>,
}

impl Events {
    pub const fn new() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            dirty: AtomicBool::new(false),
            drag: Mutex::new(None),
        }
    }
    /// The renderer says what is under the mouse now (`None`: nothing that can be dragged).
    pub fn set_drag_source(&self, p: Option<PathBuf>) {
        *self.drag.lock().unwrap() = p;
    }
    /// What a press now would pick up, for the window to drag out once the mouse moves.
    pub fn drag_source(&self) -> Option<PathBuf> {
        self.drag.lock().unwrap().clone()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_becomes_a_drag_past_a_few_pixels() {
        assert!(!moved(10, 10, 14, 12) && !moved(10, 10, 10 + DRAG_PX, 10));
        assert!(moved(10, 10, 10 + DRAG_PX + 1, 10) && moved(10, 10, 9, 10 - DRAG_PX - 1));
        let e = Events::new();
        assert_eq!(e.drag_source(), None);
        e.set_drag_source(Some("a.wav".into()));
        assert_eq!(e.drag_source(), Some("a.wav".into()));
    }
}
