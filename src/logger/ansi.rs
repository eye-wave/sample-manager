pub fn ansi_to_html(input: &str) -> String {
    let mut output = String::with_capacity(input.len() * 2);

    let mut bold = false;
    let mut dim = false;
    let mut italic = false;
    let mut underline = false;
    let mut blink = false;
    let mut reverse = false;
    let mut strikethrough = false;
    let mut fg: Option<String> = None;
    let mut bg: Option<String> = None;
    let mut span_open = false;

    macro_rules! flush_span {
        () => {
            if span_open {
                output.push_str("</span>");
                span_open = false;
            }
            let style = build_style(
                bold,
                dim,
                italic,
                underline,
                blink,
                reverse,
                strikethrough,
                fg.as_deref(),
                bg.as_deref(),
            );
            if !style.is_empty() {
                output.push_str(&format!("<span style=\"{}\">", style));
                span_open = true;
            }
        };
    }

    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // ESC character (0x1B) starts an escape sequence.
        if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            // Find the end of the CSI sequence (a letter terminates it).
            let seq_start = i + 2;
            let mut seq_end = seq_start;
            while seq_end < bytes.len() && !bytes[seq_end].is_ascii_alphabetic() {
                seq_end += 1;
            }

            if seq_end < bytes.len() && bytes[seq_end] == b'm' {
                // Parse the SGR parameter string (e.g. "1;32" or "38;5;200").
                let params_str = &input[seq_start..seq_end];
                let mut params: Vec<u32> = params_str
                    .split(';')
                    .filter_map(|p| p.parse().ok())
                    .collect();
                if params.is_empty() {
                    params.push(0); // bare ESC[m == reset
                }

                let mut p = 0;
                while p < params.len() {
                    match params[p] {
                        0 => {
                            // Reset all attributes.
                            bold = false;
                            dim = false;
                            italic = false;
                            underline = false;
                            blink = false;
                            reverse = false;
                            strikethrough = false;
                            fg = None;
                            bg = None;
                        }
                        1 => bold = true,
                        2 => dim = true,
                        3 => italic = true,
                        4 => underline = true,
                        5 | 6 => blink = true,
                        7 => reverse = true,
                        9 => strikethrough = true,
                        22 => {
                            bold = false;
                            dim = false;
                        }
                        23 => italic = false,
                        24 => underline = false,
                        25 => blink = false,
                        27 => reverse = false,
                        29 => strikethrough = false,
                        // Standard foreground colors (30–37, 90–97).
                        n @ (30..=37 | 90..=97) => {
                            fg = Some(standard_color(n, n >= 90).to_string());
                        }
                        39 => fg = None, // default fg
                        // Standard background colors (40–47, 100–107).
                        n @ (40..=47 | 100..=107) => {
                            bg = Some(
                                standard_color(n - if n >= 100 { 60 } else { 10 }, n >= 100)
                                    .to_string(),
                            );
                        }
                        49 => bg = None, // default bg
                        // Extended color: 38 = fg, 48 = bg.
                        n @ (38 | 48) => {
                            let is_fg = n == 38;
                            if params.get(p + 1) == Some(&5) && p + 2 < params.len() {
                                // 256-color: 38;5;<index>
                                let color = color_256(params[p + 2]);
                                if is_fg {
                                    fg = Some(color);
                                } else {
                                    bg = Some(color);
                                }
                                p += 2;
                            } else if params.get(p + 1) == Some(&2) && p + 4 < params.len() {
                                // Truecolor: 38;2;<r>;<g>;<b>
                                let color = format!(
                                    "#{:02X}{:02X}{:02X}",
                                    params[p + 2],
                                    params[p + 3],
                                    params[p + 4]
                                );
                                if is_fg {
                                    fg = Some(color);
                                } else {
                                    bg = Some(color);
                                }
                                p += 4;
                            }
                        }
                        _ => {} // Unknown / unsupported code - ignore.
                    }
                    p += 1;
                }

                flush_span!();
                i = seq_end + 1;
                continue;
            } else {
                // Non-SGR CSI sequence - skip it entirely.
                i = seq_end + 1;
                continue;
            }
        }

        let ch = input[i..].chars().next().unwrap();
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(ch),
        }
        i += ch.len_utf8();
    }

    if span_open {
        output.push_str("</span>");
    }

    output
}

#[allow(clippy::too_many_arguments)]
fn build_style(
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    blink: bool,
    reverse: bool,
    strikethrough: bool,
    fg: Option<&str>,
    bg: Option<&str>,
) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let mut owned: Vec<String> = Vec::new(); // backing storage for formatted strings

    if bold {
        parts.push("font-weight:bold");
    }
    if dim {
        parts.push("opacity:0.5");
    }
    if italic {
        parts.push("font-style:italic");
    }

    let mut decorations: Vec<&str> = Vec::new();
    if underline {
        decorations.push("underline");
    }
    if strikethrough {
        decorations.push("line-through");
    }
    if blink {
        decorations.push("blink"); // rendered via CSS animation by the browser
    }
    if !decorations.is_empty() {
        let s = format!("text-decoration:{}", decorations.join(" "));
        owned.push(s);
    }

    if reverse {
        // Swap fg/bg visually; use CSS filter invert as a fallback.
        parts.push("filter:invert(1)");
    }

    if let Some(color) = fg {
        let s = format!("color:{}", color);
        owned.push(s);
    }
    if let Some(color) = bg {
        let s = format!("background-color:{}", color);
        owned.push(s);
    }

    // Combine owned strings into parts.
    for s in &owned {
        parts.push(s.as_str());
    }

    parts.join(";")
}

fn standard_color(raw_code: u32, bright: bool) -> &'static str {
    // Normalize to 0–7.
    let idx = if bright {
        (raw_code - 90) as usize
    } else {
        (raw_code - 30) as usize
    };
    if bright {
        const BRIGHT: [&str; 8] = [
            "#555555", // bright black (dark gray)
            "#FF5555", // bright red
            "#55FF55", // bright green
            "#FFFF55", // bright yellow
            "#5555FF", // bright blue
            "#FF55FF", // bright magenta
            "#55FFFF", // bright cyan
            "#FFFFFF", // bright white
        ];
        BRIGHT[idx.min(7)]
    } else {
        const NORMAL: [&str; 8] = [
            "#000000", // black
            "#AA0000", // red
            "#00AA00", // green
            "#AA5500", // yellow (dark)
            "#0000AA", // blue
            "#AA00AA", // magenta
            "#00AAAA", // cyan
            "#AAAAAA", // white (light gray)
        ];
        NORMAL[idx.min(7)]
    }
}

fn color_256(index: u32) -> String {
    match index {
        // Standard colors - reuse the 16-color mapping.
        0..=7 => standard_color(index + 30, false).to_string(),
        8..=15 => standard_color(index - 8 + 90, true).to_string(),
        // 6×6×6 color cube.
        16..=231 => {
            let n = index - 16;
            let b = n % 6;
            let g = (n / 6) % 6;
            let r = n / 36;
            let expand = |v: u32| if v == 0 { 0u32 } else { v * 40 + 55 };
            format!("#{:02X}{:02X}{:02X}", expand(r), expand(g), expand(b))
        }
        // Grayscale ramp.
        232..=255 => {
            let level = (index - 232) * 10 + 8;
            format!("#{:02X}{:02X}{:02X}", level, level, level)
        }
        _ => "#000000".to_string(),
    }
}
