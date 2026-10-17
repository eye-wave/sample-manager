//! Hello world: a spinning rainbow cube (3D projected on the CPU, the renderer is 2D), a showcase
//! of icons and of the text box, and a counter. Same shell / ui crates as sample-manager.
use bx_app::bx_shell::gpu::{Scene, Vert};
use bx_app::bx_shell::input::Event;
use bx_app::bx_ui::*;
use bx_app::{App, Mesh, Typing, icons, run_window};

const W: u32 = 760;
const H: u32 = 540;

const BG: Color = rgb(0x2d2a32);
const PANEL: Color = rgb(0x37333d);
const FIELD: Color = rgb(0x443f4a);
const HOVER: Color = rgb(0x4e4856);
const LINE: Color = rgb(0x3f3b46);
const TEXT: Color = rgb(0xb8b0c5);
const DIM: Color = rgb(0x827b8f);
const PINK: Color = rgb(0xf4b6c8);

// ids for hit testing (`Rects`)
const INPUT: u32 = 1;
const CUBE: u32 = 2;
const DEC: u32 = 3;
const INC: u32 = 4;
const RESET: u32 = 5;

const MAIL: usize = 7; // index in `ICONS`
const ICONS: [Mesh; 12] = icons![
    "heart", "star", "house", "settings", "music", "bell", "camera", "mail", "sun", "moon", "zap",
    "cloud",
];

#[derive(Default)]
struct Hello {
    rects: Rects,
    input: TextBox,
    count: i32,
}

impl App for Hello {
    const ICONS: &'static [Mesh] = &ICONS;
    const ANIMATED: bool = true;

    fn view(&self, _: (u32, u32)) -> Node {
        view(self)
    }
    fn rects_mut(&mut self) -> &mut Rects {
        &mut self.rects
    }
    fn event(&mut self, e: Event, _: (u32, u32), _: [f32; 2]) {
        if self.input.feed(&e) {
            return;
        }
        if let Event::Click(x, y) = e {
            let at = [x as f32, y as f32];
            self.input.focused = self.rects.hit(INPUT, at);
            if self.rects.hit(DEC, at) {
                self.count = self.count.saturating_sub(1);
            }
            if self.rects.hit(INC, at) {
                self.count = self.count.saturating_add(1);
            }
            if self.rects.hit(RESET, at) {
                self.count = 0;
            }
        }
    }
    fn paint(&self, scene: &mut Scene, size: (u32, u32), t: f32) {
        if let Some(r) = self.rects.rect(CUBE) {
            cube(&mut scene.flat, r, t, size);
        }
    }
}

fn card(title: &str, body: Node) -> Node {
    col(vec![text(title, 10.0, DIM), body])
        .spacing(10.0)
        .pad(14.0)
        .w(Fill(1))
        .bg(PANEL, 10.0)
}

fn button(label: &str, id: u32) -> Node {
    row(vec![text(label, 15.0, TEXT)])
        .align(Align::Center)
        .pad_xy(18.0, 0.0)
        .size(Shrink, Fixed(36.0))
        .bg(FIELD, 8.0)
        .hover(HOVER)
        .cursor(Cursor::Pointer)
        .id(id)
}

fn view(app: &Hello) -> Node {
    let cells = |r: usize| {
        row((0..6)
            .map(|c| {
                row(vec![icon(r * 6 + c, 20.0, TEXT)])
                    .align(Align::Center)
                    .pad_xy(16.0, 0.0)
                    .size(Fixed(52.0), Fixed(40.0))
                    .bg(FIELD, 8.0)
                    .hover(HOVER)
            })
            .collect())
        .spacing(6.0)
    };
    let input = row(vec![
        icon(MAIL, 14.0, DIM),
        app.input.node("Type something...", 13.0, TEXT, DIM),
    ])
    .spacing(10.0)
    .align(Align::Center)
    .pad_xy(12.0, 0.0)
    .size(Fill(1), Fixed(32.0))
    .bg(FIELD, 8.0)
    .border(if app.input.focused { PINK } else { LINE }, 1.0)
    .cursor(Cursor::Text)
    .id(INPUT);
    let echo = text(
        format!(
            "{} characters: {}",
            app.input.text.chars().count(),
            app.input.text
        ),
        11.0,
        DIM,
    )
    .w(Fill(1))
    .truncate();
    let counter = row(vec![
        button("-", DEC),
        row(vec![text(app.count.to_string(), 26.0, PINK)])
            .align(Align::Center)
            .size(Fixed(90.0), Fixed(40.0)),
        button("+", INC),
        space().w(Fill(1)),
        button("Reset", RESET),
    ])
    .align(Align::Center);
    col(vec![
        row(vec![
            text("Hello, world!", 26.0, PINK),
            space().w(Fill(1)),
            text("bx_ui + Vulkan", 11.0, DIM),
        ])
        .align(Align::Center)
        .size(Fill(1), Fixed(36.0)),
        row(vec![
            card("3D CUBE", space().size(Fixed(300.0), Fixed(300.0)).id(CUBE)).w(Shrink),
            col(vec![
                card("ICONS", col(vec![cells(0), cells(1)]).spacing(6.0)),
                card("TEXT BOX", col(vec![input, echo]).spacing(8.0)),
                card("COUNTER", counter),
            ])
            .spacing(12.0)
            .w(Fill(1)),
        ])
        .spacing(12.0),
    ])
    .spacing(14.0)
    .pad(20.0)
    .fill()
    .bg(BG, 0.0)
}

/// HSV with full saturation and value, `hue` in degrees.
fn rainbow(hue: f32) -> [f32; 3] {
    let c = |n: f32| {
        let k = (n + hue / 60.0) % 6.0;
        1.0 - k.min(4.0 - k).clamp(0.0, 1.0)
    };
    [c(5.0), c(3.0), c(1.0)]
}

/// A cube turning `t` seconds' worth, centred in `rect`. Perspective projection; the back faces
/// are skipped by hand (a convex solid needs no depth buffer).
fn cube(out: &mut Vec<Vert>, [x, y, w, h]: Rect, t: f32, (fw, fh): (u32, u32)) {
    let (fw, fh) = (fw as f32, fh as f32);
    const CORNERS: [[f32; 3]; 8] = [
        [-1., -1., -1.],
        [1., -1., -1.],
        [1., 1., -1.],
        [-1., 1., -1.],
        [-1., -1., 1.],
        [1., -1., 1.],
        [1., 1., 1.],
        [-1., 1., 1.],
    ];
    const FACES: [[usize; 4]; 6] = [
        [0, 1, 2, 3],
        [5, 4, 7, 6],
        [4, 0, 3, 7],
        [1, 5, 6, 2],
        [4, 5, 1, 0],
        [3, 2, 6, 7],
    ];
    const D: f32 = 4.5; // camera distance
    let (sy, cy, sx, cx) = (
        (t * 0.9).sin(),
        (t * 0.9).cos(),
        (t * 0.6).sin(),
        (t * 0.6).cos(),
    );
    let rot = CORNERS.map(|[px, py, pz]| {
        let (x1, z1) = (px * cy + pz * sy, -px * sy + pz * cy);
        [x1, py * cx - z1 * sx, py * sx + z1 * cx]
    });
    let k = w.min(h) / 6.0;
    let scr = rot.map(|[px, py, pz]| {
        [
            x + w / 2.0 + px * k * D / (pz + D),
            y + h / 2.0 + py * k * D / (pz + D),
        ]
    });
    for f in FACES {
        // the face's outward normal is its centre (the cube sits on the origin)
        let c = f.map(|i| rot[i]).iter().fold([0.0; 3], |a, p| {
            [a[0] + p[0] / 4.0, a[1] + p[1] / 4.0, a[2] + p[2] / 4.0]
        });
        if c[0] * c[0] + c[1] * c[1] + c[2] * (c[2] + D) >= 0.0 {
            continue; // faces away from the camera at (0, 0, -D)
        }
        let shade =
            0.6 + 0.4 * (-c[2] / (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt()).clamp(0.0, 1.0);
        let v = |i: usize| -> Vert {
            let [r, g, b] = rainbow(i as f32 * 45.0).map(|v| v * shade);
            [
                scr[i][0] / fw * 2.0 - 1.0,
                scr[i][1] / fh * 2.0 - 1.0,
                0.0,
                0.0,
                r,
                g,
                b,
                1.0,
            ]
        };
        out.extend([v(f[0]), v(f[1]), v(f[2]), v(f[0]), v(f[2]), v(f[3])]);
    }
}

fn main() {
    run_window("hello", (W, H), Hello::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use bx_app::frame;

    #[test]
    fn the_counter_counts_the_cube_shows_at_most_three_faces_and_typing_works() {
        let mut app = Hello::default();
        let (none, size) = ([f32::NAN; 2], (W, H));
        let s = frame(&mut app, size, none, 1.0, vec![]); // a frame first: it records the rects
        assert!(s.flat.len() + s.textured.len() <= 65536);
        let mut tris = vec![];
        cube(&mut tris, [0.0, 0.0, 300.0, 300.0], 1.0, size);
        assert!(!tris.is_empty() && tris.len() <= 3 * 6, "{}", tris.len());
        let at = |app: &Hello, id| {
            let r = app.rects.rect(id).unwrap();
            Event::Click((r[0] + 5.0) as i32, (r[1] + 5.0) as i32)
        };
        let (inc, dec, input) = (at(&app, INC), at(&app, DEC), at(&app, INPUT));
        frame(&mut app, size, none, 0.0, vec![inc, inc, inc, dec]);
        assert_eq!(app.count, 2);
        frame(
            &mut app,
            size,
            none,
            0.0,
            vec![input, Event::Char('h'), Event::Char('i')],
        );
        assert_eq!(app.input.text, "hi");
    }
}
