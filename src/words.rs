//! What the checker needs to know about views' words: colours and keyboard shortcuts. The same
//! words are in native_ui.rs, which draws them; a test keeps the two the same.

struct Shortcut {
    command: bool,
    shift: bool,
    alt: bool,
    key: String,
}

const KEY_NAMES: &[&str] = &[
    "enter",
    "escape",
    "tab",
    "space",
    "backspace",
    "delete",
    "up",
    "down",
    "left",
    "right",
    "home",
    "end",
    "pageup",
    "pagedown",
];

fn function_key(k: &str) -> bool {
    k.strip_prefix('f')
        .and_then(|n| n.parse::<u8>().ok())
        .is_some_and(|n| (1..=24).contains(&n))
}

/// Reads a shortcut written like "ctrl+s", "shift+f5" or "escape".
pub fn parse_shortcut(spec: &str) -> Result<(), String> {
    let mut s = Shortcut {
        command: false,
        shift: false,
        alt: false,
        key: String::new(),
    };
    for part in spec.split('+') {
        let p = part.trim().to_lowercase();
        match p.as_str() {
            "ctrl" | "control" | "cmd" | "command" => s.command = true,
            "shift" => s.shift = true,
            "alt" | "option" => s.alt = true,
            "" if spec.trim() == "+" || spec.ends_with("++") => s.key = "+".into(),
            "" => return Err(format!("\"{spec}\" needs a key after each +, like \"ctrl+s\".")),
            k if !s.key.is_empty() => {
                return Err(format!(
                    "a shortcut is one key, with ctrl, shift or alt in front of it, but \"{spec}\" has {} and {k}.",
                    s.key
                ))
            }
            k => s.key = k.to_string(),
        }
    }
    if s.key.is_empty() {
        return Err(format!("\"{spec}\" needs a key as well as ctrl, shift or alt, like \"ctrl+s\"."));
    }
    if s.key.chars().count() != 1 && !KEY_NAMES.contains(&s.key.as_str()) && !function_key(&s.key) {
        return Err(format!(
            "Lust doesn't know the key {}. Keys are letters, numbers and signs (\"ctrl+s\", \"ctrl+1\"), f1 to f12, enter, escape, tab, space, backspace, delete, up, down, left, right, home, end, pageup and pagedown.",
            s.key
        ));
    }
    let _ = (s.command, s.shift, s.alt);
    Ok(())
}

pub const COLOURS: &[&str] = &[
    "white", "black", "gray1", "gray2", "gray3", "gray4", "gray5", "gray6", "gray7", "gray8", "gray9", "red", "orange", "yellow", "green",
    "teal", "blue", "indigo", "purple", "pink", "red1", "red2", "red3", "red4", "red5", "red6", "red7", "red8", "red9", "orange1",
    "orange2", "orange3", "orange4", "orange5", "orange6", "orange7", "orange8", "orange9", "yellow1", "yellow2", "yellow3", "yellow4",
    "yellow5", "yellow6", "yellow7", "yellow8", "yellow9", "green1", "green2", "green3", "green4", "green5", "green6", "green7", "green8",
    "green9", "teal1", "teal2", "teal3", "teal4", "teal5", "teal6", "teal7", "teal8", "teal9", "blue1", "blue2", "blue3", "blue4", "blue5",
    "blue6", "blue7", "blue8", "blue9", "indigo1", "indigo2", "indigo3", "indigo4", "indigo5", "indigo6", "indigo7", "indigo8", "indigo9",
    "purple1", "purple2", "purple3", "purple4", "purple5", "purple6", "purple7", "purple8", "purple9", "pink1", "pink2", "pink3", "pink4",
    "pink5", "pink6", "pink7", "pink8", "pink9",
];

/// What each colour word is, as 0xrrggbb: the Open Color palette. The views draw them with the
/// same numbers (native_ui.rs's own copy; a test keeps the two the same), and a colour value made
/// from a word (`colour warn = orange5;`) has exactly the same red, green and blue.
pub const PALETTE: &[(&str, u32)] = &[
    ("white", 0xffffff),
    ("black", 0x000000),
    ("gray1", 0xf8f9fa),
    ("gray2", 0xf1f3f5),
    ("gray3", 0xe9ecef),
    ("gray4", 0xdee2e6),
    ("gray5", 0xced4da),
    ("gray6", 0xadb5bd),
    ("gray7", 0x868e96),
    ("gray8", 0x495057),
    ("gray9", 0x212529),
    ("red", 0xe03131),
    ("orange", 0xf76707),
    ("yellow", 0xf59f00),
    ("green", 0x2f9e44),
    ("teal", 0x0c8599),
    ("blue", 0x1c7ed6),
    ("indigo", 0x4263eb),
    ("purple", 0x7048e8),
    ("pink", 0xd6336c),
    ("red1", 0xffe3e3),
    ("red2", 0xffc9c9),
    ("red3", 0xffa8a8),
    ("red4", 0xff8787),
    ("red5", 0xff6b6b),
    ("red6", 0xfa5252),
    ("red7", 0xf03e3e),
    ("red8", 0xe03131),
    ("red9", 0xc92a2a),
    ("orange1", 0xffe8cc),
    ("orange2", 0xffd8a8),
    ("orange3", 0xffc078),
    ("orange4", 0xffa94d),
    ("orange5", 0xff922b),
    ("orange6", 0xfd7e14),
    ("orange7", 0xf76707),
    ("orange8", 0xe8590c),
    ("orange9", 0xd9480f),
    ("yellow1", 0xfff3bf),
    ("yellow2", 0xffec99),
    ("yellow3", 0xffe066),
    ("yellow4", 0xffd43b),
    ("yellow5", 0xfcc419),
    ("yellow6", 0xfab005),
    ("yellow7", 0xf59f00),
    ("yellow8", 0xf08c00),
    ("yellow9", 0xe67700),
    ("green1", 0xd3f9d8),
    ("green2", 0xb2f2bb),
    ("green3", 0x8ce99a),
    ("green4", 0x69db7c),
    ("green5", 0x51cf66),
    ("green6", 0x40c057),
    ("green7", 0x37b24d),
    ("green8", 0x2f9e44),
    ("green9", 0x2b8a3e),
    ("teal1", 0xc5f6fa),
    ("teal2", 0x99e9f2),
    ("teal3", 0x66d9e8),
    ("teal4", 0x3bc9db),
    ("teal5", 0x22b8cf),
    ("teal6", 0x15aabf),
    ("teal7", 0x1098ad),
    ("teal8", 0x0c8599),
    ("teal9", 0x0b7285),
    ("blue1", 0xd0ebff),
    ("blue2", 0xa5d8ff),
    ("blue3", 0x74c0fc),
    ("blue4", 0x4dabf7),
    ("blue5", 0x339af0),
    ("blue6", 0x228be6),
    ("blue7", 0x1c7ed6),
    ("blue8", 0x1971c2),
    ("blue9", 0x1864ab),
    ("indigo1", 0xdbe4ff),
    ("indigo2", 0xbac8ff),
    ("indigo3", 0x91a7ff),
    ("indigo4", 0x748ffc),
    ("indigo5", 0x5c7cfa),
    ("indigo6", 0x4c6ef5),
    ("indigo7", 0x4263eb),
    ("indigo8", 0x3b5bdb),
    ("indigo9", 0x364fc7),
    ("purple1", 0xe5dbff),
    ("purple2", 0xd0bfff),
    ("purple3", 0xb197fc),
    ("purple4", 0x9775fa),
    ("purple5", 0x845ef7),
    ("purple6", 0x7950f2),
    ("purple7", 0x7048e8),
    ("purple8", 0x6741d9),
    ("purple9", 0x5f3dc4),
    ("pink1", 0xffdeeb),
    ("pink2", 0xfcc2d7),
    ("pink3", 0xfaa2c1),
    ("pink4", 0xf783ac),
    ("pink5", 0xf06595),
    ("pink6", 0xe64980),
    ("pink7", 0xd6336c),
    ("pink8", 0xc2255c),
    ("pink9", 0xa61e4d),
];

/// A colour word's red, green and blue, as 0xrrggbb.
pub fn rgb(word: &str) -> Option<u32> {
    PALETTE.iter().find(|(n, _)| *n == word).map(|(_, c)| *c)
}

/// Whether a word is a colour: one of Lust's names, or #rgb / #rrggbb / #rrggbbaa.
pub fn is_colour(v: &str) -> bool {
    if let Some(hex) = v.strip_prefix('#') {
        return matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    COLOURS.contains(&v)
}

#[cfg(all(test, feature = "cli"))]
mod same_as_the_drawing {
    #[test]
    fn colours() {
        assert_eq!(super::COLOURS, crate::native_ui::COLOURS);
        // Colour values made from words have exactly the colours the views draw.
        assert_eq!(super::PALETTE.len(), super::COLOURS.len());
        for (word, rgb) in super::PALETTE {
            assert!(super::COLOURS.contains(word), "{word}");
            assert_eq!(crate::native_ui::colour(word), Some(*rgb), "{word}");
        }
        for w in [
            "red",
            "#abc",
            "#a1b2c3",
            "#a1b2c380",
            "#a1b2c3d",
            "#abcd",
            "#xyz",
            "nope",
            "gray5",
            "white",
        ] {
            assert_eq!(super::is_colour(w), crate::native_ui::colour(w).is_some(), "{w}");
        }
    }

    #[test]
    fn dates_as_the_runtime_says() {
        for s in [
            "2026-11-13",
            "2024-02-29",
            "2000-02-29",
            "1900-02-29",
            "2100-02-29",
            "2026-02-30",
            "2026-13-01",
            "2026-00-01",
            "2026-01-00",
            "0000-01-01",
            "9999-12-31",
            "2026-1-01",
            "2026/01/01",
            "",
            "tomorrow",
            "2026-01-011",
        ] {
            assert_eq!(super::is_date(s), crate::native_rt::time_isDate(s.to_string()), "{s}");
        }
    }

    #[test]
    fn shortcuts() {
        for s in [
            "ctrl+s",
            "shift+f5",
            "escape",
            "+",
            "ctrl++",
            "ctrl+",
            "ctrl+s+x",
            "ctrl",
            "hyper",
            "f25",
            "alt+pagedown",
        ] {
            assert_eq!(super::parse_shortcut(s), crate::native_ui::parse_shortcut(s).map(|_| ()), "{s}");
        }
    }
}

/// Whether text is a real date written like "2026-11-13", as Time:isDate says: a datePicker's
/// min and max written in place are checked with it.
pub fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' || !b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()) {
        return false;
    }
    let (y, m, d): (u32, u32, u32) = (
        s[0..4].parse().unwrap_or(0),
        s[5..7].parse().unwrap_or(0),
        s[8..10].parse().unwrap_or(0),
    );
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&d)
}
