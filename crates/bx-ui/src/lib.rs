//! A small layout engine in the spirit of iced: build a tree of rows / columns / leaves sized with
//! `Length`, call [`layout`], get a flat draw list of pixel-space [`Prim`]s. No GPU, no deps.
//! Hover: pass the mouse to [`layout`]; a node can change its background (`hover`) or pop a second
//! node over everything (`over`). Not here (add when a screen needs it): clipping, scrolling,
//! clicks, text wrapping.
mod text_box;
pub use text_box::{Edit, TextBox};

pub type Color = [f32; 4];

/// `0xRRGGBB` -> opaque colour.
pub const fn rgb(c: u32) -> Color {
    [
        (c >> 16 & 255) as f32 / 255.0,
        (c >> 8 & 255) as f32 / 255.0,
        (c & 255) as f32 / 255.0,
        1.0,
    ]
}

/// x, y, w, h in px, y down.
pub type Rect = [f32; 4];

/// What a `Rect` can tell about a point.
pub trait RectExt {
    /// Whether the point (px, same space as the rect) is inside; NaN (no mouse) never is.
    fn hit(&self, p: [f32; 2]) -> bool;
}
impl RectExt for Rect {
    fn hit(&self, p: [f32; 2]) -> bool {
        let [x, y, w, h] = *self;
        (x..x + w).contains(&p[0]) && (y..y + h).contains(&p[1])
    }
}

/// Where the nodes with an `id` were put by [`layout`] (a `Vec` of `(id, rect)`, in draw order),
/// to find out what a click hit.
#[derive(Clone, Default, Debug)]
pub struct Rects(pub Vec<(u32, Rect)>);
impl std::ops::Deref for Rects {
    type Target = Vec<(u32, Rect)>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Rects {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<Vec<(u32, Rect)>> for Rects {
    fn from(v: Vec<(u32, Rect)>) -> Self {
        Self(v)
    }
}
impl Rects {
    /// Where node `id` is (the last one with that id).
    pub fn rect(&self, id: u32) -> Option<Rect> {
        self.0.iter().rev().find(|r| r.0 == id).map(|r| r.1)
    }
    /// The ids of the nodes under `p`, first drawn first.
    pub fn at(&self, p: [f32; 2]) -> impl Iterator<Item = u32> + '_ {
        self.0.iter().filter(move |r| r.1.hit(p)).map(|r| r.0)
    }
    /// Whether node `id` is under `p`.
    pub fn hit(&self, id: u32, p: [f32; 2]) -> bool {
        self.0.iter().any(|r| r.0 == id && r.1.hit(p))
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Length {
    /// Share of what the siblings leave over, by weight (iced's `FillPortion`).
    Fill(u16),
    /// Exactly what the content needs.
    Shrink,
    Fixed(f32),
}
pub use Length::{Fill, Fixed, Shrink};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Align {
    Start,
    Center,
    End,
}

/// The mouse cursor to show.
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum Cursor {
    #[default]
    Default,
    /// The hand, for what can be clicked.
    Pointer,
    /// The I-beam, for text that can be edited.
    Text,
    /// The diagonal double arrow of a bottom right corner handle.
    Resize,
}

/// What [`layout`] returns.
pub struct Layout {
    /// Prims in draw order.
    pub base: Vec<Prim>,
    /// Prims that go on top of all of `base` (tooltips), in draw order.
    pub over: Vec<Prim>,
    /// What the mouse is on asked for, the last node in draw order wins.
    pub cursor: Cursor,
    /// Where the nodes with an `id` were put, to find out what a click hit.
    pub rects: Rects,
}

/// Background of a node: a (rounded) fill and an optional border inside it.
#[derive(Clone, Copy)]
pub struct Bg {
    pub color: Color,
    pub radius: f32,
    pub border: Option<(Color, f32)>,
    /// Dashed instead of solid border.
    pub dashed: bool,
}

enum Kind {
    /// Lays its children out along axis 0 (row, x) or 1 (column, y).
    Flow(usize),
    Text(String, f32, Color),
    Icon(usize, Color),
}

pub struct Node {
    kind: Kind,
    len: [Length; 2],
    lo: [f32; 2],
    hi: [f32; 2],
    spacing: f32,
    align: Align,
    bg: Option<Bg>,
    /// Background while the mouse is on the node.
    hover: Option<Color>,
    /// Drawn on top of everything from the node's top left while the mouse is on it, but only if
    /// it is wider than the node (a tooltip for text that does not fit).
    over: Option<Box<Node>>,
    /// Text only: cut to the width it gets, ending in "...".
    trunc: bool,
    /// Cursor while the mouse is on the node.
    cursor: Cursor,
    id: Option<u32>,
    /// Text only: the caret is before this character; the text shows what fits around it.
    caret: Option<usize>,
    kids: Vec<Node>,
}

/// What [`layout`] returns, in draw order.
#[derive(Debug)]
pub enum Prim {
    Quad(Rect, Color, f32),
    /// `x`, `y` is the baseline origin.
    Text {
        x: f32,
        y: f32,
        size: f32,
        color: Color,
        s: String,
    },
    Icon(usize, Rect, Color),
}

fn node(kind: Kind, kids: Vec<Node>) -> Node {
    Node {
        kind,
        len: [Shrink; 2],
        lo: [0.0; 2],
        hi: [0.0; 2],
        spacing: 0.0,
        align: Align::Start,
        bg: None,
        hover: None,
        over: None,
        trunc: false,
        cursor: Cursor::Default,
        id: None,
        caret: None,
        kids,
    }
}
pub fn row(kids: Vec<Node>) -> Node {
    node(Kind::Flow(0), kids)
}
pub fn col(kids: Vec<Node>) -> Node {
    node(Kind::Flow(1), kids)
}
pub fn text(s: impl Into<String>, size: f32, color: Color) -> Node {
    node(Kind::Text(s.into(), size, color), vec![])
}
/// An icon (an index the app understands) of `size` x `size` px.
pub fn icon(id: usize, size: f32, color: Color) -> Node {
    node(Kind::Icon(id, color), vec![]).size(Fixed(size), Fixed(size))
}
/// Empty; give it a size (`.w(Fill(1))` pushes its siblings apart).
pub fn space() -> Node {
    row(vec![])
}

impl Node {
    pub fn w(mut self, l: Length) -> Self {
        self.len[0] = l;
        self
    }
    pub fn h(mut self, l: Length) -> Self {
        self.len[1] = l;
        self
    }
    pub fn size(self, w: Length, h: Length) -> Self {
        self.w(w).h(h)
    }
    pub fn fill(self) -> Self {
        self.size(Fill(1), Fill(1))
    }
    pub fn pad(self, p: f32) -> Self {
        self.pad_xy(p, p)
    }
    pub fn pad_xy(mut self, x: f32, y: f32) -> Self {
        (self.lo, self.hi) = ([x, y], [x, y]);
        self
    }
    pub fn spacing(mut self, s: f32) -> Self {
        self.spacing = s;
        self
    }
    /// Where children sit on the axis across the flow.
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;
        self
    }
    pub fn bg(mut self, color: Color, radius: f32) -> Self {
        self.bg = Some(Bg {
            color,
            radius,
            border: None,
            dashed: false,
        });
        self
    }
    pub fn hover(mut self, color: Color) -> Self {
        self.hover = Some(color);
        self
    }
    pub fn over(mut self, n: Node) -> Self {
        self.over = Some(Box::new(n));
        self
    }
    /// The mouse cursor while the mouse is on the node (`Pointer` for what can be clicked).
    pub fn cursor(mut self, c: Cursor) -> Self {
        self.cursor = c;
        self
    }
    /// A name for [`Layout::rects`].
    pub fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }
    /// Text with a caret before character `c` (see `TextBox`); give it `w(Fill(1))`.
    pub fn caret(mut self, c: Option<usize>) -> Self {
        self.caret = c;
        self
    }
    /// Text that gets less room than it needs ends in "..." (give it `w(Fill(1))`).
    pub fn truncate(mut self) -> Self {
        self.trunc = true;
        self
    }
    /// A dashed border (set after `border`).
    pub fn dashed(mut self) -> Self {
        if let Some(b) = &mut self.bg {
            b.dashed = true;
        }
        self
    }
    pub fn border(mut self, color: Color, width: f32) -> Self {
        if let Some(b) = &mut self.bg {
            b.border = Some((color, width));
        }
        self
    }

    /// Content size in px.
    fn natural(&self, m: &dyn Fn(&str, f32) -> f32) -> [f32; 2] {
        let pad = [self.lo[0] + self.hi[0], self.lo[1] + self.hi[1]];
        match &self.kind {
            Kind::Text(s, size, _) => [m(s, *size) + pad[0], size * LINE + pad[1]],
            Kind::Icon(..) => [0.0; 2], // always Fixed
            Kind::Flow(a) => {
                let sizes: Vec<_> = self.kids.iter().map(|k| k.resolved(m)).collect();
                let n = sizes.len().saturating_sub(1) as f32;
                let mut o = [0.0; 2];
                o[*a] = sizes.iter().map(|s| s[*a]).sum::<f32>() + self.spacing * n;
                o[1 - a] = sizes.iter().fold(0.0, |x, s| f32::max(x, s[1 - a]));
                [o[0] + pad[0], o[1] + pad[1]]
            }
        }
    }
    /// Size when the parent gives no extra room: Fixed as asked, otherwise the content.
    fn resolved(&self, m: &dyn Fn(&str, f32) -> f32) -> [f32; 2] {
        let n = self.natural(m);
        [0, 1].map(|i| if let Fixed(f) = self.len[i] { f } else { n[i] })
    }
}

/// Line height in em.
const LINE: f32 = 1.2;
/// Baseline from the top of a text line, in em (DejaVu: ascent .93, descent .24).
const BASE: f32 = 0.95;

/// The mouse is nowhere (NaN never compares inside anything).
pub const NO_MOUSE: [f32; 2] = [f32::NAN; 2];

/// Lays `root` out in `rect`, `measure(text, size)` giving a string's width in px, `mouse` the
/// pointer in the same px (or [`NO_MOUSE`]).
pub fn layout(
    root: &Node,
    rect: Rect,
    measure: &dyn Fn(&str, f32) -> f32,
    mouse: [f32; 2],
) -> Layout {
    let mut l = Layout {
        base: vec![],
        over: vec![],
        cursor: Cursor::Default,
        rects: Rects::default(),
    };
    place(root, rect, measure, mouse, &mut l);
    l
}

/// `s` cut to fit `w` px, with "..." at the end.
fn ellipsize(s: &str, size: f32, w: f32, m: &dyn Fn(&str, f32) -> f32) -> String {
    let mut t: Vec<char> = s.chars().collect();
    while !t.is_empty() && m(&(t.iter().collect::<String>() + "..."), size) > w {
        t.pop();
    }
    t.iter().collect::<String>() + "..."
}

/// The part of `s` that fits `w` px and has the caret (before character `c`) in it, and the
/// caret's x in it.
fn around_caret(
    s: &str,
    c: usize,
    size: f32,
    w: f32,
    m: &dyn Fn(&str, f32) -> f32,
) -> (String, f32) {
    let ch: Vec<char> = s.chars().collect();
    let c = c.min(ch.len());
    let sub = |a: usize, b: usize| ch[a..b].iter().collect::<String>();
    let mut a = 0;
    while a < c && m(&sub(a, c), size) > w - 1.0 {
        a += 1;
    }
    let mut b = ch.len();
    while b > c && m(&sub(a, b), size) > w {
        b -= 1;
    }
    (sub(a, b), m(&sub(a, c), size))
}

/// Points along the outline of a rounded rect (clockwise), `step` px apart or so.
fn outline([x, y, w, h]: Rect, radius: f32) -> Vec<[f32; 2]> {
    let r = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    let mut pts = vec![];
    for (cx, cy, a0) in [
        (x + w - r, y + r, -90.0f32),
        (x + w - r, y + h - r, 0.0),
        (x + r, y + h - r, 90.0),
        (x + r, y + r, 180.0),
    ] {
        for i in 0..=8 {
            let a = (a0 + 90.0 * i as f32 / 8.0).to_radians();
            pts.push([cx + r * a.cos(), cy + r * a.sin()]);
        }
    }
    pts.push(pts[0]);
    pts
}

/// A dashed border of width `bw` just inside `r`: 1 px wide squares along the outline, 4 on, 3 off.
fn dashes(out: &mut Vec<Prim>, r: Rect, radius: f32, bw: f32, c: Color) {
    let half = bw / 2.0;
    let pts = outline(
        [r[0] + half, r[1] + half, r[2] - bw, r[3] - bw],
        radius - half,
    );
    let mut d = 0.0f32;
    for seg in pts.windows(2) {
        let (a, b) = (seg[0], seg[1]);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let n = len.ceil().max(1.0) as usize;
        for i in 0..n {
            if d % 7.0 < 4.0 {
                let t = i as f32 / n as f32;
                let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                out.push(Prim::Quad([p[0] - half, p[1] - half, bw, bw], c, 0.0));
            }
            d += len / n as f32;
        }
    }
}

fn place(n: &Node, r: Rect, m: &dyn Fn(&str, f32) -> f32, mouse: [f32; 2], l: &mut Layout) {
    let tip_w = n.over.as_ref().map_or(0.0, |o| o.resolved(m)[0]);
    let tip = tip_w > r[2];
    let hot = r.hit(mouse); // only the node itself, not the tooltip that reaches past it
    if hot && n.cursor != Cursor::Default {
        l.cursor = n.cursor;
    }
    if let Some(id) = n.id {
        l.rects.push((id, r));
    }
    let bg = match (n.bg, n.hover) {
        (Some(b), Some(c)) if hot => Some(Bg { color: c, ..b }),
        (None, Some(c)) if hot => Some(Bg {
            color: c,
            radius: 0.0,
            border: None,
            dashed: false,
        }),
        (b, _) => b,
    };
    if let Some(bg) = bg {
        match bg.border {
            Some((c, bw)) => {
                if bg.dashed {
                    l.base.push(Prim::Quad(r, bg.color, bg.radius));
                    dashes(&mut l.base, r, bg.radius, bw, c);
                } else {
                    l.base.push(Prim::Quad(r, c, bg.radius));
                    let inset = [r[0] + bw, r[1] + bw, r[2] - 2.0 * bw, r[3] - 2.0 * bw];
                    l.base
                        .push(Prim::Quad(inset, bg.color, (bg.radius - bw).max(0.0)));
                }
            }
            None => l.base.push(Prim::Quad(r, bg.color, bg.radius)),
        }
    }
    if hot
        && tip
        && let Some(o) = &n.over
    {
        let mut t = Layout {
            base: vec![],
            over: vec![],
            cursor: Cursor::Default,
            rects: Rects::default(),
        };
        place(o, [r[0], r[1], tip_w, r[3]], m, NO_MOUSE, &mut t);
        l.over.extend(t.base);
    }
    let inner = [
        r[0] + n.lo[0],
        r[1] + n.lo[1],
        r[2] - n.lo[0] - n.hi[0],
        r[3] - n.lo[1] - n.hi[1],
    ];
    match &n.kind {
        Kind::Text(s, size, color) => {
            let top = inner[1] + (inner[3] - size * LINE) / 2.0;
            let y = top + size * BASE;
            let s = if let Some(c) = n.caret {
                let (shown, cx) = around_caret(s, c, *size, inner[2], m);
                l.base.push(Prim::Quad(
                    [inner[0] + cx, top, 1.0, size * LINE],
                    *color,
                    0.0,
                ));
                shown
            } else if n.trunc && m(s, *size) > inner[2] {
                ellipsize(s, *size, inner[2], m)
            } else {
                s.clone()
            };
            l.base.push(Prim::Text {
                x: inner[0],
                y,
                size: *size,
                color: *color,
                s,
            });
        }
        Kind::Icon(id, c) => l.base.push(Prim::Icon(*id, inner, *c)),
        Kind::Flow(a) => {
            let (a, c) = (*a, 1 - *a);
            // main axis: Fixed / Shrink first, Fill kids split what is left by weight
            let sizes: Vec<_> = n.kids.iter().map(|k| k.resolved(m)).collect();
            let used: f32 = n
                .kids
                .iter()
                .zip(&sizes)
                .filter(|(k, _)| !matches!(k.len[a], Fill(_)))
                .map(|(_, s)| s[a])
                .sum();
            let weight: f32 = n
                .kids
                .iter()
                .map(|k| {
                    if let Fill(w) = k.len[a] {
                        w as f32
                    } else {
                        0.0
                    }
                })
                .sum();
            let gaps = n.spacing * n.kids.len().saturating_sub(1) as f32;
            let left = (inner[2 + a] - used - gaps).max(0.0);
            let mut pos = inner[a];
            for (k, s) in n.kids.iter().zip(&sizes) {
                let main = match k.len[a] {
                    Fill(w) => left * w as f32 / weight,
                    _ => s[a],
                };
                let cross = if matches!(k.len[c], Fill(_)) {
                    inner[2 + c]
                } else {
                    s[c]
                };
                let off = match n.align {
                    Align::Start => 0.0,
                    Align::Center => (inner[2 + c] - cross) / 2.0,
                    Align::End => inner[2 + c] - cross,
                };
                let mut kr = [0.0; 4];
                (kr[a], kr[c], kr[2 + a], kr[2 + c]) = (pos, inner[c] + off, main, cross);
                place(k, kr, m, mouse, l);
                pos += main + n.spacing;
            }
        }
    }
}

/// Triangles (a flat list of x, y) of a rounded rect: a fan from the centre over the outline.
pub fn quad_tris(out: &mut Vec<[f32; 2]>, [x, y, w, h]: Rect, radius: f32) {
    let r = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    let steps = if r < 0.5 {
        0
    } else if r < 8.0 {
        3
    } else {
        6
    };
    let mut ring = vec![];
    // corners clockwise from the top right, each an arc of a quarter turn
    for (cx, cy, a0) in [
        (x + w - r, y + r, -90.0f32),
        (x + w - r, y + h - r, 0.0),
        (x + r, y + h - r, 90.0),
        (x + r, y + r, 180.0),
    ] {
        for i in 0..=steps {
            let a = (a0 + 90.0 * i as f32 / steps.max(1) as f32).to_radians();
            ring.push([cx + r * a.cos(), cy + r * a.sin()]);
            if steps == 0 {
                break;
            }
        }
    }
    let mid = [x + w / 2.0, y + h / 2.0];
    for i in 0..ring.len() {
        out.extend([mid, ring[i], ring[(i + 1) % ring.len()]]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(s: &str, size: f32) -> f32 {
        s.len() as f32 * size / 2.0 // 5 px per char at size 10
    }
    fn quads(p: &[Prim]) -> Vec<Rect> {
        p.iter()
            .filter_map(|p| {
                if let Prim::Quad(r, ..) = p {
                    Some(*r)
                } else {
                    None
                }
            })
            .collect()
    }
    fn block(w: Length, h: Length) -> Node {
        space().size(w, h).bg([1.0; 4], 0.0)
    }

    #[test]
    fn fill_shares_what_fixed_and_spacing_leave() {
        let ui = row(vec![
            block(Fixed(20.0), Fill(1)),
            block(Fill(1), Fill(1)),
            block(Fill(3), Fill(1)),
        ])
        .spacing(10.0)
        .pad(5.0);
        let q = quads(&layout(&ui, [0.0, 0.0, 200.0, 50.0], &m, NO_MOUSE).base);
        // inner 190, minus 20 fixed and 20 gaps = 150 -> 37.5 and 112.5
        assert_eq!(
            q,
            [
                [5.0, 5.0, 20.0, 40.0],
                [35.0, 5.0, 37.5, 40.0],
                [82.5, 5.0, 112.5, 40.0]
            ]
        );
    }

    #[test]
    fn shrink_text_and_cross_alignment() {
        let ui = col(vec![text("abcd", 10.0, [1.0; 4]).bg([0.0; 4], 0.0)])
            .align(Align::Center)
            .w(Fixed(100.0));
        let p = layout(&ui, [0.0, 0.0, 100.0, 50.0], &m, NO_MOUSE).base;
        assert_eq!(quads(&p), [[40.0, 0.0, 20.0, 12.0]]); // 4 chars * 5 px, centred; line is 1.2 em
    }

    #[test]
    fn padding_counts_towards_shrink_size() {
        let ui = row(vec![
            text("abcd", 10.0, [1.0; 4])
                .pad_xy(3.0, 2.0)
                .bg([0.0; 4], 0.0),
        ]);
        assert_eq!(
            quads(&layout(&ui, [0.0, 0.0, 100.0, 50.0], &m, NO_MOUSE).base),
            [[0.0, 0.0, 26.0, 16.0]]
        );
    }

    #[test]
    fn spacer_pushes_siblings_apart() {
        let ui = row(vec![
            block(Fixed(10.0), Fixed(10.0)),
            space().w(Fill(1)),
            block(Fixed(10.0), Fixed(10.0)),
        ]);
        let q = quads(&layout(&ui, [0.0, 0.0, 100.0, 10.0], &m, NO_MOUSE).base);
        assert_eq!((q[0][0], q[1][0]), (0.0, 90.0));
    }

    #[test]
    fn rounded_rect_triangles_stay_inside() {
        let mut t = vec![];
        quad_tris(&mut t, [10.0, 10.0, 40.0, 20.0], 6.0);
        assert!(
            t.len() % 3 == 0
                && t.iter()
                    .all(|p| (10.0..=50.0).contains(&p[0]) && (10.0..=30.0).contains(&p[1]))
        );
        t.clear();
        quad_tris(&mut t, [0.0, 0.0, 4.0, 4.0], 0.0);
        assert_eq!(t.len(), 12); // a square is 4 fan triangles, fine
    }

    #[test]
    fn hover_colours_the_node_and_a_too_wide_tip_goes_on_top() {
        let tip = || text("abcdefghij", 10.0, [1.0; 4]).bg([9.0; 4], 0.0); // 50 px
        let cell = || {
            text("abcdefghij", 10.0, [1.0; 4])
                .w(Fixed(30.0))
                .truncate()
                .hover([5.0; 4])
                .over(tip())
        };
        let ui = || row(vec![cell()]);
        let r = [0.0, 0.0, 100.0, 20.0];
        let l = layout(&ui(), r, &m, NO_MOUSE);
        let (base, over) = (l.base, l.over);
        assert!(over.is_empty() && quads(&base).is_empty());
        // 30 px fit 5 chars at 5 px: "a..." is 4 chars = 20 px, "ab..." is 25, "abcd..." 35
        assert!(
            base.iter()
                .any(|p| matches!(p, Prim::Text { s, .. } if s == "abc..."))
        );
        let l = layout(&ui(), r, &m, [10.0, 5.0]);
        let (base, over) = (l.base, l.over);
        assert_eq!(quads(&base), [[0.0, 0.0, 30.0, 12.0]]);
        assert_eq!(quads(&over), [[0.0, 0.0, 50.0, 12.0]]);
        assert!(layout(&ui(), r, &m, [40.0, 5.0]).over.is_empty()); // on where the tip would be
        let hand = row(vec![cell().cursor(Cursor::Pointer)]);
        assert_eq!(layout(&hand, r, &m, [10.0, 5.0]).cursor, Cursor::Pointer);
        assert_eq!(layout(&hand, r, &m, [90.0, 5.0]).cursor, Cursor::Default);
    }
}
