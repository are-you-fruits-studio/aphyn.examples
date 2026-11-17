use aphyn::{parse_color_rgb_f32, parse_color_rgba_f32};

// Color codes: `rgb`, `rrggbb`, `rrggbbaa` (leading `#` optional, case-insensitive) or a name from
// `NAMED`. The digits go through `aphyn_math`'s `parse_color_rgb_f32` / `parse_color_rgba_f32`, so each
// channel is `byte / 255`.
//
// The codes are sRGB, as a color picker shows them, and the color target is linear:
// `srgb_to_linear` converts with the exact sRGB curve, and the sRGB window surface encodes the
// color back.

const NAMED: &[(&str, &str)] = &[
    ("black", "000000"),
    ("white", "ffffff"),
    ("red", "ff0000"),
    ("green", "00ff00"),
    ("blue", "0000ff"),
    ("yellow", "ffff00"),
    ("cyan", "00ffff"),
    ("magenta", "ff00ff"),
    ("orange", "ff8800"),
    ("gray", "808080"),
];

pub fn parse(input: &str) -> Option<[f32; 4]> {
    let raw = input.trim().trim_start_matches('#').to_ascii_lowercase();

    let hex = match NAMED.iter().find(|(name, _)| *name == raw) {
        Some((_, hex)) => (*hex).to_string(),
        None => raw,
    };

    // the engine parsers stop after 6 or 8 digits, so the length and the digits are checked here
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    match hex.len() {
        // CSS shorthand: `f0c` is `ff00cc`
        3 => {
            let doubled: String = hex.chars().flat_map(|c| [c, c]).collect();
            parse_color_rgb_f32(&doubled).map(|[r, g, b]| [r, g, b, 1.0])
        }
        6 => parse_color_rgb_f32(&hex).map(|[r, g, b]| [r, g, b, 1.0]),
        8 => parse_color_rgba_f32(&hex),
        _ => None,
    }
}

pub fn to_hex(color: [f32; 4]) -> String {
    let [r, g, b, a] = color.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
    format!("{r:02x}{g:02x}{b:02x}{a:02x}")
}

pub fn srgb_to_linear(color: [f32; 4]) -> [f32; 4] {
    let channel = |c: f32| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let [r, g, b, a] = color;
    [channel(r), channel(g), channel(b), a]
}
