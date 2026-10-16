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
    /// Ctrl+Backspace / Ctrl+Delete: the word before / after the caret.
    WordBackspace,
    WordDelete,
    /// Ctrl+A. hack, nothing is drawn; the next Backspace / Delete / typing replaces
    /// everything, any other edit drops it. TODO: real selection.
    SelectAll,
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
    all: bool,
}

impl TextBox {
    /// With `text` in it, the caret at its end.
    pub fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            focused: false,
            caret: text.chars().count(),
            all: false,
        }
    }

    fn byte(&self, chars: usize) -> usize {
        self.text
            .char_indices()
            .nth(chars)
            .map_or(self.text.len(), |(i, _)| i)
    }

    /// Removes chars `a..b`, the caret goes to `a`.
    fn cut(&mut self, a: usize, b: usize) {
        self.text.replace_range(self.byte(a)..self.byte(b), "");
        self.caret = a;
    }

    /// Where Ctrl+Backspace / Ctrl+Delete stop: past the blanks, then past the word.
    fn word(&self, back: bool) -> usize {
        let c: Vec<char> = self.text.chars().collect();
        let (mut i, blank) = (self.caret, |c: &char| c.is_whitespace());
        if back {
            while i > 0 && blank(&c[i - 1]) {
                i -= 1;
            }
            while i > 0 && !blank(&c[i - 1]) {
                i -= 1;
            }
        } else {
            while i < c.len() && !blank(&c[i]) {
                i += 1;
            }
            while i < c.len() && blank(&c[i]) {
                i += 1;
            }
        }
        i
    }

    pub fn edit(&mut self, e: Edit) {
        let len = self.text.chars().count();
        if std::mem::take(&mut self.all) {
            match e {
                Edit::Backspace | Edit::Delete | Edit::WordBackspace | Edit::WordDelete => {
                    return self.cut(0, len);
                }
                Edit::Char(c) if !c.is_control() => self.cut(0, len),
                _ => {}
            }
        }
        match e {
            Edit::SelectAll => self.all = true,
            Edit::WordBackspace => self.cut(self.word(true), self.caret),
            Edit::WordDelete => self.cut(self.caret, self.word(false)),
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

    #[test]
    fn words_and_select_all() {
        let mut t = TextBox::new("one two  three");
        t.edit(Edit::WordBackspace);
        assert_eq!(t.text, "one two  ");
        t.edit(Edit::WordBackspace); // blanks, then the word
        assert_eq!(t.text, "one ");
        t.edit(Edit::Home);
        t.edit(Edit::WordDelete); // the word and the blank after it
        assert_eq!(t.text, "");
        let mut t = TextBox::new("héllo wörld");
        t.edit(Edit::Left);
        t.edit(Edit::WordBackspace);
        assert_eq!(t.text, "héllo d");
        t.edit(Edit::SelectAll);
        t.edit(Edit::End); // any other edit drops the hack
        t.edit(Edit::Delete);
        assert_eq!(t.text, "héllo ");
        t.edit(Edit::SelectAll);
        t.edit(Edit::Backspace);
        assert_eq!(t.text, "");
        t.edit(Edit::Char('a'));
        t.edit(Edit::SelectAll);
        t.edit(Edit::Char('b')); // typing replaces
        assert_eq!(t.text, "b");
    }
}
