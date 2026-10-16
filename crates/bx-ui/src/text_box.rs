//! A single-line text box: the state (`TextBox`, edited with `Edit`s) and its node. Drawing the
//! frame around it, focusing it on a click and turning keys into `Edit`s is up to the app.
//! Not here (add when a screen needs it): selection, copy / paste, caret blink, clicking to
//! place the caret.
use crate::{Color, Fill, Node, text};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Edit {
    /// Insert (control characters are ignored).
    Char(char),
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
}

#[derive(Default, Clone, Debug)]
pub struct TextBox {
    pub text: String,
    /// Typing goes here, and the caret is drawn.
    pub focused: bool,
    /// In characters from the start.
    caret: usize,
}

impl TextBox {
    /// With `text` in it, the caret at its end.
    pub fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            focused: false,
            caret: text.chars().count(),
        }
    }

    fn byte(&self, chars: usize) -> usize {
        self.text
            .char_indices()
            .nth(chars)
            .map_or(self.text.len(), |(i, _)| i)
    }

    pub fn edit(&mut self, e: Edit) {
        let len = self.text.chars().count();
        match e {
            Edit::Char(c) if !c.is_control() => {
                self.text.insert(self.byte(self.caret), c);
                self.caret += 1;
            }
            Edit::Backspace if self.caret > 0 => {
                self.caret -= 1;
                self.text.remove(self.byte(self.caret));
            }
            Edit::Delete if self.caret < len => {
                self.text.remove(self.byte(self.caret));
            }
            Edit::Left => self.caret = self.caret.saturating_sub(1),
            Edit::Right => self.caret = (self.caret + 1).min(len),
            Edit::Home => self.caret = 0,
            Edit::End => self.caret = len,
            _ => {}
        }
    }

    /// The text (`placeholder` in `dim` while empty), filling the width it is given and showing
    /// the part around the caret if it is too long; the caret is drawn while `focused`.
    pub fn node(&self, placeholder: &str, size: f32, color: Color, dim: Color) -> Node {
        let n = if self.text.is_empty() {
            text(placeholder, size, dim)
        } else {
            text(&self.text, size, color)
        };
        n.w(Fill(1)).caret(self.focused.then_some(self.caret))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_and_deleting_around_the_caret() {
        let mut t = TextBox::new("héllo");
        t.edit(Edit::Left);
        t.edit(Edit::Left);
        t.edit(Edit::Char('X'));
        assert_eq!(t.text, "hélXlo");
        for _ in 0..3 {
            t.edit(Edit::Backspace); // X, l, then the multi-byte é
        }
        assert_eq!(t.text, "hlo");
        t.edit(Edit::Home);
        t.edit(Edit::Backspace); // nothing before the start
        t.edit(Edit::Delete);
        assert_eq!(t.text, "lo");
        t.edit(Edit::End);
        t.edit(Edit::Delete); // nothing after the end
        t.edit(Edit::Char('\n')); // not text
        assert_eq!(t.text, "lo");
    }
}
