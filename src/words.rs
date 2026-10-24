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

/// Whether a word is a colour: one of Lust's names, or #rgb / #rrggbb.
pub fn is_colour(v: &str) -> bool {
    if let Some(hex) = v.strip_prefix('#') {
        return matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    COLOURS.contains(&v)
}

#[cfg(all(test, feature = "cli"))]
mod same_as_the_drawing {
    #[test]
    fn colours() {
        assert_eq!(super::COLOURS, crate::native_ui::COLOURS);
        for w in ["red", "#abc", "#a1b2c3", "#abcd", "#xyz", "nope", "gray5", "white"] {
            assert_eq!(super::is_colour(w), crate::native_ui::colour(w).is_some(), "{w}");
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
