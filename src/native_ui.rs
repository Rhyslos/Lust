// Screens for Lust: views drawn by Lust itself, from the pixels up.
//
// A view is turned into a tree of Nodes each time it's drawn. This file lays the tree out,
// paints it into a buffer of pixels, opens the window and passes clicks back. Only three small
// libraries are borrowed: winit opens the window, softbuffer puts our pixels in it, and ab_glyph
// turns the font's letters into shapes. Everything else here is Lust's own.
//
// It goes into built programs that have a view, as `rt::ui`, and lust uses the same code for
// LustScript, so both draw exactly the same.
//
// With LUST_VIEW_TEST set, no window opens: commands come in on the program's input instead
// (dump, click "Start", snapshot file.png, size 400 300), which is how views are tested.

use std::collections::HashMap;

// ----- what a view gives: a tree of nodes -----

/// A style's value: `center` (a flag), `gap 12`, `color "#ff0000"`, or `hover:[ … ]`.
#[derive(Clone, Debug)]
pub enum SV {
    Flag,
    Num(f64),
    Text(String),
    Truth(bool),
    Nested(Vec<(String, SV)>),
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: String,
    pub text: String,
    pub style: Vec<(String, SV)>,
    pub children: Vec<Node>,
    /// Which of the view's actions a click runs, or an input's change.
    pub action: Option<usize>,
    /// What an input, checkbox or slider shows: the value it's bound to.
    pub value: Option<SV>,
    /// Which action Enter runs in an input.
    pub enter: Option<usize>,
}

impl Node {
    pub fn new(kind: &str) -> Node {
        Node { kind: kind.to_string(), text: String::new(), style: vec![], children: vec![], action: None, value: None, enter: None }
    }
}

/// Pieces that take no room on the screen: timers and keyboard shortcuts.
fn unseen(kind: &str) -> bool {
    kind == "every" || kind == "key"
}

// ----- keyboard shortcuts -----

/// A keyboard shortcut: `ctrl+shift+s`. Ctrl is Cmd on a Mac.
#[derive(Clone, Debug, PartialEq)]
pub struct Shortcut {
    command: bool,
    shift: bool,
    alt: bool,
    key: String,
}

const KEY_NAMES: &[&str] =
    &["enter", "escape", "tab", "space", "backspace", "delete", "up", "down", "left", "right", "home", "end", "pageup", "pagedown"];

fn function_key(k: &str) -> bool {
    k.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()).is_some_and(|n| (1..=24).contains(&n))
}

/// Reads a shortcut written like "ctrl+s", "shift+f5" or "escape".
pub fn parse_shortcut(spec: &str) -> Result<Shortcut, String> {
    let mut s = Shortcut { command: false, shift: false, alt: false, key: String::new() };
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
    Ok(s)
}

/// The view's keyboard shortcuts and the action each runs.
fn shortcuts(n: &Node, out: &mut Vec<(Shortcut, usize)>) {
    if n.kind == "key" {
        if let (Some(a), Ok(s)) = (n.action, parse_shortcut(&n.text)) {
            out.push((s, a));
        }
    }
    for c in &n.children {
        shortcuts(c, out);
    }
}

pub type Action = Box<dyn FnMut() -> Result<(), String>>;

thread_local! {
    /// The new value an input hands to its action, just before the action runs.
    static INCOMING: std::cell::RefCell<SV> = const { std::cell::RefCell::new(SV::Flag) };
}

/// Runs an input's action with its new value.
fn run_with(action: &mut Action, value: SV) -> Result<(), String> {
    INCOMING.with(|i| *i.borrow_mut() = value);
    action()
}

pub fn incoming() -> SV {
    INCOMING.with(|i| i.borrow().clone())
}

pub fn incoming_text() -> String {
    match incoming() {
        SV::Text(t) => t,
        _ => String::new(),
    }
}

pub fn incoming_num() -> f64 {
    match incoming() {
        SV::Num(n) => n,
        _ => 0.0,
    }
}

pub fn incoming_truth() -> bool {
    matches!(incoming(), SV::Truth(true))
}

// ----- fonts -----

pub const FONT_FILES: [&str; 4] = ["Inter-Regular.otf", "Inter-Medium.otf", "Inter-SemiBold.otf", "Inter-Bold.otf"];

pub static FONTS: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/Inter-Regular.otf"),
    include_bytes!("../assets/fonts/Inter-Medium.otf"),
    include_bytes!("../assets/fonts/Inter-SemiBold.otf"),
    include_bytes!("../assets/fonts/Inter-Bold.otf"),
];

/// A letter drawn at one size: where it sits from the pen and the baseline, how far the pen moves
/// on, and how much of each of its pixels is covered.
struct Glyph {
    left: i64,
    top: i64,
    width: usize,
    height: usize,
    coverage: Vec<u8>,
}

/// A font's shapes are read straight from its file when they're needed (ab_glyph), so a font
/// costs little more memory than its file. Faces 0 to 3 are Inter's four weights; fonts read from
/// files come after.
struct Faces {
    fonts: Vec<ab_glyph::FontArc>,
    /// Font files already read, by path, and the face each became (None if it couldn't be read).
    files: HashMap<String, Option<usize>>,
    /// Each letter's shape number and its width before scaling, by face.
    advances: HashMap<(usize, char), (ab_glyph::GlyphId, f32)>,
    glyphs: HashMap<(usize, char, u32), std::rc::Rc<Glyph>>,
}

thread_local! {
    static FACES: std::cell::RefCell<Faces> = std::cell::RefCell::new(Faces {
        fonts: FONTS.iter().map(|b| ab_glyph::FontArc::try_from_slice(b).expect("Lust's font")).collect(),
        files: HashMap::new(),
        advances: HashMap::new(),
        glyphs: HashMap::new(),
    });
    /// The window's own font, from its view's [font "…"], used by every piece without one.
    static WINDOW_FONT: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// Which of Inter's faces draws this weight.
fn weight_face(weight: f32) -> usize {
    if weight >= 650.0 {
        3
    } else if weight >= 550.0 {
        2
    } else if weight >= 450.0 {
        1
    } else {
        0
    }
}

/// A font file (TrueType or OpenType), read the first time it's asked for.
fn font_file(path: &str) -> Option<usize> {
    if let Some(f) = FACES.with(|f| f.borrow().files.get(path).copied()) {
        return f;
    }
    let font = std::fs::read(path).ok().and_then(|b| ab_glyph::FontArc::try_from_vec(b).ok());
    FACES.with(|f| {
        let mut f = f.borrow_mut();
        let face = font.map(|font| {
            f.fonts.push(font);
            f.fonts.len() - 1
        });
        f.files.insert(path.to_string(), face);
        face
    })
}

/// How many pixels one of the font's units is at this size.
fn per_unit(font: &ab_glyph::FontArc, size: f32) -> f32 {
    use ab_glyph::Font;
    size / font.units_per_em().unwrap_or(1000.0)
}

/// How wide a line of text is, in pixels at this size.
fn text_width(text: &str, size: f32, face: usize) -> f32 {
    use ab_glyph::Font;
    FACES.with(|f| {
        let f = &mut *f.borrow_mut();
        let font = &f.fonts[face];
        let k = per_unit(font, size);
        let mut w = 0.0;
        let mut prev: Option<ab_glyph::GlyphId> = None;
        for ch in text.chars() {
            let (id, adv) = *f.advances.entry((face, ch)).or_insert_with(|| {
                let id = font.glyph_id(ch);
                (id, font.h_advance_unscaled(id))
            });
            if let Some(p) = prev {
                w += font.kern_unscaled(p, id) * k;
            }
            w += adv * k;
            prev = Some(id);
        }
        w
    })
}

/// How far the pen moves on after one letter, in pixels at this size.
fn text_width_one(ch: char, size: f32, face: usize) -> f32 {
    use ab_glyph::Font;
    FACES.with(|f| {
        let f = &mut *f.borrow_mut();
        let font = &f.fonts[face];
        let (_, adv) = *f.advances.entry((face, ch)).or_insert_with(|| {
            let id = font.glyph_id(ch);
            (id, font.h_advance_unscaled(id))
        });
        adv * per_unit(font, size)
    })
}

/// (ascent, height of a line) at this size.
fn line_metrics(size: f32, face: usize) -> (f32, f32) {
    use ab_glyph::Font;
    FACES.with(|f| {
        let f = f.borrow();
        let font = &f.fonts[face];
        let k = per_unit(font, size);
        let (a, d, g) = (font.ascent_unscaled() * k, font.descent_unscaled() * k, font.line_gap_unscaled() * k);
        (a, (a - d + g).ceil())
    })
}

/// A letter's shape at a size in pixels, drawn once and kept.
fn glyph(face: usize, ch: char, px: f32) -> std::rc::Rc<Glyph> {
    use ab_glyph::Font;
    FACES.with(|f| {
        let f = &mut *f.borrow_mut();
        if let Some(g) = f.glyphs.get(&(face, ch, px.to_bits())) {
            return g.clone();
        }
        let font = &f.fonts[face];
        let scale = ab_glyph::PxScale::from(px * font.height_unscaled() / font.units_per_em().unwrap_or(1000.0));
        let g = font.glyph_id(ch).with_scale_and_position(scale, ab_glyph::point(0.0, 0.0));
        let made = match font.outline_glyph(g) {
            Some(o) => {
                let b = o.px_bounds();
                let (width, height) = (b.width() as usize, b.height() as usize);
                let mut coverage = vec![0u8; width * height];
                o.draw(|x, y, c| {
                    let i = y as usize * width + x as usize;
                    if i < coverage.len() {
                        coverage[i] = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
                    }
                });
                Glyph { left: b.min.x as i64, top: b.min.y as i64, width, height, coverage }
            }
            None => Glyph { left: 0, top: 0, width: 0, height: 0, coverage: vec![] },
        };
        let made = std::rc::Rc::new(made);
        f.glyphs.insert((face, ch, px.to_bits()), made.clone());
        made
    })
}

/// The kerning between two letters, in pixels at this size.
fn kern(face: usize, a: char, b: char, size: f32) -> f32 {
    use ab_glyph::Font;
    FACES.with(|f| {
        let f = f.borrow();
        let font = &f.fonts[face];
        font.kern_unscaled(font.glyph_id(a), font.glyph_id(b)) * per_unit(font, size)
    })
}

// ----- colours -----

fn named(name: &str) -> Option<u32> {
    Some(match name {
        "white" => 0xffffff,
        "black" => 0x000000,
        "gray1" => 0xf8f9fa,
        "gray2" => 0xf1f3f5,
        "gray3" => 0xe9ecef,
        "gray4" => 0xdee2e6,
        "gray5" => 0xced4da,
        "gray6" => 0xadb5bd,
        "gray7" => 0x868e96,
        "gray8" => 0x495057,
        "gray9" => 0x212529,
        "red" => 0xe03131,
        "orange" => 0xf76707,
        "yellow" => 0xf59f00,
        "green" => 0x2f9e44,
        "teal" => 0x0c8599,
        "blue" => 0x1c7ed6,
        "indigo" => 0x4263eb,
        "purple" => 0x7048e8,
        "pink" => 0xd6336c,
        "red1" => 0xffe3e3,
        "red2" => 0xffc9c9,
        "red3" => 0xffa8a8,
        "red4" => 0xff8787,
        "red5" => 0xff6b6b,
        "red6" => 0xfa5252,
        "red7" => 0xf03e3e,
        "red8" => 0xe03131,
        "red9" => 0xc92a2a,
        "orange1" => 0xffe8cc,
        "orange2" => 0xffd8a8,
        "orange3" => 0xffc078,
        "orange4" => 0xffa94d,
        "orange5" => 0xff922b,
        "orange6" => 0xfd7e14,
        "orange7" => 0xf76707,
        "orange8" => 0xe8590c,
        "orange9" => 0xd9480f,
        "yellow1" => 0xfff3bf,
        "yellow2" => 0xffec99,
        "yellow3" => 0xffe066,
        "yellow4" => 0xffd43b,
        "yellow5" => 0xfcc419,
        "yellow6" => 0xfab005,
        "yellow7" => 0xf59f00,
        "yellow8" => 0xf08c00,
        "yellow9" => 0xe67700,
        "green1" => 0xd3f9d8,
        "green2" => 0xb2f2bb,
        "green3" => 0x8ce99a,
        "green4" => 0x69db7c,
        "green5" => 0x51cf66,
        "green6" => 0x40c057,
        "green7" => 0x37b24d,
        "green8" => 0x2f9e44,
        "green9" => 0x2b8a3e,
        "teal1" => 0xc5f6fa,
        "teal2" => 0x99e9f2,
        "teal3" => 0x66d9e8,
        "teal4" => 0x3bc9db,
        "teal5" => 0x22b8cf,
        "teal6" => 0x15aabf,
        "teal7" => 0x1098ad,
        "teal8" => 0x0c8599,
        "teal9" => 0x0b7285,
        "blue1" => 0xd0ebff,
        "blue2" => 0xa5d8ff,
        "blue3" => 0x74c0fc,
        "blue4" => 0x4dabf7,
        "blue5" => 0x339af0,
        "blue6" => 0x228be6,
        "blue7" => 0x1c7ed6,
        "blue8" => 0x1971c2,
        "blue9" => 0x1864ab,
        "indigo1" => 0xdbe4ff,
        "indigo2" => 0xbac8ff,
        "indigo3" => 0x91a7ff,
        "indigo4" => 0x748ffc,
        "indigo5" => 0x5c7cfa,
        "indigo6" => 0x4c6ef5,
        "indigo7" => 0x4263eb,
        "indigo8" => 0x3b5bdb,
        "indigo9" => 0x364fc7,
        "purple1" => 0xe5dbff,
        "purple2" => 0xd0bfff,
        "purple3" => 0xb197fc,
        "purple4" => 0x9775fa,
        "purple5" => 0x845ef7,
        "purple6" => 0x7950f2,
        "purple7" => 0x7048e8,
        "purple8" => 0x6741d9,
        "purple9" => 0x5f3dc4,
        "pink1" => 0xffdeeb,
        "pink2" => 0xfcc2d7,
        "pink3" => 0xfaa2c1,
        "pink4" => 0xf783ac,
        "pink5" => 0xf06595,
        "pink6" => 0xe64980,
        "pink7" => 0xd6336c,
        "pink8" => 0xc2255c,
        "pink9" => 0xa61e4d,
        _ => return None,
    })
}

pub const COLOURS: &[&str] = &[
    "white", "black", "gray1", "gray2", "gray3", "gray4", "gray5", "gray6", "gray7", "gray8", "gray9", "red", "orange", "yellow", "green",
    "teal", "blue", "indigo", "purple", "pink", "red1", "red2", "red3", "red4", "red5", "red6", "red7", "red8", "red9", "orange1", "orange2", "orange3", "orange4", "orange5", "orange6", "orange7", "orange8", "orange9", "yellow1", "yellow2", "yellow3", "yellow4", "yellow5", "yellow6", "yellow7", "yellow8", "yellow9", "green1", "green2", "green3", "green4", "green5", "green6", "green7", "green8", "green9", "teal1", "teal2", "teal3", "teal4", "teal5", "teal6", "teal7", "teal8", "teal9", "blue1", "blue2", "blue3", "blue4", "blue5", "blue6", "blue7", "blue8", "blue9", "indigo1", "indigo2", "indigo3", "indigo4", "indigo5", "indigo6", "indigo7", "indigo8", "indigo9", "purple1", "purple2", "purple3", "purple4", "purple5", "purple6", "purple7", "purple8", "purple9", "pink1", "pink2", "pink3", "pink4", "pink5", "pink6", "pink7", "pink8", "pink9",
];

pub fn colour(v: &str) -> Option<u32> {
    if let Some(hex) = v.strip_prefix('#') {
        let full = match hex.len() {
            3 => hex.chars().flat_map(|c| [c, c]).collect::<String>(),
            6 => hex.to_string(),
            _ => return None,
        };
        return u32::from_str_radix(&full, 16).ok();
    }
    named(v)
}

const INK: u32 = 0x1f2328;
const MUTED: u32 = 0x6b7280;

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ch = |s: u32| -> [f32; 3] { [((s >> 16) & 255) as f32, ((s >> 8) & 255) as f32, (s & 255) as f32] };
    let (x, y) = (ch(a), ch(b));
    let c = |i: usize| (x[i] + (y[i] - x[i]) * t).round().clamp(0.0, 255.0) as u32;
    (c(0) << 16) | (c(1) << 8) | c(2)
}

// ----- styles, worked out -----

#[derive(Clone, Copy, PartialEq, Debug)]
enum Size {
    Auto,
    Fixed(f32),
    Fill,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Align {
    Start,
    Center,
    End,
}

#[derive(Clone, Debug)]
struct Look {
    /// top, right, bottom, left
    pad: [f32; 4],
    gap: f32,
    width: Size,
    height: Size,
    /// A share of the leftover room along its row or column; `width fill` is grow 1.
    grow: Option<f32>,
    min_w: f32,
    max_w: f32,
    min_h: f32,
    max_h: f32,
    size: f32,
    weight: f32,
    color: u32,
    background: Option<u32>,
    radius: f32,
    border: Option<u32>,
    border_w: f32,
    shadow: bool,
    /// left, center, right: for children (and a text's lines) side to side.
    halign: Align,
    /// top, middle, bottom: for children up and down.
    valign: Align,
    spread: bool,
    wrap: bool,
    strike: bool,
    opacity: f32,
    /// The colour of a checked checkbox, a slider's filled part, and the ring around what has focus.
    accent: u32,
    /// What an empty input shows.
    placeholder: String,
    /// A slider's range and how far one step goes.
    from: f32,
    to: f32,
    step: f32,
    /// A picture fills its box, cut to fit, instead of fitting inside it.
    cover: bool,
    /// Greyed out, and not clicked, typed in or focused.
    disabled: bool,
    /// How much of the background shows, while it fades in or out.
    bg_alpha: f32,
    /// A font read from a file, instead of Inter.
    font: Option<usize>,
}

impl Look {
    /// The face its words are drawn in: its own font, or Inter at its weight.
    fn face(&self) -> usize {
        self.font.unwrap_or_else(|| weight_face(self.weight))
    }
}

fn num(v: &SV) -> Option<f32> {
    match v {
        SV::Num(n) => Some(*n as f32),
        _ => None,
    }
}

fn text_of(v: &SV) -> Option<&str> {
    match v {
        SV::Text(s) => Some(s),
        _ => None,
    }
}

/// Light colours get dark words on them, dark colours white ones.
fn light(c: u32) -> bool {
    let (r, g, b) = (((c >> 16) & 255) as f32, ((c >> 8) & 255) as f32, (c & 255) as f32);
    (0.299 * r + 0.587 * g + 0.114 * b) / 255.0 > 0.6
}

fn base_look(kind: &str) -> Look {
    let mut l = Look {
        pad: [0.0; 4],
        gap: 0.0,
        width: Size::Auto,
        height: Size::Auto,
        grow: None,
        min_w: 0.0,
        max_w: f32::INFINITY,
        min_h: 0.0,
        max_h: f32::INFINITY,
        size: 15.0,
        weight: 400.0,
        color: INK,
        background: None,
        radius: 0.0,
        border: None,
        border_w: 1.0,
        shadow: false,
        halign: Align::Start,
        valign: Align::Start,
        spread: false,
        wrap: true,
        strike: false,
        opacity: 1.0,
        accent: 0x1c7ed6,
        placeholder: String::new(),
        from: 0.0,
        to: 100.0,
        step: 1.0,
        cover: false,
        disabled: false,
        bg_alpha: 1.0,
        font: WINDOW_FONT.with(|f| f.get()),
    };
    match kind {
        "input" => {
            l.pad = [7.0, 10.0, 7.0, 10.0];
            l.width = Size::Fixed(220.0);
            l.background = Some(0xffffff);
            l.border = Some(0xced4da);
            l.radius = 6.0;
        }
        "checkbox" => {
            l.width = Size::Fixed(18.0);
            l.height = Size::Fixed(18.0);
            l.background = Some(0xffffff);
            l.border = Some(0xadb5bd);
            l.border_w = 1.5;
            l.radius = 4.0;
        }
        "slider" => {
            l.width = Size::Fixed(200.0);
            l.height = Size::Fixed(20.0);
        }
        "scroll" => l.width = Size::Fill,
        "button" => {
            l.pad = [7.0, 14.0, 7.0, 14.0];
            l.background = Some(0xe9ecef);
            l.radius = 6.0;
            l.weight = 500.0;
            l.halign = Align::Center;
        }
        // A row's children sit in the middle, up and down.
        "row" => l.valign = Align::Center,
        _ => {}
    }
    l
}

fn apply(l: &mut Look, kind: &str, style: &[(String, SV)]) {
    for (k, v) in style {
        // A style that's on only when something is true, and it isn't.
        if let SV::Truth(false) = v {
            continue;
        }
        match k.as_str() {
            "pad" => l.pad = [num(v).unwrap_or(8.0); 4],
            "padX" => {
                let p = num(v).unwrap_or(8.0);
                l.pad[1] = p;
                l.pad[3] = p;
            }
            "padY" => {
                let p = num(v).unwrap_or(8.0);
                l.pad[0] = p;
                l.pad[2] = p;
            }
            "padTop" => l.pad[0] = num(v).unwrap_or(8.0),
            "padRight" => l.pad[1] = num(v).unwrap_or(8.0),
            "padBottom" => l.pad[2] = num(v).unwrap_or(8.0),
            "padLeft" => l.pad[3] = num(v).unwrap_or(8.0),
            "gap" => l.gap = num(v).unwrap_or(8.0),
            "width" | "height" => {
                let s = match v {
                    SV::Num(n) => Size::Fixed(*n as f32),
                    SV::Text(t) if t == "fill" => Size::Fill,
                    SV::Flag => Size::Fill,
                    _ => Size::Auto,
                };
                if k == "width" {
                    l.width = s
                } else {
                    l.height = s
                }
            }
            "grow" => l.grow = Some(num(v).unwrap_or(1.0).max(0.0)),
            "minWidth" => l.min_w = num(v).unwrap_or(0.0),
            "maxWidth" => l.max_w = num(v).unwrap_or(f32::INFINITY),
            "minHeight" => l.min_h = num(v).unwrap_or(0.0),
            "maxHeight" => l.max_h = num(v).unwrap_or(f32::INFINITY),
            "size" => l.size = num(v).unwrap_or(15.0),
            "tiny" => l.size = 11.0,
            "small" => l.size = 13.0,
            "large" => l.size = 18.0,
            "huge" => l.size = 24.0,
            "weight" => l.weight = num(v).unwrap_or(400.0),
            "bold" => l.weight = 700.0,
            "muted" => l.color = MUTED,
            "strike" => l.strike = true,
            "nowrap" => l.wrap = false,
            "color" => {
                if let Some(c) = text_of(v).and_then(colour) {
                    l.color = c
                }
            }
            "background" => l.background = text_of(v).and_then(colour),
            "round" => l.radius = num(v).unwrap_or(8.0),
            "pill" => l.radius = f32::INFINITY,
            "border" => match v {
                SV::Num(n) => {
                    l.border_w = *n as f32;
                    l.border = Some(l.border.unwrap_or(0xdee2e6));
                }
                _ => l.border = Some(text_of(v).and_then(colour).unwrap_or(0xdee2e6)),
            },
            "borderWidth" => {
                l.border_w = num(v).unwrap_or(1.0);
                l.border = Some(l.border.unwrap_or(0xdee2e6));
            }
            "shadow" => l.shadow = true,
            "opacity" => l.opacity = num(v).unwrap_or(1.0).clamp(0.0, 1.0),
            "left" => l.halign = Align::Start,
            "center" => l.halign = Align::Center,
            "right" => l.halign = Align::End,
            "top" => l.valign = Align::Start,
            "middle" => l.valign = Align::Center,
            "bottom" => l.valign = Align::End,
            "spread" => l.spread = true,
            "subtle" => l.background = None,
            "placeholder" => l.placeholder = text_of(v).unwrap_or("").to_string(),
            "from" => l.from = num(v).unwrap_or(0.0),
            "to" => l.to = num(v).unwrap_or(100.0),
            "step" => l.step = num(v).unwrap_or(1.0).max(0.0),
            "cover" => l.cover = true,
            "disabled" => l.disabled = true,
            "font" => {
                if let Some(f) = text_of(v).and_then(font_file) {
                    l.font = Some(f);
                }
            }
            other => {
                // A colour on its own: a button's colour, or text's.
                if let Some(c) = colour(other) {
                    if kind == "button" {
                        l.background = Some(c);
                        l.color = if light(c) { INK } else { 0xffffff };
                    } else if matches!(kind, "checkbox" | "slider" | "input") {
                        l.accent = c;
                    } else {
                        l.color = c;
                    }
                }
            }
        }
    }
}

fn hover_style(style: &[(String, SV)]) -> Option<&Vec<(String, SV)>> {
    style.iter().find_map(|(k, v)| match (k.as_str(), v) {
        ("hover", SV::Nested(n)) => Some(n),
        _ => None,
    })
}

// ----- text in lines -----

/// Breaks text into lines: at each \n, and between words where a line would be wider than
/// `max_w`. A word too long for a line on its own is broken between letters.
fn wrap_lines(text: &str, size: f32, face: usize, max_w: f32, wrap: bool) -> Vec<String> {
    let fits = |s: &str| text_width(s, size, face) <= max_w + 0.5;
    let mut out = Vec::new();
    for para in text.split('\n') {
        if !wrap || !max_w.is_finite() || fits(para) {
            out.push(para.to_string());
            continue;
        }
        let mut line = String::new();
        for word in para.split(' ') {
            let joined = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if fits(&joined) {
                line = joined;
                continue;
            }
            if !line.is_empty() {
                out.push(std::mem::take(&mut line));
            }
            // The word starts a line of its own; if it's still too wide, break it up.
            for ch in word.chars() {
                let mut longer = line.clone();
                longer.push(ch);
                if !line.is_empty() && !fits(&longer) {
                    out.push(std::mem::take(&mut line));
                    line.push(ch);
                } else {
                    line = longer;
                }
            }
        }
        out.push(line);
    }
    out
}

// ----- pictures -----

/// A picture read from a file: its size, and its pixels as red, green, blue and see-through.
struct Picture {
    w: usize,
    h: usize,
    px: Vec<[u8; 4]>,
}

thread_local! {
    // Each file is read once. A file that can't be read is remembered as missing.
    static PICTURES: std::cell::RefCell<HashMap<String, Option<std::rc::Rc<Picture>>>> = std::cell::RefCell::new(HashMap::new());
}

fn picture(path: &str) -> Option<std::rc::Rc<Picture>> {
    if let Some(p) = PICTURES.with(|c| c.borrow().get(path).cloned()) {
        return p;
    }
    let p = read_picture(path).map(std::rc::Rc::new);
    PICTURES.with(|c| c.borrow_mut().insert(path.to_string(), p.clone()));
    p
}

/// PNG and JPEG files.
fn read_picture(path: &str) -> Option<Picture> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.starts_with(b"\x89PNG") {
        let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
        dec.set_transformations(png::Transformations::normalize_to_color8());
        let mut r = dec.read_info().ok()?;
        let mut buf = vec![0; r.output_buffer_size()];
        let info = r.next_frame(&mut buf).ok()?;
        let (w, h) = (info.width as usize, info.height as usize);
        let px: Vec<[u8; 4]> = match info.color_type {
            png::ColorType::Rgba => buf.chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect(),
            png::ColorType::Rgb => buf.chunks_exact(3).map(|c| [c[0], c[1], c[2], 255]).collect(),
            png::ColorType::GrayscaleAlpha => buf.chunks_exact(2).map(|c| [c[0], c[0], c[0], c[1]]).collect(),
            png::ColorType::Grayscale => buf.iter().map(|&g| [g, g, g, 255]).collect(),
            _ => return None,
        };
        return (px.len() >= w * h).then_some(Picture { w, h, px });
    }
    let mut dec = jpeg_decoder::Decoder::new(std::io::Cursor::new(bytes));
    let data = dec.decode().ok()?;
    let info = dec.info()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let px: Vec<[u8; 4]> = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => data.chunks_exact(3).map(|c| [c[0], c[1], c[2], 255]).collect(),
        jpeg_decoder::PixelFormat::L8 => data.iter().map(|&g| [g, g, g, 255]).collect(),
        jpeg_decoder::PixelFormat::CMYK32 => data
            .chunks_exact(4)
            .map(|c| {
                let k = 255 - c[3] as u32;
                let f = |x: u8| ((255 - x as u32) * k / 255) as u8;
                [f(c[0]), f(c[1]), f(c[2]), 255]
            })
            .collect(),
        _ => return None,
    };
    (px.len() >= w * h).then_some(Picture { w, h, px })
}

// ----- layout -----

/// A node, worked out and placed.
struct Laid {
    kind: String,
    text: String,
    /// A text's or button's words, broken into the lines it shows.
    lines: Vec<String>,
    look: Look,
    hover: Option<Look>,
    action: Option<usize>,
    /// What Enter runs in an input.
    enter: Option<usize>,
    /// What an input, checkbox or slider shows.
    value: Option<SV>,
    /// A scroll's content, top to bottom, and how far it's scrolled down.
    content_h: f32,
    scrolled: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    children: Vec<Laid>,
}

fn lay(n: &Node) -> Laid {
    let mut look = base_look(&n.kind);
    apply(&mut look, &n.kind, &n.style);
    let disabled = look.disabled;
    if disabled {
        look.opacity *= 0.45;
    }
    let hover = match hover_style(&n.style) {
        _ if disabled => None,
        Some(h) => {
            let mut hl = look.clone();
            apply(&mut hl, &n.kind, h);
            Some(hl)
        }
        None if n.kind == "button" => {
            let mut hl = look.clone();
            hl.background = Some(match look.background {
                Some(b) => mix(b, 0x000000, 0.08),
                None => 0xf1f3f5,
            });
            Some(hl)
        }
        None => None,
    };
    Laid {
        kind: n.kind.clone(),
        text: n.text.clone(),
        lines: vec![],
        look,
        hover,
        action: if disabled { None } else { n.action },
        value: n.value.clone(),
        content_h: 0.0,
        scrolled: 0.0,
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
        enter: if disabled { None } else { n.enter },
        // Timers and shortcuts take no room on the screen.
        children: n.children.iter().filter(|c| !unseen(&c.kind)).map(lay).collect(),
    }
}

fn shows_text(kind: &str) -> bool {
    kind == "text" || kind == "button"
}

/// How much of the leftover room a child takes along its row (or column), if any.
fn flex(c: &Laid, row: bool) -> Option<f32> {
    if let Some(g) = c.look.grow {
        return if g > 0.0 { Some(g) } else { None };
    }
    let s = if row { c.look.width } else { c.look.height };
    // A scroll takes the room left in a column, so there's something to scroll, and so does
    // whatever holds one.
    if s == Size::Fill || c.kind == "space" || (!row && s == Size::Auto && holds_scroll(c)) {
        Some(1.0)
    } else {
        None
    }
}

/// Whether this is a scroll, or holds one (not counting scrolls inside scrolls).
fn holds_scroll(n: &Laid) -> bool {
    n.kind == "scroll" || n.children.iter().any(holds_scroll)
}

/// Rows told `center`, `right` or `spread`, and columns and stacks told `center` or `right`, are
/// as wide as they can be, so there's room to place their children in.
fn wants_width(n: &Laid) -> bool {
    match n.kind.as_str() {
        "row" => n.look.halign != Align::Start || n.look.spread,
        "column" | "stack" => n.look.halign != Align::Start,
        _ => false,
    }
}

/// Columns in a row told `middle`, `bottom` or `spread` are as tall as the row.
fn wants_height(n: &Laid) -> bool {
    match n.kind.as_str() {
        "column" => n.look.valign != Align::Start || n.look.spread,
        "stack" => n.look.valign != Align::Start,
        _ => false,
    }
}

fn clamp_w(c: &Laid, w: f32) -> f32 {
    w.min(c.look.max_w).max(c.look.min_w)
}

fn clamp_h(c: &Laid, h: f32) -> f32 {
    h.min(c.look.max_h).max(c.look.min_h)
}

thread_local! {
    // Sizes already measured while laying out, by node and the width it may use.
    static MEASURED: std::cell::RefCell<HashMap<(usize, u32), (f32, f32)>> = std::cell::RefCell::new(HashMap::new());
}

/// The size a node would like to be when it may be at most `max_w` wide.
fn natural(n: &Laid, max_w: f32) -> (f32, f32) {
    let key = (n as *const Laid as usize, max_w.to_bits());
    if let Some(s) = MEASURED.with(|m| m.borrow().get(&key).copied()) {
        return s;
    }
    let s = measure(n, max_w);
    MEASURED.with(|m| m.borrow_mut().insert(key, s));
    s
}

fn measure(n: &Laid, max_w: f32) -> (f32, f32) {
    let l = &n.look;
    let (padx, pady) = (l.pad[1] + l.pad[3], l.pad[0] + l.pad[2]);
    let outer = clamp_w(n, if let Size::Fixed(f) = l.width { f } else { max_w });
    let inner = (outer - padx).max(0.0);
    let gaps = l.gap * (n.children.len().saturating_sub(1)) as f32;
    let (mut w, mut h) = match n.kind.as_str() {
        k if shows_text(k) => {
            let lines = wrap_lines(&n.text, l.size, l.face(), inner, l.wrap);
            let (_, lh) = line_metrics(l.size, l.face());
            let w = lines.iter().map(|s| text_width(s, l.size, l.face())).fold(0.0, f32::max);
            (w.ceil(), lh * lines.len() as f32)
        }
        "space" | "checkbox" | "slider" => (0.0, 0.0),
        "image" => {
            let (pw, ph) = picture(&n.text).map(|p| (p.w as f32, p.h as f32)).unwrap_or((64.0, 64.0));
            let ratio = ph / pw.max(1.0);
            let w = match (l.width, l.height) {
                (Size::Fixed(f), _) => f - padx,
                (Size::Fill, _) if inner.is_finite() => inner,
                (Size::Auto, Size::Fixed(g)) => (g - pady) / ratio.max(1e-6),
                _ => pw.min(inner),
            };
            let h = match l.height {
                Size::Fixed(g) => g - pady,
                _ => w * ratio,
            };
            (w.max(0.0), h.max(0.0))
        }
        "input" => (0.0, line_metrics(l.size, l.face()).1),
        "row" => {
            let sizes = row_sizes(n, inner);
            (sizes.iter().map(|s| s.0).sum::<f32>() + gaps, sizes.iter().map(|s| s.1).fold(0.0, f32::max))
        }
        "stack" => n.children.iter().map(|c| natural(c, inner)).fold((0.0f32, 0.0f32), |a, s| (a.0.max(s.0), a.1.max(s.1))),
        _ => {
            let sizes: Vec<(f32, f32)> = n.children.iter().map(|c| natural(c, inner)).collect();
            (sizes.iter().map(|s| s.0).fold(0.0, f32::max), sizes.iter().map(|s| s.1).sum::<f32>() + gaps)
        }
    };
    // A group told where to put its children side to side takes the whole width to do it in.
    if wants_width(n) && inner.is_finite() {
        w = w.max(inner);
    }
    w += padx;
    h += pady;
    if let Size::Fixed(f) = l.width {
        w = f;
    }
    if let Size::Fixed(f) = l.height {
        h = f;
    }
    (clamp_w(n, w), clamp_h(n, h))
}

/// Each child's width and height in a row `iw` wide: children that fill share what's left.
fn row_sizes(n: &Laid, iw: f32) -> Vec<(f32, f32)> {
    let gaps = n.look.gap * (n.children.len().saturating_sub(1)) as f32;
    let widths: Vec<Option<f32>> =
        n.children.iter().map(|c| if flex(c, true).is_some() { None } else { Some(natural(c, iw).0) }).collect();
    let fixed: f32 = widths.iter().flatten().sum();
    let total: f32 = n.children.iter().filter_map(|c| flex(c, true)).sum();
    let room = (iw - fixed - gaps).max(0.0);
    n.children
        .iter()
        .zip(widths)
        .map(|(c, w)| {
            let w = match w {
                Some(w) => w,
                None if iw.is_finite() => clamp_w(c, room * flex(c, true).unwrap_or(0.0) / total),
                None => natural(c, f32::INFINITY).0,
            };
            let h = match c.look.height {
                Size::Fixed(f) => clamp_h(c, f),
                _ => natural(c, w).1,
            };
            (w, h)
        })
        .collect()
}

/// Where a piece goes in `room` to spare: at the start, the middle or the end.
fn offset(a: Align, room: f32) -> f32 {
    match a {
        Align::Start => 0.0,
        Align::Center => (room / 2.0).max(0.0),
        Align::End => room.max(0.0),
    }
}

/// Along a row or column: where the first child starts, and extra room between children.
fn spread_out(a: Align, spread: bool, room: f32, count: usize) -> (f32, f32) {
    if room <= 0.0 {
        return (0.0, 0.0);
    }
    if spread {
        return if count > 1 { (0.0, room / (count - 1) as f32) } else { (0.0, 0.0) };
    }
    (offset(a, room), 0.0)
}

/// A child's size across a column or stack `iw` wide.
fn cross_width(c: &Laid, iw: f32) -> f32 {
    clamp_w(
        c,
        match c.look.width {
            Size::Fill => iw,
            Size::Fixed(f) => f,
            Size::Auto => natural(c, iw).0.min(iw),
        },
    )
}

/// Places a node in the space it's given, then its children inside it.
fn place(n: &mut Laid, x: f32, y: f32, w: f32, h: f32) {
    n.x = x;
    n.y = y;
    n.w = w;
    n.h = h;
    let l = n.look.clone();
    let (ix, iy) = (x + l.pad[3], y + l.pad[0]);
    let (iw, ih) = ((w - l.pad[1] - l.pad[3]).max(0.0), (h - l.pad[0] - l.pad[2]).max(0.0));
    if shows_text(&n.kind) {
        n.lines = wrap_lines(&n.text, l.size, l.face(), iw, l.wrap);
        return;
    }
    if n.children.is_empty() {
        return;
    }
    let count = n.children.len();
    match n.kind.as_str() {
        "stack" => {
            for c in n.children.iter_mut() {
                let cw = cross_width(c, iw);
                let ch = clamp_h(
                    c,
                    match c.look.height {
                        Size::Fill => ih,
                        Size::Fixed(f) => f,
                        Size::Auto => natural(c, cw).1,
                    },
                );
                let (cx, cy) = (ix + offset(l.halign, iw - cw), iy + offset(l.valign, ih - ch));
                place(c, cx.round(), cy.round(), cw.round(), ch.round());
            }
        }
        "scroll" => {
            // Like a column, but as tall as its content: what doesn't fit is scrolled to.
            let mut at = 0.0;
            for c in n.children.iter_mut() {
                let cw = cross_width(c, iw);
                let ch = match c.look.height {
                    Size::Fixed(f) => clamp_h(c, f),
                    _ => natural(c, cw).1,
                };
                let cx = ix + offset(l.halign, iw - cw);
                place(c, cx.round(), (iy + at).round(), cw.round(), ch.round());
                at += ch + l.gap;
            }
            n.content_h = (at - l.gap).max(0.0) + l.pad[0] + l.pad[2];
        }
        "row" => {
            let sizes = row_sizes(n, iw);
            let used: f32 = sizes.iter().map(|s| s.0).sum::<f32>() + l.gap * (count - 1) as f32;
            let (mut at, extra) = spread_out(l.halign, l.spread, iw - used, count);
            for (c, (cw, nh)) in n.children.iter_mut().zip(sizes) {
                let ch = clamp_h(
                    c,
                    match c.look.height {
                        Size::Fill => ih,
                        Size::Fixed(f) => f,
                        Size::Auto if wants_height(c) => ih,
                        Size::Auto => nh.min(ih),
                    },
                );
                let cy = iy + offset(l.valign, ih - ch);
                place(c, (ix + at).round(), cy.round(), cw.round(), ch.round());
                at += cw + l.gap + extra;
            }
        }
        _ => {
            // A column, and the view itself.
            let widths: Vec<f32> = n.children.iter().map(|c| cross_width(c, iw)).collect();
            let heights: Vec<Option<f32>> = n
                .children
                .iter()
                .zip(&widths)
                .map(|(c, &cw)| match (flex(c, false), c.look.height) {
                    (Some(_), _) => None,
                    (None, Size::Fixed(f)) => Some(clamp_h(c, f)),
                    (None, _) => Some(natural(c, cw).1),
                })
                .collect();
            let gaps = l.gap * (count - 1) as f32;
            let fixed: f32 = heights.iter().flatten().sum();
            let total: f32 = n.children.iter().filter_map(|c| flex(c, false)).sum();
            let room = (ih - fixed - gaps).max(0.0);
            let heights: Vec<f32> = n
                .children
                .iter()
                .zip(heights)
                .map(|(c, h)| h.unwrap_or_else(|| clamp_h(c, room * flex(c, false).unwrap_or(0.0) / total)))
                .collect();
            let used: f32 = heights.iter().sum::<f32>() + gaps;
            let (mut at, extra) = spread_out(l.valign, l.spread, ih - used, count);
            for ((c, cw), ch) in n.children.iter_mut().zip(widths).zip(heights) {
                let cx = ix + offset(l.halign, iw - cw);
                place(c, cx.round(), (iy + at).round(), cw.round(), ch.round());
                at += ch + l.gap + extra;
            }
        }
    }
}

/// The view's top: a column as big as the window. Columns and rows directly in it are as wide as
/// the window.
fn lay_out(root: &Node, w: f32, h: f32) -> Laid {
    WINDOW_FONT.with(|f| f.set(settings(&root.style).font));
    let mut top = lay(root);
    // The root carries the window's settings, which aren't styles.
    top.look = base_look("view");
    for c in &mut top.children {
        if (c.kind == "column" || c.kind == "row" || c.kind == "piece") && c.look.width == Size::Auto {
            c.look.width = Size::Fill;
        }
    }
    MEASURED.with(|m| m.borrow_mut().clear());
    place(&mut top, 0.0, 0.0, w, h);
    MEASURED.with(|m| m.borrow_mut().clear());
    top
}

// ----- painting -----

struct Canvas {
    w: usize,
    h: usize,
    scale: f32,
    /// How see-through everything painted now is (opacity), 1 for solid.
    alpha: f32,
    /// Only pixels inside this box (in pixels: left, top, right, bottom) are painted.
    clip: Option<[i64; 4]>,
    px: Vec<u32>,
}

/// How far a point is outside a rounded rectangle: below zero inside.
fn outside(fx: f32, fy: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> f32 {
    let qx = (x0 + r - fx).max(fx - (x1 - r)).max(0.0);
    let qy = (y0 + r - fy).max(fy - (y1 - r)).max(0.0);
    if r > 0.0 && qx > 0.0 && qy > 0.0 {
        (qx * qx + qy * qy).sqrt() - r
    } else {
        let ex = (x0 - fx).max(fx - x1);
        let ey = (y0 - fy).max(fy - y1);
        ex.max(ey).max(if r > 0.0 { qx.max(qy) - r } else { f32::MIN })
    }
}

impl Canvas {
    fn new(w: usize, h: usize, scale: f32) -> Canvas {
        Canvas { w, h, scale, alpha: 1.0, clip: None, px: vec![0xffffff; w * h] }
    }

    fn blend(&mut self, x: i64, y: i64, c: u32, a: f32) {
        let a = a * self.alpha;
        if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 || a <= 0.0 {
            return;
        }
        if let Some([x0, y0, x1, y1]) = self.clip {
            if x < x0 || y < y0 || x >= x1 || y >= y1 {
                return;
            }
        }
        let i = y as usize * self.w + x as usize;
        self.px[i] = if a >= 1.0 { c } else { mix(self.px[i], c, a) };
    }

    /// A rectangle with rounded corners, smooth at the edges. Sizes are in the view's units.
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: u32, alpha: f32) {
        self.shape(x, y, w, h, r, None, c, alpha);
    }

    /// A rounded rectangle, or with `ring` only its edge that many units thick.
    #[allow(clippy::too_many_arguments)]
    fn shape(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, ring: Option<f32>, c: u32, alpha: f32) {
        let s = self.scale;
        let (x0, y0, x1, y1) = (x * s, y * s, (x + w) * s, (y + h) * s);
        let r = (r * s).min((x1 - x0) / 2.0).min((y1 - y0) / 2.0).max(0.0);
        let b = ring.map(|b| (b * s).max(0.0));
        // Only the pixels that can show are visited.
        let [bx0, by0, bx1, by1] = self.bounds();
        let (px0, px1) = ((x0.floor() as i64).max(bx0), (x1.ceil() as i64).min(bx1));
        let (py0, py1) = ((y0.floor() as i64).max(by0), (y1.ceil() as i64).min(by1));
        if px0 >= px1 || py0 >= py1 {
            return;
        }
        let cover_at = |px: i64, py: i64| {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let mut cover = (0.5 - outside(fx, fy, x0, y0, x1, y1, r)).clamp(0.0, 1.0);
            if let Some(b) = b {
                let inner = outside(fx, fy, x0 + b, y0 + b, x1 - b, y1 - b, (r - b).max(0.0));
                cover *= 1.0 - (0.5 - inner).clamp(0.0, 1.0);
            }
            cover
        };
        // Away from the left and right edges (by more than the corners and the ring), how much
        // of a pixel is covered depends only on its row, so it's worked out once a row.
        let m = r.max(b.unwrap_or(0.0)) + 1.0;
        let (s0, s1) = (((x0 + m - 0.5).ceil() as i64).max(px0), ((x1 - m - 0.5).floor() as i64 + 1).min(px1));
        for py in py0..py1 {
            if s0 >= s1 {
                for px in px0..px1 {
                    let cv = cover_at(px, py);
                    self.blend(px, py, c, cv * alpha);
                }
                continue;
            }
            for px in (px0..s0).chain(s1..px1) {
                let cv = cover_at(px, py);
                self.blend(px, py, c, cv * alpha);
            }
            let a = cover_at(s0, py) * alpha;
            if a <= 0.0 {
                continue;
            }
            if a * self.alpha >= 1.0 && self.clip.map_or(true, |k| py >= k[1] && py < k[3]) {
                let row = py as usize * self.w;
                self.px[row + s0 as usize..row + s1 as usize].fill(c);
            } else {
                for px in s0..s1 {
                    self.blend(px, py, c, a);
                }
            }
        }
    }

    /// The pixels that can be painted now: the canvas, within the clip.
    fn bounds(&self) -> [i64; 4] {
        let mut b = [0, 0, self.w as i64, self.h as i64];
        if let Some(c) = self.clip {
            b = [b[0].max(c[0]), b[1].max(c[1]), b[2].min(c[2]), b[3].min(c[3])];
        }
        b
    }

    /// Paints only inside this box (in the view's units) until `unclip`, within any box already set.
    fn clip_to(&mut self, x: f32, y: f32, w: f32, h: f32) -> Option<[i64; 4]> {
        let s = self.scale;
        let mut b = [(x * s).floor() as i64, (y * s).floor() as i64, ((x + w) * s).ceil() as i64, ((y + h) * s).ceil() as i64];
        if let Some(o) = self.clip {
            b = [b[0].max(o[0]), b[1].max(o[1]), b[2].min(o[2]), b[3].min(o[3])];
        }
        std::mem::replace(&mut self.clip, Some(b))
    }

    /// A straight line `width` thick with round ends, smooth at the edges.
    #[allow(clippy::too_many_arguments)]
    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, c: u32) {
        let s = self.scale;
        let (ax, ay, bx, by, r) = (x0 * s, y0 * s, x1 * s, y1 * s, width * s / 2.0);
        let (dx, dy) = (bx - ax, by - ay);
        let len2 = (dx * dx + dy * dy).max(1e-6);
        for py in ((ay.min(by) - r).floor() as i64)..((ay.max(by) + r).ceil() as i64) {
            for px in ((ax.min(bx) - r).floor() as i64)..((ax.max(bx) + r).ceil() as i64) {
                let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                let t = (((fx - ax) * dx + (fy - ay) * dy) / len2).clamp(0.0, 1.0);
                let (qx, qy) = (ax + t * dx - fx, ay + t * dy - fy);
                let d = (qx * qx + qy * qy).sqrt() - r;
                self.blend(px, py, c, (0.5 - d).clamp(0.0, 1.0));
            }
        }
    }

    fn text(&mut self, text: &str, x: f32, y: f32, size: f32, face: usize, c: u32) {
        let s = self.scale;
        let px_size = size * s;
        let (ascent, _) = line_metrics(px_size, face);
        let mut pen = x * s;
        let base = (y * s + ascent).round();
        // A line that's out of sight is skipped whole.
        let [_, by0, _, by1] = self.bounds();
        if (base + px_size) < by0 as f32 || (base - px_size * 1.2) > by1 as f32 {
            return;
        }
        let mut prev: Option<char> = None;
        for ch in text.chars() {
            if let Some(p) = prev {
                pen += kern(face, p, ch, px_size);
            }
            let g = glyph(face, ch, px_size);
            let advance = text_width_one(ch, px_size, face);
            let gx = pen.round() as i64 + g.left;
            let gy = base as i64 + g.top;
            let [bx0, by0, bx1, by1] = self.bounds();
            if g.width == 0 || gx + g.width as i64 <= bx0 || gx >= bx1 || gy + g.height as i64 <= by0 || gy >= by1 {
                pen += advance;
                prev = Some(ch);
                continue;
            }
            for row in 0..g.height {
                for col in 0..g.width {
                    let a = g.coverage[row * g.width + col] as f32 / 255.0;
                    self.blend(gx + col as i64, gy + row as i64, c, a);
                }
            }
            pen += advance;
            prev = Some(ch);
        }
    }
}

/// What painting needs to know besides the tree: the mouse, and what has focus.
#[derive(Clone, Default)]
struct PaintState {
    hover: Option<(f32, f32)>,
    focus: Option<Vec<usize>>,
    /// Whether focus came from the keyboard, so it gets a ring.
    ring: bool,
    caret: Caret,
    /// How far the focused input's text is slid left, to keep the caret in view.
    scroll_x: f32,
    caret_on: bool,
    /// How far each piece under the mouse has faded to its hover look (in a window); without
    /// these, hover looks show straight away (tests).
    fades: Option<HashMap<Vec<usize>, f32>>,
    /// The button being held down.
    pressed: Option<Vec<usize>>,
}

const RING: f32 = 3.0;

/// A look part way to another: colours mixed, a background fading in or out.
fn blend_look(a: &Look, b: &Look, t: f32) -> Look {
    let mut l = if t < 0.5 { a.clone() } else { b.clone() };
    l.color = mix(a.color, b.color, t);
    match (a.background, b.background) {
        (Some(x), Some(y)) => {
            l.background = Some(mix(x, y, t));
            l.bg_alpha = a.bg_alpha + (b.bg_alpha - a.bg_alpha) * t;
        }
        (None, Some(y)) => {
            l.background = Some(y);
            l.bg_alpha = b.bg_alpha * t;
        }
        (Some(x), None) => {
            l.background = Some(x);
            l.bg_alpha = a.bg_alpha * (1.0 - t);
        }
        (None, None) => {}
    }
    if let (Some(x), Some(y)) = (a.border, b.border) {
        l.border = Some(mix(x, y, t));
    }
    l.opacity = a.opacity + (b.opacity - a.opacity) * t;
    l
}

fn ring_around(c: &mut Canvas, n: &Laid, accent: u32) {
    let r = n.look.radius.min(n.w.min(n.h) / 2.0) + RING;
    c.shape(n.x - RING, n.y - RING, n.w + 2.0 * RING, n.h + 2.0 * RING, r, Some(RING), mix(accent, 0xffffff, 0.65), 1.0);
}

fn value_text(n: &Laid) -> &str {
    match &n.value {
        Some(SV::Text(t)) => t,
        _ => "",
    }
}

fn value_num(n: &Laid) -> f32 {
    match &n.value {
        Some(SV::Num(x)) => *x as f32,
        _ => 0.0,
    }
}

/// Where a slider's thumb sits, from 0 to 1.
fn slider_frac(n: &Laid) -> f32 {
    let l = &n.look;
    if l.to == l.from {
        return 0.0;
    }
    ((value_num(n) - l.from) / (l.to - l.from)).clamp(0.0, 1.0)
}

const THUMB: f32 = 8.0;

/// The inside of an input, where its text goes: x, top of the line, width.
fn input_inner(n: &Laid) -> (f32, f32, f32) {
    let l = &n.look;
    let (_, lh) = line_metrics(l.size, l.face());
    let top = n.y + l.pad[0] + ((n.h - l.pad[0] - l.pad[2] - lh) / 2.0).max(0.0);
    (n.x + l.pad[3], top, (n.w - l.pad[1] - l.pad[3]).max(0.0))
}

fn paint_input(c: &mut Canvas, n: &Laid, focused: bool, st: &PaintState) {
    let l = &n.look;
    let (ix, top, iw) = input_inner(n);
    let (_, lh) = line_metrics(l.size, l.face());
    if focused {
        ring_around(c, n, l.accent);
        c.shape(n.x, n.y, n.w, n.h, l.radius, Some(1.5), l.accent, 1.0);
    }
    let text = value_text(n);
    let scroll = if focused { st.scroll_x } else { 0.0 };
    let was = c.clip_to(ix - 1.0, n.y, iw + 2.0, n.h);
    if text.is_empty() {
        c.text(&l.placeholder, ix, top, l.size, l.face(), 0x868e96);
    } else {
        if focused {
            let (a, b) = st.caret.range();
            if a != b && b <= text.len() {
                let xa = text_width(&text[..a], l.size, l.face()) - scroll;
                let xb = text_width(&text[..b], l.size, l.face()) - scroll;
                c.rect(ix + xa, top, xb - xa, lh, 0.0, mix(l.accent, 0xffffff, 0.7), 1.0);
            }
        }
        c.text(text, ix - scroll, top, l.size, l.face(), l.color);
    }
    if focused && st.caret_on && st.caret.caret <= text.len() {
        let x = ix + text_width(&text[..st.caret.caret], l.size, l.face()) - scroll;
        c.rect(x - 0.5, top + 1.0, 1.5, lh - 2.0, 0.0, l.color, 1.0);
    }
    c.clip = was;
}

fn paint_checkbox(c: &mut Canvas, n: &Laid, l: &Look, ring: bool) {
    let on = matches!(n.value, Some(SV::Truth(true)));
    if ring {
        ring_around(c, n, l.accent);
    }
    if on {
        c.rect(n.x, n.y, n.w, n.h, l.radius, l.accent, 1.0);
        let (x, y, w, h) = (n.x, n.y, n.w, n.h);
        c.line(x + w * 0.27, y + h * 0.53, x + w * 0.44, y + h * 0.69, 2.0, 0xffffff);
        c.line(x + w * 0.44, y + h * 0.69, x + w * 0.74, y + h * 0.33, 2.0, 0xffffff);
    } else {
        if let Some(bg) = l.background {
            c.rect(n.x, n.y, n.w, n.h, l.radius, bg, 1.0);
        }
        if let Some(b) = l.border {
            c.shape(n.x, n.y, n.w, n.h, l.radius, Some(l.border_w), b, 1.0);
        }
    }
}

fn paint_slider(c: &mut Canvas, n: &Laid, l: &Look, ring: bool) {
    let f = slider_frac(n);
    let cy = n.y + n.h / 2.0;
    let (x0, x1) = (n.x + THUMB, n.x + n.w - THUMB);
    let tx = x0 + f * (x1 - x0);
    c.rect(x0, cy - 2.0, x1 - x0, 4.0, 2.0, 0xe9ecef, 1.0);
    c.rect(x0, cy - 2.0, tx - x0, 4.0, 2.0, l.accent, 1.0);
    if ring {
        c.shape(tx - THUMB - RING, cy - THUMB - RING, 2.0 * (THUMB + RING), 2.0 * (THUMB + RING), THUMB + RING, Some(RING), mix(l.accent, 0xffffff, 0.65), 1.0);
    }
    c.rect(tx - THUMB, cy - THUMB + 1.0, 2.0 * THUMB, 2.0 * THUMB, THUMB, 0x000000, 0.12);
    c.rect(tx - THUMB, cy - THUMB, 2.0 * THUMB, 2.0 * THUMB, THUMB, 0xffffff, 1.0);
    c.shape(tx - THUMB, cy - THUMB, 2.0 * THUMB, 2.0 * THUMB, THUMB, Some(1.5), l.accent, 1.0);
}

/// A picture in its box: fitted inside it, or with `cover` filling it and cut to fit.
fn paint_image(c: &mut Canvas, n: &Laid, l: &Look) {
    let (bx, by) = (n.x + l.pad[3], n.y + l.pad[0]);
    let (bw, bh) = ((n.w - l.pad[1] - l.pad[3]).max(0.0), (n.h - l.pad[0] - l.pad[2]).max(0.0));
    let Some(p) = picture(&n.text) else {
        // A picture that isn't there shows as a grey box with its name.
        c.rect(bx, by, bw, bh, l.radius, 0xe9ecef, 1.0);
        let name = n.text.rsplit(['/', '\\']).next().unwrap_or("");
        let tw = text_width(name, 11.0, 0);
        if tw < bw - 8.0 {
            c.text(name, bx + (bw - tw) / 2.0, by + bh / 2.0 - 7.0, 11.0, 0, 0x868e96);
        }
        return;
    };
    let (pw, ph) = (p.w as f32, p.h as f32);
    let fit = if l.cover { (bw / pw).max(bh / ph) } else { (bw / pw).min(bh / ph) };
    let (dw, dh) = (pw * fit, ph * fit);
    let (dx, dy) = (bx + (bw - dw) / 2.0, by + (bh - dh) / 2.0);
    let s = c.scale;
    // Only the part inside the box shows, with its corners rounded.
    let (x0, y0, x1, y1) = (dx.max(bx) * s, dy.max(by) * s, (dx + dw).min(bx + bw) * s, (dy + dh).min(by + bh) * s);
    let r = (l.radius * s).min((x1 - x0) / 2.0).min((y1 - y0) / 2.0).max(0.0);
    let per = 1.0 / (fit * s);
    let [bx0, by0, bx1, by1] = c.bounds();
    for py in (y0.floor() as i64).max(by0)..(y1.ceil() as i64).min(by1) {
        for px in (x0.floor() as i64).max(bx0)..(x1.ceil() as i64).min(bx1) {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let cover = (0.5 - outside(fx, fy, x0, y0, x1, y1, r)).clamp(0.0, 1.0);
            if cover <= 0.0 {
                continue;
            }
            // Where this pixel falls in the picture, blended from the four nearest of its pixels.
            let sx = ((fx - dx * s) * per - 0.5).clamp(0.0, pw - 1.0);
            let sy = ((fy - dy * s) * per - 0.5).clamp(0.0, ph - 1.0);
            let (ix, iy) = (sx.floor() as usize, sy.floor() as usize);
            let (ix2, iy2) = ((ix + 1).min(p.w - 1), (iy + 1).min(p.h - 1));
            let (tx, ty) = (sx - ix as f32, sy - iy as f32);
            let at = |x: usize, y: usize| p.px[y * p.w + x];
            let mut rgba = [0.0f32; 4];
            for (q, wgt) in [(at(ix, iy), (1.0 - tx) * (1.0 - ty)), (at(ix2, iy), tx * (1.0 - ty)), (at(ix, iy2), (1.0 - tx) * ty), (at(ix2, iy2), tx * ty)] {
                let a = q[3] as f32 / 255.0 * wgt;
                rgba[0] += q[0] as f32 * a;
                rgba[1] += q[1] as f32 * a;
                rgba[2] += q[2] as f32 * a;
                rgba[3] += a;
            }
            if rgba[3] <= 0.0 {
                continue;
            }
            let colour = ((rgba[0] / rgba[3]) as u32) << 16 | ((rgba[1] / rgba[3]) as u32) << 8 | (rgba[2] / rgba[3]) as u32;
            c.blend(px, py, colour, rgba[3] * cover);
        }
    }
}

/// The scroll bar's knob: top and height, when there's more to see than fits.
fn scroll_knob(n: &Laid) -> Option<(f32, f32)> {
    if n.content_h <= n.h + 0.5 {
        return None;
    }
    let kh = (n.h * n.h / n.content_h).max(24.0).min(n.h);
    let max = n.content_h - n.h;
    Some((n.y + (n.scrolled / max) * (n.h - kh), kh))
}

fn paint(c: &mut Canvas, n: &Laid, st: &PaintState, path: &mut Vec<usize>) {
    let hover = st.hover;
    let over = hover.map(|(hx, hy)| hx >= n.x && hy >= n.y && hx < n.x + n.w && hy < n.y + n.h).unwrap_or(false);
    let t = match &st.fades {
        Some(f) => f.get(path.as_slice()).copied().unwrap_or(0.0),
        None => {
            if over {
                1.0
            } else {
                0.0
            }
        }
    };
    let blended;
    let pressed;
    let mut l: &Look = match &n.hover {
        Some(h) if t >= 1.0 => h,
        Some(h) if t > 0.0 => {
            blended = blend_look(&n.look, h, t);
            &blended
        }
        _ => &n.look,
    };
    // A button held down under the mouse looks pressed in.
    if over && st.pressed.as_deref() == Some(path.as_slice()) && n.kind == "button" {
        let mut p = l.clone();
        p.background = Some(mix(l.background.unwrap_or(0xe9ecef), 0x000000, 0.12));
        p.bg_alpha = 1.0;
        pressed = p;
        l = &pressed;
    }
    let focused = st.focus.as_deref() == Some(path.as_slice());
    let was = c.alpha;
    c.alpha *= l.opacity;
    if l.shadow {
        for i in 1..=6 {
            let e = i as f32;
            c.rect(n.x - e + 0.0, n.y - e + 3.0, n.w + 2.0 * e, n.h + 2.0 * e, l.radius + e, 0x000000, 0.012);
        }
    }
    match n.kind.as_str() {
        "checkbox" => paint_checkbox(c, n, l, focused && st.ring),
        "slider" => paint_slider(c, n, l, focused && st.ring),
        _ => {
            if focused && st.ring && n.kind == "button" {
                ring_around(c, n, 0x1c7ed6);
            }
            if let Some(bg) = l.background {
                c.rect(n.x, n.y, n.w, n.h, l.radius, bg, l.bg_alpha);
            }
            if let Some(b) = l.border {
                c.shape(n.x, n.y, n.w, n.h, l.radius, Some(l.border_w), b, 1.0);
            }
        }
    }
    if n.kind == "input" {
        paint_input(c, n, focused, st);
    }
    if n.kind == "image" {
        paint_image(c, n, l);
    }
    let unclipped = if n.kind == "scroll" { Some(c.clip_to(n.x, n.y, n.w, n.h)) } else { None };
    if shows_text(&n.kind) {
        let (_, lh) = line_metrics(l.size, l.face());
        let inner_w = n.w - l.pad[1] - l.pad[3];
        let block = lh * n.lines.len() as f32;
        let top = n.y + l.pad[0] + ((n.h - l.pad[0] - l.pad[2] - block) / 2.0).max(0.0);
        for (i, line) in n.lines.iter().enumerate() {
            let tw = text_width(line, l.size, l.face());
            let tx = n.x + l.pad[3] + offset(l.halign, inner_w - tw);
            let ty = top + lh * i as f32;
            c.text(line, tx, ty, l.size, l.face(), l.color);
            if l.strike {
                c.rect(tx, ty + lh * 0.55, tw, (l.size / 14.0).max(1.0), 0.0, l.color, 1.0);
            }
        }
    }
    for (i, ch) in n.children.iter().enumerate() {
        path.push(i);
        paint(c, ch, st, path);
        path.pop();
    }
    if let Some(was) = unclipped {
        c.clip = was;
        if let Some((ky, kh)) = scroll_knob(n) {
            c.rect(n.x + n.w - 8.0, ky + 2.0, 5.0, kh - 4.0, 2.5, 0x000000, 0.28);
        }
    }
    c.alpha = was;
}

fn render(tree: &Laid, w: usize, h: usize, scale: f32, st: &PaintState, background: u32) -> Canvas {
    let mut c = Canvas::new(w, h, scale);
    c.px.iter_mut().for_each(|p| *p = background);
    paint(&mut c, tree, st, &mut vec![]);
    c
}

/// The way to the deepest node with an action at this point: which child, at each level.
fn hit(n: &Laid, x: f32, y: f32) -> Option<Vec<usize>> {
    if x < n.x || y < n.y || x >= n.x + n.w || y >= n.y + n.h {
        return None;
    }
    for (i, c) in n.children.iter().enumerate().rev() {
        if let Some(mut p) = hit(c, x, y) {
            p.insert(0, i);
            return Some(p);
        }
    }
    n.action.map(|_| vec![])
}

fn node_at<'a>(n: &'a Laid, path: &[usize]) -> Option<&'a Laid> {
    match path.split_first() {
        None => Some(n),
        Some((&i, rest)) => n.children.get(i).and_then(|c| node_at(c, rest)),
    }
}

/// Everything that can have focus, in the order Tab goes through them.
fn focusable(n: &Laid, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if n.action.is_some() && matches!(n.kind.as_str(), "button" | "input" | "checkbox" | "slider") {
        out.push(path.clone());
    }
    for (i, c) in n.children.iter().enumerate() {
        path.push(i);
        focusable(c, path, out);
        path.pop();
    }
}

// ----- the window -----

pub struct Settings {
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub min_width: f64,
    pub min_height: f64,
    pub background: u32,
    /// The font every piece uses unless it has its own.
    pub font: Option<usize>,
}

pub fn settings(style: &[(String, SV)]) -> Settings {
    let mut s = Settings { title: "Lust".into(), width: 640.0, height: 480.0, min_width: 0.0, min_height: 0.0, background: 0xffffff, font: None };
    for (k, v) in style {
        match (k.as_str(), v) {
            ("title", SV::Text(t)) => s.title = t.clone(),
            ("width", SV::Num(n)) => s.width = *n,
            ("height", SV::Num(n)) => s.height = *n,
            ("minWidth", SV::Num(n)) => s.min_width = *n,
            ("minHeight", SV::Num(n)) => s.min_height = *n,
            ("background", SV::Text(t)) => s.background = colour(t).unwrap_or(0xffffff),
            ("font", SV::Text(t)) => s.font = font_file(t),
            _ => {}
        }
    }
    s
}

pub type Build<'a> = dyn FnMut() -> Result<(Node, Vec<Action>), String> + 'a;

// ----- pieces: kept between redraws -----

type PieceKey = (usize, usize, String);

/// The pieces of one open window: by the piece they're in, where they're written, and their turn
/// there (the third TodoRow in a loop is the same third TodoRow next time).
#[derive(Default)]
struct Pieces {
    items: HashMap<PieceKey, std::rc::Rc<dyn std::any::Any>>,
    seen: std::collections::HashSet<PieceKey>,
    turns: HashMap<(usize, usize), u32>,
}

thread_local! {
    static PIECES: std::cell::RefCell<Vec<Pieces>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The piece written at `site` inside `parent`, made the first time it's drawn.
/// In a loop over a map or cMap, `key` is the row's key, so a piece stays with its row.
pub fn piece<T: 'static>(parent: usize, site: usize, key: Option<String>, make: impl FnOnce() -> std::rc::Rc<T>) -> std::rc::Rc<T> {
    let (key, found) = PIECES.with(|p| {
        let mut p = p.borrow_mut();
        let Some(st) = p.last_mut() else { return ((parent, site, String::new()), None) };
        let turn = st.turns.entry((parent, site)).or_insert(0);
        *turn += 1;
        let key = (parent, site, key.unwrap_or_else(|| format!("#{turn}")));
        st.seen.insert(key.clone());
        let found = st.items.get(&key).cloned();
        (key, found)
    });
    if let Some(found) = found.and_then(|f| f.downcast::<T>().ok()) {
        return found;
    }
    let made = make();
    PIECES.with(|p| {
        if let Some(st) = p.borrow_mut().last_mut() {
            st.items.insert(key, made.clone());
        }
    });
    made
}

/// Opens a window showing the view, until it's closed. Each click runs its action, and then the
/// view is drawn again from the data as it is now.
pub fn show(style: &[(String, SV)], build: &mut Build) -> Result<(), String> {
    let s = settings(style);
    PIECES.with(|p| p.borrow_mut().push(Pieces::default()));
    // Each drawing starts the pieces' turns again, and forgets pieces that weren't drawn.
    let mut framed = || {
        PIECES.with(|p| {
            if let Some(st) = p.borrow_mut().last_mut() {
                st.seen.clear();
                st.turns.clear();
            }
        });
        let r = build();
        let gone: Vec<std::rc::Rc<dyn std::any::Any>> = PIECES.with(|p| {
            let mut p = p.borrow_mut();
            let Some(st) = p.last_mut() else { return vec![] };
            let keys: Vec<PieceKey> = st.items.keys().filter(|k| !st.seen.contains(*k)).cloned().collect();
            keys.iter().filter_map(|k| st.items.remove(k)).collect()
        });
        drop(gone);
        r
    };
    let r = if std::env::var("LUST_VIEW_TEST").is_ok() { test(s, &mut framed) } else { windowed::run(s, &mut framed) };
    let gone = PIECES.with(|p| p.borrow_mut().pop());
    drop(gone);
    r
}

// ----- what happens: clicks, keys and typing, the same in a window and in tests -----

/// The keys held down. `command` is Ctrl (Cmd on a Mac); `word` moves by words: Ctrl (Option on a Mac).
#[derive(Clone, Copy, PartialEq, Debug, Default)]
struct Mods {
    shift: bool,
    command: bool,
    word: bool,
    alt: bool,
}

/// A key that does something, besides typing.
#[derive(Clone, Debug, PartialEq)]
enum Nav {
    Edit(Key),
    Tab,
    Enter,
    Space,
    Escape,
    Up,
    Down,
    Copy,
    Cut,
    Paste,
    Undo,
    Redo,
}

/// One input's earlier texts, for undo and redo.
#[derive(Default)]
struct History {
    undo: Vec<(String, Caret)>,
    redo: Vec<(String, Caret)>,
    /// What the last change was (typing or deleting) and when, so a run of typing undoes in one go.
    last: Option<(bool, std::time::Instant)>,
}

#[derive(Clone, PartialEq, Debug)]
enum Drag {
    None,
    Select,
    Slide,
    /// A scroll bar's knob, grabbed at a height with the scroll where it was.
    Scroll(Vec<usize>, f32, f32),
}

/// An `every` in the view: how often, which action, and when it's next due (in seconds).
#[derive(Clone, Debug)]
struct Timer {
    secs: f64,
    action: usize,
    due: f64,
}

fn timers(n: &Node, out: &mut Vec<(f64, usize)>) {
    if n.kind == "every" {
        if let (Some(a), Some(SV::Num(s))) = (n.action, &n.value) {
            out.push((s.max(0.016), a));
        }
    }
    for c in &n.children {
        timers(c, out);
    }
}

/// Every scroll at this point, outermost first.
fn scrolls_at(n: &Laid, x: f32, y: f32, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if x < n.x || y < n.y || x >= n.x + n.w || y >= n.y + n.h {
        return;
    }
    if n.kind == "scroll" {
        out.push(path.clone());
    }
    for (i, c) in n.children.iter().enumerate() {
        path.push(i);
        scrolls_at(c, x, y, path, out);
        path.pop();
    }
}

fn shift_down(n: &mut Laid, dy: f32) {
    for c in n.children.iter_mut() {
        c.y += dy;
        shift_down(c, dy);
    }
}

/// Moves each scroll's content up by how far it's scrolled, keeping that within what there is.
fn apply_scrolls(n: &mut Laid, path: &mut Vec<usize>, offsets: &mut HashMap<Vec<usize>, f32>) {
    if n.kind == "scroll" {
        let max = (n.content_h - n.h).max(0.0);
        let off = offsets.get(path.as_slice()).copied().unwrap_or(0.0).clamp(0.0, max).round();
        if off > 0.0 {
            offsets.insert(path.clone(), off);
        } else {
            offsets.remove(path.as_slice());
        }
        n.scrolled = off;
        shift_down(n, -off);
    }
    for (i, c) in n.children.iter_mut().enumerate() {
        path.push(i);
        apply_scrolls(c, path, offsets);
        path.pop();
    }
}

/// A slider's value, kept in its range and on its steps.
fn snap(l: &Look, v: f32) -> f32 {
    let (lo, hi) = (l.from.min(l.to), l.from.max(l.to));
    let mut v = v.clamp(lo, hi);
    if l.step > 0.0 {
        v = (l.from + ((v - l.from) / l.step).round() * l.step).clamp(lo, hi);
    }
    (v * 1e6).round() / 1e6
}

/// A window's view while it's open: the tree, what has focus, and what the mouse is doing.
struct Ui<'a, 'b> {
    s: Settings,
    build: &'a mut Build<'b>,
    w: f32,
    h: f32,
    /// The view as last drawn, laid out again when only scrolling changes.
    node: Node,
    tree: Laid,
    actions: Vec<Action>,
    /// How far each scroll is scrolled down, by its place in the tree.
    scrolls: HashMap<Vec<usize>, f32>,
    timers: Vec<Timer>,
    started: std::time::Instant,
    /// Tests keep their own clock, moved on by `wait`.
    test_clock: Option<f64>,
    st: PaintState,
    pressed: Option<Vec<usize>>,
    drag: Drag,
    /// When and where the last click was, and how many came quickly together (a double click).
    clicks: (std::time::Instant, u32, (f32, f32)),
    /// Copied text, for when there's no system clipboard (and in tests).
    clip: String,
    system_clipboard: bool,
    /// When the caret last started blinking; None keeps it showing (tests).
    blink: Option<std::time::Instant>,
    /// The title changed with the data, and the window should show the new one.
    title_changed: bool,
    /// Undo and redo for each input, by its place in the tree.
    history: HashMap<Vec<usize>, History>,
    /// Hover looks fading in and out (in a window), and when they last moved on.
    fades: HashMap<Vec<usize>, f32>,
    faded_at: std::time::Instant,
}

const FADE_SECS: f32 = 0.12;

/// Every piece with a hover look under this point.
fn hover_targets(n: &Laid, x: f32, y: f32, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if x < n.x || y < n.y || x >= n.x + n.w || y >= n.y + n.h {
        return;
    }
    if n.hover.is_some() {
        out.push(path.clone());
    }
    for (i, c) in n.children.iter().enumerate() {
        path.push(i);
        hover_targets(c, x, y, path, out);
        path.pop();
    }
}

const BLINK_MS: u128 = 530;

impl<'a, 'b> Ui<'a, 'b> {
    fn new(s: Settings, build: &'a mut Build<'b>, windowed: bool) -> Result<Ui<'a, 'b>, String> {
        let (w, h) = (s.width as f32, s.height as f32);
        let (node, actions) = build()?;
        let tree = lay_out(&node, w, h);
        let now = std::time::Instant::now();
        let mut ui = Ui {
            s,
            build,
            w,
            h,
            node,
            tree,
            actions,
            scrolls: HashMap::new(),
            timers: vec![],
            started: now,
            test_clock: (!windowed).then_some(0.0),
            st: PaintState { caret_on: true, ..Default::default() },
            pressed: None,
            drag: Drag::None,
            clicks: (now, 0, (0.0, 0.0)),
            clip: String::new(),
            system_clipboard: windowed,
            blink: windowed.then_some(now),
            title_changed: false,
            history: HashMap::new(),
            fades: HashMap::new(),
            faded_at: now,
        };
        ui.set_timers();
        Ok(ui)
    }

    /// Seconds since the window opened (or on the test's clock).
    fn now(&self) -> f64 {
        self.test_clock.unwrap_or_else(|| self.started.elapsed().as_secs_f64())
    }

    /// The view's timers, keeping when each is due if it was there last time too.
    fn set_timers(&mut self) {
        let mut found = Vec::new();
        timers(&self.node, &mut found);
        let now = self.now();
        let old = std::mem::take(&mut self.timers);
        self.timers = found
            .into_iter()
            .enumerate()
            .map(|(i, (secs, action))| {
                let due = match old.get(i) {
                    Some(t) if t.secs == secs => t.due,
                    _ => now + secs,
                };
                Timer { secs, action, due }
            })
            .collect();
    }

    /// Runs the timers that are due. Each runs once, however late it is.
    fn fire_timers(&mut self) -> Result<bool, String> {
        let now = self.now();
        let due: Vec<usize> = (0..self.timers.len()).filter(|&i| self.timers[i].due <= now + 1e-9).collect();
        if due.is_empty() {
            return Ok(false);
        }
        for &i in &due {
            let t = &mut self.timers[i];
            t.due = if t.due + t.secs > now { t.due + t.secs } else { now + t.secs };
        }
        for &i in &due {
            let a = self.timers[i].action;
            run_with(&mut self.actions[a], SV::Flag)?;
        }
        self.rebuild()?;
        Ok(true)
    }

    fn next_timer(&self) -> Option<f64> {
        self.timers.iter().map(|t| t.due).fold(None, |m: Option<f64>, d| Some(m.map_or(d, |m| m.min(d))))
    }

    /// Moves the test's clock on, running timers as they come due.
    fn wait(&mut self, secs: f64) -> Result<(), String> {
        let end = self.now() + secs;
        while let Some(d) = self.next_timer().filter(|&d| d <= end + 1e-9) {
            self.test_clock = Some(d.max(self.now()));
            self.fire_timers()?;
        }
        self.test_clock = Some(end);
        Ok(())
    }

    /// Lays the last view out again, with each scroll where it was.
    fn relayout(&mut self) {
        self.tree = lay_out(&self.node, self.w, self.h);
        apply_scrolls(&mut self.tree, &mut vec![], &mut self.scrolls);
    }

    /// Scrolls the scroll under the mouse, or the one around it if that one can't go further.
    fn wheel(&mut self, x: f32, y: f32, dy: f32) -> bool {
        let mut found = Vec::new();
        scrolls_at(&self.tree, x, y, &mut vec![], &mut found);
        for p in found.iter().rev() {
            let Some(n) = node_at(&self.tree, p) else { continue };
            let max = (n.content_h - n.h).max(0.0);
            let next = (n.scrolled + dy).clamp(0.0, max);
            if (next - n.scrolled).abs() >= 0.5 {
                self.scrolls.insert(p.clone(), next);
                self.relayout();
                return true;
            }
        }
        false
    }

    /// Scrolls so what has focus can be seen.
    fn reveal(&mut self) {
        let Some(path) = self.st.focus.clone() else { return };
        for depth in 0..path.len() {
            let (Some(s), Some(t)) = (node_at(&self.tree, &path[..depth]), node_at(&self.tree, &path)) else { return };
            if s.kind != "scroll" {
                continue;
            }
            let mut off = s.scrolled;
            if t.y < s.y {
                off -= s.y - t.y + 4.0;
            } else if t.y + t.h > s.y + s.h {
                off += t.y + t.h - (s.y + s.h) + 4.0;
            } else {
                continue;
            }
            self.scrolls.insert(path[..depth].to_vec(), off);
            self.relayout();
        }
    }

    /// Draws the view again from the data as it is now. Focus stays where it was, if it can.
    fn rebuild(&mut self) -> Result<(), String> {
        let (node, actions) = (self.build)()?;
        let now = settings(&node.style);
        if now.title != self.s.title {
            self.s.title = now.title;
            self.title_changed = true;
        }
        self.s.background = now.background;
        self.node = node;
        self.relayout();
        self.actions = actions;
        self.set_timers();
        if let Some(f) = self.st.focus.clone() {
            match node_at(&self.tree, &f) {
                Some(n) if n.action.is_some() => {
                    let t = value_text(n).to_string();
                    self.st.caret.fit(&t);
                }
                _ => self.st.focus = None,
            }
        }
        self.keep_caret_in_view();
        Ok(())
    }

    fn resize(&mut self, w: f32, h: f32) -> Result<(), String> {
        self.w = w;
        self.h = h;
        self.rebuild()
    }

    /// Runs what's at `path` with its new value, then draws again.
    fn run(&mut self, path: &[usize], value: SV) -> Result<(), String> {
        let Some(a) = node_at(&self.tree, path).and_then(|n| n.action) else { return Ok(()) };
        run_with(&mut self.actions[a], value)?;
        self.rebuild()
    }

    fn focused(&self) -> Option<&Laid> {
        self.st.focus.as_ref().and_then(|f| node_at(&self.tree, f))
    }

    fn focused_kind(&self) -> String {
        self.focused().map(|n| n.kind.clone()).unwrap_or_default()
    }

    fn set_focus(&mut self, path: Option<Vec<usize>>, ring: bool) {
        if path != self.st.focus {
            self.st.scroll_x = 0.0;
            let end = path.as_ref().and_then(|p| node_at(&self.tree, p)).map(|n| value_text(n).len()).unwrap_or(0);
            self.st.caret = Caret::at(end);
        }
        self.st.focus = path;
        self.st.ring = ring;
        self.wake();
    }

    /// Shows the caret again, straight away.
    fn wake(&mut self) {
        if self.blink.is_some() {
            self.blink = Some(std::time::Instant::now());
        }
    }

    fn paint_state(&self) -> PaintState {
        let mut st = self.st.clone();
        st.caret_on = match self.blink {
            Some(t) => (t.elapsed().as_millis() / BLINK_MS) % 2 == 0,
            None => true,
        };
        if self.blink.is_some() {
            st.fades = Some(self.fades.clone());
        }
        st.pressed = self.pressed.clone();
        st
    }

    /// What's under the mouse that has a hover look.
    fn hovered(&self) -> Vec<Vec<usize>> {
        let mut out = Vec::new();
        if let Some((x, y)) = self.st.hover {
            hover_targets(&self.tree, x, y, &mut vec![], &mut out);
        }
        out
    }

    /// Moves the hover fades on. Returns whether any is still moving.
    fn step_fades(&mut self) -> bool {
        let now = std::time::Instant::now();
        let step = (now.duration_since(self.faded_at).as_secs_f32() / FADE_SECS).min(1.0);
        self.faded_at = now;
        let on = self.hovered();
        for p in &on {
            let t = self.fades.entry(p.clone()).or_insert(0.0);
            *t = (*t + step).min(1.0);
        }
        self.fades.retain(|p, t| {
            if !on.contains(p) {
                *t -= step;
            }
            *t > 0.0
        });
        self.fades.iter().any(|(p, t)| if on.contains(p) { *t < 1.0 } else { *t > 0.0 })
    }

    /// When the caret next blinks, if an input has focus.
    fn next_blink(&self) -> Option<std::time::Instant> {
        let t = self.blink?;
        if self.focused_kind() != "input" {
            return None;
        }
        let ms = t.elapsed().as_millis();
        Some(t + std::time::Duration::from_millis(((ms / BLINK_MS + 1) * BLINK_MS) as u64))
    }

    /// Slides the focused input's text so the caret stays in view.
    fn keep_caret_in_view(&mut self) {
        let Some(n) = self.focused() else { return };
        if n.kind != "input" {
            return;
        }
        let (_, _, iw) = input_inner(n);
        let t = value_text(n);
        let l = &n.look;
        let cx = text_width(&t[..self.st.caret.caret.min(t.len())], l.size, l.face());
        let tw = text_width(t, l.size, l.face());
        let mut s = self.st.scroll_x;
        if cx - s > iw - 2.0 {
            s = cx - iw + 2.0;
        }
        if cx < s {
            s = cx;
        }
        self.st.scroll_x = s.min((tw - iw + 2.0).max(0.0)).max(0.0);
    }

    fn place_in_input(&self, n: &Laid, x: f32) -> usize {
        let (ix, _, _) = input_inner(n);
        place_at(value_text(n), x - ix + self.st.scroll_x, n.look.size, n.look.face())
    }

    fn slide_to(&mut self, path: &[usize], x: f32) -> Result<(), String> {
        let Some(n) = node_at(&self.tree, path) else { return Ok(()) };
        let l = &n.look;
        let (x0, x1) = (n.x + THUMB, n.x + n.w - THUMB);
        let f = ((x - x0) / (x1 - x0).max(1.0)).clamp(0.0, 1.0);
        let v = snap(l, l.from + f * (l.to - l.from));
        if (v - value_num(n)).abs() < 1e-6 {
            return Ok(());
        }
        self.run(path, SV::Num(v as f64))
    }

    fn clip_get(&self) -> Option<String> {
        if self.system_clipboard {
            if let Some(t) = clipboard_get() {
                return Some(t);
            }
        }
        Some(self.clip.clone())
    }

    fn clip_set(&mut self, t: &str) {
        self.clip = t.to_string();
        if self.system_clipboard {
            clipboard_set(t);
        }
    }

    fn press(&mut self, x: f32, y: f32, mods: Mods) -> Result<(), String> {
        self.st.hover = Some((x, y));
        let now = std::time::Instant::now();
        let (t, n, (px, py)) = self.clicks;
        let quick = now.duration_since(t) < std::time::Duration::from_millis(400) && (x - px).abs() < 4.0 && (y - py).abs() < 4.0;
        let count = if quick { n + 1 } else { 1 };
        self.clicks = (now, count, (x, y));
        // The knob of a scroll bar, grabbed to drag.
        let mut found = Vec::new();
        scrolls_at(&self.tree, x, y, &mut vec![], &mut found);
        for p in found.iter().rev() {
            let Some(n) = node_at(&self.tree, p) else { continue };
            if let Some((ky, kh)) = scroll_knob(n) {
                if x >= n.x + n.w - 12.0 && y >= ky && y < ky + kh {
                    self.drag = Drag::Scroll(p.clone(), y, n.scrolled);
                    return Ok(());
                }
            }
        }
        let at = hit(&self.tree, x, y);
        self.pressed = at.clone();
        let Some(path) = at else {
            self.set_focus(None, false);
            return Ok(());
        };
        let kind = node_at(&self.tree, &path).map(|n| n.kind.clone()).unwrap_or_default();
        match kind.as_str() {
            "input" => {
                if self.st.focus.as_ref() != Some(&path) {
                    self.set_focus(Some(path.clone()), false);
                }
                self.st.ring = false;
                let Some(n) = node_at(&self.tree, &path) else { return Ok(()) };
                let i = self.place_in_input(n, x);
                let t = value_text(n).to_string();
                match count {
                    1 if mods.shift => self.st.caret.caret = i,
                    1 => self.st.caret = Caret::at(i),
                    2 => {
                        let (a, b) = word_at(&t, i);
                        self.st.caret = Caret { anchor: a, caret: b };
                    }
                    _ => self.st.caret = Caret { anchor: 0, caret: t.len() },
                }
                self.drag = if count == 1 { Drag::Select } else { Drag::None };
                self.wake();
                self.keep_caret_in_view();
            }
            "slider" => {
                self.set_focus(Some(path.clone()), false);
                self.drag = Drag::Slide;
                self.slide_to(&path, x)?;
            }
            // Buttons and checkboxes act when the mouse is let go over them. Clicking one takes
            // focus away from an input, without giving the button a ring.
            _ => self.set_focus(None, false),
        }
        Ok(())
    }

    fn moved(&mut self, x: f32, y: f32) -> Result<(), String> {
        self.st.hover = Some((x, y));
        match self.drag.clone() {
            Drag::Scroll(p, from_y, from) => {
                if let Some(n) = node_at(&self.tree, &p) {
                    if let Some((_, kh)) = scroll_knob(n) {
                        let per = (n.content_h - n.h) / (n.h - kh).max(1.0);
                        self.scrolls.insert(p.clone(), from + (y - from_y) * per);
                        self.relayout();
                    }
                }
            }
            Drag::Select => {
                if let Some(n) = self.focused() {
                    let i = self.place_in_input(n, x);
                    self.st.caret.caret = i;
                    self.keep_caret_in_view();
                }
            }
            Drag::Slide => {
                if let Some(p) = self.st.focus.clone() {
                    self.slide_to(&p, x)?;
                }
            }
            Drag::None => {}
        }
        Ok(())
    }

    fn release(&mut self, x: f32, y: f32) -> Result<(), String> {
        if let Drag::Scroll(..) = std::mem::replace(&mut self.drag, Drag::None) {
            return Ok(());
        }
        let at = hit(&self.tree, x, y);
        let pressed = self.pressed.take();
        let Some(path) = at.filter(|a| Some(a) == pressed.as_ref()) else { return Ok(()) };
        let Some(n) = node_at(&self.tree, &path) else { return Ok(()) };
        match n.kind.as_str() {
            "checkbox" => {
                let on = matches!(n.value, Some(SV::Truth(true)));
                self.run(&path, SV::Truth(!on))
            }
            "button" => self.run(&path, SV::Flag),
            _ => Ok(()),
        }
    }

    fn typed(&mut self, s: &str) -> Result<(), String> {
        self.edit(Key::Type(s.to_string()), Mods::default())
    }

    /// A key in the focused input: the text changes, then its action runs with the new text.
    fn edit(&mut self, k: Key, mods: Mods) -> Result<(), String> {
        let Some(path) = self.st.focus.clone() else { return Ok(()) };
        let Some(n) = node_at(&self.tree, &path) else { return Ok(()) };
        if n.kind != "input" {
            return Ok(());
        }
        let t = value_text(n).to_string();
        let before = self.st.caret.clone();
        let changed = self.st.caret.key(&t, &k, mods.shift, mods.word);
        self.wake();
        if let Some(new) = changed {
            // Remember the text before, unless this carries on a run of typing (or of deleting).
            let typing = matches!(k, Key::Type(_));
            let h = self.history.entry(path.clone()).or_default();
            let now = std::time::Instant::now();
            let carries_on = matches!(h.last, Some((was, at)) if was == typing && now.duration_since(at).as_millis() < 1000)
                && !matches!(&k, Key::Type(s) if s.contains(' '));
            if !carries_on || h.undo.is_empty() {
                h.undo.push((t.clone(), before));
                if h.undo.len() > 200 {
                    h.undo.remove(0);
                }
            }
            h.redo.clear();
            h.last = Some((typing, now));
            let caret = self.st.caret.clone();
            self.run(&path, SV::Text(new))?;
            self.st.caret = caret;
            if let Some(n) = self.focused() {
                let t = value_text(n).to_string();
                self.st.caret.fit(&t);
            }
        }
        self.keep_caret_in_view();
        Ok(())
    }

    fn nav(&mut self, k: Nav, mods: Mods) -> Result<(), String> {
        match k {
            Nav::Tab => {
                let mut all = Vec::new();
                focusable(&self.tree, &mut vec![], &mut all);
                if all.is_empty() {
                    return Ok(());
                }
                let len = all.len();
                let at = self.st.focus.as_ref().and_then(|f| all.iter().position(|p| p == f));
                let next = match (at, mods.shift) {
                    (None, false) => 0,
                    (None, true) => len - 1,
                    (Some(i), false) => (i + 1) % len,
                    (Some(i), true) => (i + len - 1) % len,
                };
                self.set_focus(Some(all[next].clone()), true);
                self.reveal();
                // Tabbing into an input selects its text, ready to be typed over.
                if let Some(n) = self.focused() {
                    if n.kind == "input" {
                        let end = value_text(n).len();
                        self.st.caret = Caret { anchor: 0, caret: end };
                    }
                }
                self.keep_caret_in_view();
                Ok(())
            }
            Nav::Escape => {
                self.set_focus(None, false);
                Ok(())
            }
            _ => {
                let Some(path) = self.st.focus.clone() else { return Ok(()) };
                let Some(n) = node_at(&self.tree, &path) else { return Ok(()) };
                match (n.kind.as_str(), &k) {
                    ("input", Nav::Edit(e)) => self.edit(e.clone(), mods),
                    ("input", Nav::Copy | Nav::Cut) => {
                        let sel = self.st.caret.selected(value_text(n)).to_string();
                        if sel.is_empty() {
                            return Ok(());
                        }
                        self.clip_set(&sel);
                        if k == Nav::Cut {
                            return self.edit(Key::Back, Mods::default());
                        }
                        Ok(())
                    }
                    ("input", Nav::Undo | Nav::Redo) => {
                        let current = (value_text(n).to_string(), self.st.caret.clone());
                        let h = self.history.entry(path.clone()).or_default();
                        let back = if k == Nav::Undo { h.undo.pop() } else { h.redo.pop() };
                        let Some((text, caret)) = back else { return Ok(()) };
                        if k == Nav::Undo {
                            h.redo.push(current);
                        } else {
                            h.undo.push(current);
                        }
                        h.last = None;
                        self.run(&path, SV::Text(text))?;
                        self.st.caret = caret;
                        if let Some(n) = self.focused() {
                            let t = value_text(n).to_string();
                            self.st.caret.fit(&t);
                        }
                        self.keep_caret_in_view();
                        Ok(())
                    }
                    ("input", Nav::Enter) => match n.enter {
                        Some(a) => {
                            run_with(&mut self.actions[a], SV::Flag)?;
                            self.rebuild()
                        }
                        None => Ok(()),
                    },
                    ("input", Nav::Paste) => match self.clip_get() {
                        // An input is one line, so pasted lines are joined with spaces.
                        Some(t) => self.edit(Key::Type(t.replace("\r\n", " ").replace(['\n', '\r'], " ")), Mods::default()),
                        None => Ok(()),
                    },
                    ("button", Nav::Enter | Nav::Space) => self.run(&path, SV::Flag),
                    ("checkbox", Nav::Enter | Nav::Space) => {
                        let on = matches!(n.value, Some(SV::Truth(true)));
                        self.run(&path, SV::Truth(!on))
                    }
                    ("slider", Nav::Edit(Key::Left | Key::Right | Key::Home | Key::End) | Nav::Up | Nav::Down) => {
                        let l = &n.look;
                        let step = if l.step > 0.0 { l.step } else { (l.to - l.from).abs() / 100.0 };
                        let sign = if l.to >= l.from { 1.0 } else { -1.0 };
                        let v = value_num(n);
                        let nv = match &k {
                            Nav::Edit(Key::Left) | Nav::Down => v - step * sign,
                            Nav::Edit(Key::Right) | Nav::Up => v + step * sign,
                            Nav::Edit(Key::Home) => l.from,
                            _ => l.to,
                        };
                        let nv = snap(l, nv);
                        if (nv - v).abs() < 1e-6 {
                            return Ok(());
                        }
                        self.run(&path, SV::Num(nv as f64))
                    }
                    _ => Ok(()),
                }
            }
        }
    }

    /// Whether what has focus uses this key itself, so a shortcut doesn't take it.
    fn focus_uses(&self, key: &str, m: Mods) -> bool {
        let plain = !m.command && !m.alt;
        if key == "tab" && !m.command {
            return true;
        }
        match self.focused_kind().as_str() {
            "input" | "textarea" => {
                if plain {
                    !function_key(key)
                } else if m.command {
                    matches!(key, "a" | "c" | "x" | "v" | "z" | "y" | "left" | "right" | "up" | "down" | "backspace" | "delete" | "home" | "end")
                } else {
                    matches!(key, "left" | "right" | "backspace" | "delete")
                }
            }
            "button" | "checkbox" => plain && matches!(key, "enter" | "space"),
            "slider" => plain && matches!(key, "left" | "right" | "up" | "down" | "home" | "end"),
            _ => false,
        }
    }

    /// Runs the view's shortcut for this key, if it has one. Returns whether one ran.
    fn shortcut(&mut self, key: &str, m: Mods) -> Result<bool, String> {
        if self.focus_uses(key, m) {
            return Ok(false);
        }
        let mut all = Vec::new();
        shortcuts(&self.node, &mut all);
        let found = all.into_iter().find(|(s, _)| s.key == key && s.command == m.command && s.shift == m.shift && s.alt == m.alt);
        let Some((_, a)) = found else { return Ok(false) };
        run_with(&mut self.actions[a], SV::Flag)?;
        self.rebuild()?;
        Ok(true)
    }

    /// The mouse pointer for this spot: a hand over things that act, a text bar over inputs.
    fn pointer(&self, x: f32, y: f32) -> u8 {
        match hit(&self.tree, x, y).and_then(|p| node_at(&self.tree, &p)).map(|n| n.kind.as_str()) {
            Some("input") => 2,
            Some(_) => 1,
            None => 0,
        }
    }
}

mod windowed {
    use super::*;
    use std::num::NonZeroU32;
    use std::rc::Rc;
    use winit::application::ApplicationHandler;
    use winit::event::{ElementState, MouseButton, MouseScrollDelta, StartCause, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::keyboard::{Key as WKey, ModifiersState, NamedKey};
    use winit::window::{CursorIcon, Window, WindowId};

    struct App<'a, 'b> {
        ui: Ui<'a, 'b>,
        window: Option<Rc<Window>>,
        surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
        mouse: Option<(f32, f32)>,
        mods: ModifiersState,
        error: Option<String>,
    }

    impl<'a, 'b> App<'a, 'b> {
        fn logical(&self) -> (f32, f32, f32) {
            let w = self.window.as_ref().unwrap();
            let scale = w.scale_factor() as f32;
            let size = w.inner_size();
            (size.width as f32 / scale, size.height as f32 / scale, scale)
        }

        fn mods(&self) -> Mods {
            let m = self.mods;
            if cfg!(target_os = "macos") {
                Mods { shift: m.shift_key(), command: m.super_key(), word: m.alt_key(), alt: m.alt_key() }
            } else {
                Mods { shift: m.shift_key(), command: m.control_key(), word: m.control_key(), alt: m.alt_key() }
            }
        }

        fn redraw(&self) {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }

        /// Keeps a failure to stop the window with.
        fn check(&mut self, r: Result<(), String>, el: &ActiveEventLoop) {
            if let Err(e) = r {
                self.error = Some(e);
                el.exit();
            }
            if self.ui.title_changed {
                self.ui.title_changed = false;
                if let Some(w) = &self.window {
                    w.set_title(&self.ui.s.title);
                }
            }
            self.redraw();
        }

        fn key(&mut self, event: winit::event::KeyEvent) -> Result<(), String> {
            let mods = self.mods();
            // The view's shortcuts go first, unless what has focus uses the key.
            if let Some(name) = key_name(&event) {
                if self.ui.shortcut(&name, mods)? {
                    return Ok(());
                }
            }
            let input = self.ui.focused_kind() == "input";
            if mods.command {
                if let WKey::Character(c) = &event.logical_key {
                    let nav = match c.to_lowercase().as_str() {
                        "a" => Some(Nav::Edit(Key::All)),
                        "c" => Some(Nav::Copy),
                        "x" => Some(Nav::Cut),
                        "v" => Some(Nav::Paste),
                        "z" if mods.shift => Some(Nav::Redo),
                        "z" => Some(Nav::Undo),
                        "y" => Some(Nav::Redo),
                        _ => None,
                    };
                    if let Some(n) = nav {
                        return self.ui.nav(n, mods);
                    }
                }
            }
            let mac_line = cfg!(target_os = "macos") && mods.command;
            let nav = match &event.logical_key {
                WKey::Named(NamedKey::Backspace) => Some(Nav::Edit(Key::Back)),
                WKey::Named(NamedKey::Delete) => Some(Nav::Edit(Key::Delete)),
                WKey::Named(NamedKey::ArrowLeft) if mac_line => Some(Nav::Edit(Key::Home)),
                WKey::Named(NamedKey::ArrowRight) if mac_line => Some(Nav::Edit(Key::End)),
                WKey::Named(NamedKey::ArrowLeft) => Some(Nav::Edit(Key::Left)),
                WKey::Named(NamedKey::ArrowRight) => Some(Nav::Edit(Key::Right)),
                WKey::Named(NamedKey::Home) => Some(Nav::Edit(Key::Home)),
                WKey::Named(NamedKey::End) => Some(Nav::Edit(Key::End)),
                WKey::Named(NamedKey::ArrowUp) => Some(Nav::Up),
                WKey::Named(NamedKey::ArrowDown) => Some(Nav::Down),
                WKey::Named(NamedKey::Tab) => Some(Nav::Tab),
                WKey::Named(NamedKey::Enter) => Some(Nav::Enter),
                WKey::Named(NamedKey::Escape) => Some(Nav::Escape),
                WKey::Named(NamedKey::Space) if !input => Some(Nav::Space),
                _ => None,
            };
            if let Some(n) = nav {
                return self.ui.nav(n, mods);
            }
            if input && !mods.command {
                if let Some(t) = &event.text {
                    if !t.chars().any(|c| c.is_control()) {
                        return self.ui.typed(t);
                    }
                }
            }
            Ok(())
        }
    }

    /// A key's name as shortcuts write it: "s", "1", "enter", "f5". Shift doesn't change it, so
    /// shift+1 is "1", not "!".
    fn key_name(event: &winit::event::KeyEvent) -> Option<String> {
        #[cfg(any(target_os = "windows", target_os = "macos", all(unix, not(any(target_os = "ios", target_os = "android")))))]
        let key = {
            use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
            event.key_without_modifiers()
        };
        #[cfg(not(any(target_os = "windows", target_os = "macos", all(unix, not(any(target_os = "ios", target_os = "android"))))))]
        let key = event.logical_key.clone();
        Some(match key {
            WKey::Character(c) => c.to_lowercase(),
            WKey::Named(n) => match n {
                NamedKey::Enter => "enter".into(),
                NamedKey::Escape => "escape".into(),
                NamedKey::Tab => "tab".into(),
                NamedKey::Space => "space".into(),
                NamedKey::Backspace => "backspace".into(),
                NamedKey::Delete => "delete".into(),
                NamedKey::ArrowUp => "up".into(),
                NamedKey::ArrowDown => "down".into(),
                NamedKey::ArrowLeft => "left".into(),
                NamedKey::ArrowRight => "right".into(),
                NamedKey::Home => "home".into(),
                NamedKey::End => "end".into(),
                NamedKey::PageUp => "pageup".into(),
                NamedKey::PageDown => "pagedown".into(),
                other => {
                    let f = format!("{other:?}").to_lowercase();
                    if function_key(&f) {
                        f
                    } else {
                        return None;
                    }
                }
            },
            _ => return None,
        })
    }

    impl<'a, 'b> ApplicationHandler for App<'a, 'b> {
        fn resumed(&mut self, el: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let attrs = Window::default_attributes()
                .with_title(self.ui.s.title.clone())
                .with_inner_size(winit::dpi::LogicalSize::new(self.ui.s.width, self.ui.s.height))
                .with_min_inner_size(winit::dpi::LogicalSize::new(self.ui.s.min_width.max(1.0), self.ui.s.min_height.max(1.0)));
            let window = match el.create_window(attrs) {
                Ok(w) => Rc::new(w),
                Err(e) => {
                    self.error = Some(format!("can't open a window: {e}."));
                    el.exit();
                    return;
                }
            };
            let surface = softbuffer::Context::new(window.clone()).and_then(|ctx| softbuffer::Surface::new(&ctx, window.clone()));
            match surface {
                Ok(s) => self.surface = Some(s),
                Err(e) => {
                    self.error = Some(format!("can't draw in the window: {e}."));
                    el.exit();
                    return;
                }
            }
            self.window = Some(window);
            let (w, h, _) = self.logical();
            let r = self.ui.resize(w, h);
            self.check(r, el);
        }

        fn new_events(&mut self, el: &ActiveEventLoop, cause: StartCause) {
            if let StartCause::ResumeTimeReached { .. } = cause {
                let r = self.ui.fire_timers().map(|_| ());
                self.check(r, el);
            }
        }

        fn about_to_wait(&mut self, el: &ActiveEventLoop) {
            if self.error.is_some() {
                return;
            }
            // Wake for the caret's next blink, or the next timer, whichever is sooner.
            let timer = self.ui.next_timer().map(|d| self.ui.started + std::time::Duration::from_secs_f64(d.max(0.0)));
            let wake = match (self.ui.next_blink(), timer) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
            match wake {
                Some(t) => el.set_control_flow(ControlFlow::WaitUntil(t)),
                None => el.set_control_flow(ControlFlow::Wait),
            }
        }

        fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
            if self.error.is_some() {
                el.exit();
                return;
            }
            match event {
                WindowEvent::CloseRequested => el.exit(),
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    let (w, h, _) = self.logical();
                    let r = self.ui.resize(w, h);
                    self.check(r, el);
                }
                WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
                WindowEvent::CursorMoved { position, .. } => {
                    let (_, _, scale) = self.logical();
                    let p = (position.x as f32 / scale, position.y as f32 / scale);
                    self.mouse = Some(p);
                    let before = (self.ui.hovered(), self.ui.pointer(p.0, p.1));
                    let dragging = self.ui.drag != Drag::None;
                    let r = self.ui.moved(p.0, p.1);
                    if let Some(w) = &self.window {
                        w.set_cursor(match self.ui.pointer(p.0, p.1) {
                            2 => CursorIcon::Text,
                            1 => CursorIcon::Pointer,
                            _ => CursorIcon::Default,
                        });
                    }
                    // Only draw again when something under the mouse changed, or while dragging.
                    if dragging || r.is_err() || before != (self.ui.hovered(), self.ui.pointer(p.0, p.1)) || self.ui.pressed.is_some() {
                        self.check(r, el);
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let Some((x, y)) = self.mouse else { return };
                    let (_, _, scale) = self.logical();
                    let dy = match delta {
                        MouseScrollDelta::LineDelta(_, lines) => -lines * 48.0,
                        MouseScrollDelta::PixelDelta(p) => -(p.y as f32) / scale,
                    };
                    if self.ui.wheel(x, y, dy) {
                        let _ = self.ui.moved(x, y);
                        self.redraw();
                    }
                }
                WindowEvent::CursorLeft { .. } => {
                    self.mouse = None;
                    self.ui.st.hover = None;
                    self.redraw();
                }
                WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                    let Some((x, y)) = self.mouse else { return };
                    let mods = self.mods();
                    let r = match state {
                        ElementState::Pressed => self.ui.press(x, y, mods),
                        ElementState::Released => self.ui.release(x, y),
                    };
                    self.check(r, el);
                }
                WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                    let r = self.key(event);
                    self.check(r, el);
                }
                WindowEvent::RedrawRequested => {
                    let (lw, lh, scale) = self.logical();
                    let (Some(win), Some(surface)) = (&self.window, &mut self.surface) else { return };
                    let size = win.inner_size();
                    let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return };
                    if surface.resize(w, h).is_err() {
                        return;
                    }
                    if (self.ui.tree.w - lw).abs() > 0.5 || (self.ui.tree.h - lh).abs() > 0.5 {
                        win.request_redraw();
                    }
                    if self.ui.step_fades() {
                        win.request_redraw();
                    }
                    let c = render(&self.ui.tree, size.width as usize, size.height as usize, scale, &self.ui.paint_state(), self.ui.s.background);
                    if let Ok(mut buf) = surface.buffer_mut() {
                        let n = buf.len().min(c.px.len());
                        buf[..n].copy_from_slice(&c.px[..n]);
                        let _ = buf.present();
                    }
                }
                _ => {}
            }
        }
    }

    thread_local! {
        // winit allows one event loop per program, so every window shares it.
        static LOOP: std::cell::RefCell<Option<EventLoop<()>>> = const { std::cell::RefCell::new(None) };
    }

    fn new_loop() -> Result<EventLoop<()>, String> {
        let mut b = EventLoop::builder();
        // Lust programs run on a helper thread with a big stack. Windows and
        // Linux allow windows there; macOS programs run on the main thread instead.
        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::EventLoopBuilderExtWindows;
            b.with_any_thread(true);
        }
        #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
        {
            use winit::platform::wayland::EventLoopBuilderExtWayland;
            use winit::platform::x11::EventLoopBuilderExtX11;
            EventLoopBuilderExtX11::with_any_thread(&mut b, true);
            EventLoopBuilderExtWayland::with_any_thread(&mut b, true);
        }
        b.build().map_err(|e| {
            // winit's message starts with where in its code it failed; keep only the reason.
            let raw = e.to_string();
            let reason = raw.rsplit(": ").next().unwrap_or(&raw).trim_end_matches('.').to_string();
            format!("can't open a window: {reason}.")
        })
    }

    pub fn run(s: Settings, build: &mut Build) -> Result<(), String> {
        use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
        let mut el = match LOOP.with(|l| l.borrow_mut().take()) {
            Some(el) => el,
            None => new_loop()?,
        };
        let ui = Ui::new(s, build, true)?;
        let mut app = App { ui, window: None, surface: None, mouse: None, mods: ModifiersState::default(), error: None };
        let ran = el.run_app_on_demand(&mut app);
        app.surface = None;
        app.window = None;
        LOOP.with(|l| *l.borrow_mut() = Some(el));
        ran.map_err(|e| format!("the window stopped: {e}."))?;
        match app.error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

// ----- typing: the text being edited in an input -----

/// The caret and selection in the focused input. Places are byte offsets on letter boundaries;
/// `anchor` is where a selection started, equal to `caret` when nothing is selected.
#[derive(Clone, Debug, Default, PartialEq)]
struct Caret {
    caret: usize,
    anchor: usize,
}

fn prev_char(t: &str, i: usize) -> usize {
    t[..i].char_indices().next_back().map(|(p, _)| p).unwrap_or(0)
}

fn next_char(t: &str, i: usize) -> usize {
    t[i..].chars().next().map(|c| i + c.len_utf8()).unwrap_or(t.len())
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The start of the word before i (skipping spaces first), as Ctrl+Left goes.
fn prev_word(t: &str, mut i: usize) -> usize {
    while i > 0 && !is_word(t[..i].chars().next_back().unwrap()) {
        i = prev_char(t, i);
    }
    while i > 0 && is_word(t[..i].chars().next_back().unwrap()) {
        i = prev_char(t, i);
    }
    i
}

/// The end of the word after i, as Ctrl+Right goes.
fn next_word(t: &str, mut i: usize) -> usize {
    while i < t.len() && !is_word(t[i..].chars().next().unwrap()) {
        i = next_char(t, i);
    }
    while i < t.len() && is_word(t[i..].chars().next().unwrap()) {
        i = next_char(t, i);
    }
    i
}

/// The word around i, for a double-click.
fn word_at(t: &str, i: usize) -> (usize, usize) {
    let mut a = i.min(t.len());
    let mut b = a;
    while a > 0 && is_word(t[..a].chars().next_back().unwrap()) {
        a = prev_char(t, a);
    }
    while b < t.len() && is_word(t[b..].chars().next().unwrap()) {
        b = next_char(t, b);
    }
    (a, b)
}

/// What a key does in an input.
#[derive(Clone, Debug, PartialEq)]
enum Key {
    Type(String),
    Back,
    Delete,
    Left,
    Right,
    Home,
    End,
    All,
}

impl Caret {
    fn at(i: usize) -> Caret {
        Caret { caret: i, anchor: i }
    }

    fn range(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    fn selected<'a>(&self, t: &'a str) -> &'a str {
        let (a, b) = self.range();
        &t[a.min(t.len())..b.min(t.len())]
    }

    /// Keeps the caret on a letter boundary inside the text, after the text changed from outside.
    fn fit(&mut self, t: &str) {
        let fix = |mut i: usize| {
            i = i.min(t.len());
            while !t.is_char_boundary(i) {
                i -= 1;
            }
            i
        };
        self.caret = fix(self.caret);
        self.anchor = fix(self.anchor);
    }

    /// Applies a key. Returns the new text when it changed. `shift` extends the selection, and
    /// `word` moves or deletes by words (Ctrl, or Option on a Mac).
    fn key(&mut self, t: &str, k: &Key, shift: bool, word: bool) -> Option<String> {
        self.fit(t);
        let (a, b) = self.range();
        let replace = |c: &mut Caret, from: usize, to: usize, with: &str| {
            let mut s = String::with_capacity(t.len() + with.len());
            s.push_str(&t[..from]);
            s.push_str(with);
            s.push_str(&t[to..]);
            *c = Caret::at(from + with.len());
            Some(s)
        };
        let moved = |c: &mut Caret, to: usize| {
            c.caret = to;
            if !shift {
                c.anchor = to;
            }
            None
        };
        match k {
            Key::Type(s) => {
                let s: String = s.chars().filter(|c| !c.is_control()).collect();
                if s.is_empty() {
                    return None;
                }
                replace(self, a, b, &s)
            }
            Key::Back if a != b => replace(self, a, b, ""),
            Key::Delete if a != b => replace(self, a, b, ""),
            Key::Back if a == 0 => None,
            Key::Back => {
                let from = if word { prev_word(t, a) } else { prev_char(t, a) };
                replace(self, from, a, "")
            }
            Key::Delete if a >= t.len() => None,
            Key::Delete => {
                let to = if word { next_word(t, a) } else { next_char(t, a) };
                replace(self, a, to, "")
            }
            Key::Left if a != b && !shift => moved(self, a),
            Key::Right if a != b && !shift => moved(self, b),
            Key::Left => {
                let to = if word { prev_word(t, self.caret) } else { prev_char(t, self.caret) };
                moved(self, to)
            }
            Key::Right => {
                let to = if word { next_word(t, self.caret) } else { next_char(t, self.caret) };
                moved(self, to)
            }
            Key::Home => moved(self, 0),
            Key::End => moved(self, t.len()),
            Key::All => {
                self.anchor = 0;
                self.caret = t.len();
                None
            }
        }
    }
}

/// The place in the text nearest to x pixels from its start.
fn place_at(t: &str, x: f32, size: f32, face: usize) -> usize {
    let mut best = (0usize, x.abs());
    let mut i = 0;
    while i < t.len() {
        i = next_char(t, i);
        let d = (text_width(&t[..i], size, face) - x).abs();
        if d < best.1 {
            best = (i, d);
        }
    }
    best.0
}

// ----- the system clipboard -----

fn clipboard_get() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

fn clipboard_set(t: &str) {
    if let Ok(mut c) = arboard::Clipboard::new() {
        let _ = c.set_text(t.to_string());
    }
}

// ----- test mode: commands instead of a window -----

fn num_text(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

fn dump(n: &Laid, depth: usize, path: &mut Vec<usize>, st: &PaintState, out: &mut String) {
    let pad = "  ".repeat(depth);
    let shown = match (&n.value, n.kind.as_str()) {
        (Some(SV::Text(t)), _) => Some(t.clone()),
        (Some(SV::Truth(b)), _) => Some(if *b { "on" } else { "off" }.to_string()),
        (Some(SV::Num(x)), "slider") => Some(num_text(*x)),
        _ => None,
    };
    let text = match shown {
        Some(t) => format!(" {t:?}"),
        None if !n.text.is_empty() => format!(" {:?}", n.text),
        None => String::new(),
    };
    let mut focus = String::new();
    if n.look.disabled {
        focus.push_str(" (disabled)");
    }
    if n.kind == "scroll" {
        focus.push_str(&format!(" (scrolled {} of {})", n.scrolled as i64, (n.content_h - n.h).max(0.0) as i64));
    }
    if st.focus.as_deref() == Some(path.as_slice()) {
        focus.push_str(" (focus");
        if n.kind == "input" {
            let (a, b) = st.caret.range();
            let t = value_text(n);
            let chars = |i: usize| t[..i.min(t.len())].chars().count();
            if a == b {
                focus.push_str(&format!(", caret {}", chars(a)));
            } else {
                focus.push_str(&format!(", selected {}..{}", chars(a), chars(b)));
            }
        }
        focus.push(')');
    }
    out.push_str(&format!("{pad}{}{text} {},{} {}x{}{focus}\n", n.kind, n.x as i64, n.y as i64, n.w as i64, n.h as i64));
    for (i, c) in n.children.iter().enumerate() {
        path.push(i);
        dump(c, depth + 1, path, st, out);
        path.pop();
    }
}

fn find_button(n: &Laid, text: &str, path: &mut Vec<usize>) -> Option<Vec<usize>> {
    if n.kind == "button" && n.text == text {
        return Some(path.clone());
    }
    for (i, c) in n.children.iter().enumerate() {
        path.push(i);
        let found = find_button(c, text, path);
        path.pop();
        if found.is_some() {
            return found;
        }
    }
    None
}

/// The n-th (from 1) input, checkbox or slider, by kind.
fn find_nth(n: &Laid, kind: &str, nth: usize) -> Option<Vec<usize>> {
    let mut all = Vec::new();
    focusable(n, &mut vec![], &mut all);
    all.into_iter().filter(|p| node_at(n, p).map(|x| x.kind == kind).unwrap_or(false)).nth(nth.checked_sub(1)?)
}

/// A key written like ctrl+shift+left.
fn parse_key(spec: &str) -> Option<(Nav, Mods)> {
    let mut mods = Mods::default();
    let mut name = "";
    for part in spec.split('+') {
        match part.trim().to_lowercase().as_str() {
            "shift" => mods.shift = true,
            "ctrl" | "cmd" => {
                mods.command = true;
                mods.word = true;
            }
            _ => name = part.trim(),
        }
    }
    let nav = match name.to_lowercase().as_str() {
        "backspace" => Nav::Edit(Key::Back),
        "delete" => Nav::Edit(Key::Delete),
        "left" => Nav::Edit(Key::Left),
        "right" => Nav::Edit(Key::Right),
        "home" => Nav::Edit(Key::Home),
        "end" => Nav::Edit(Key::End),
        "up" => Nav::Up,
        "down" => Nav::Down,
        "tab" => Nav::Tab,
        "enter" => Nav::Enter,
        "space" => Nav::Space,
        "escape" => Nav::Escape,
        "a" if mods.command => Nav::Edit(Key::All),
        "c" if mods.command => Nav::Copy,
        "x" if mods.command => Nav::Cut,
        "v" if mods.command => Nav::Paste,
        "z" if mods.command && mods.shift => Nav::Redo,
        "z" if mods.command => Nav::Undo,
        "y" if mods.command => Nav::Redo,
        _ => return None,
    };
    Some((nav, mods))
}

fn centre(ui: &Ui, path: &[usize]) -> (f32, f32) {
    node_at(&ui.tree, path).map(|n| (n.x + n.w / 2.0, n.y + n.h / 2.0)).unwrap_or((-1.0, -1.0))
}

fn test(s: Settings, build: &mut Build) -> Result<(), String> {
    use std::io::BufRead;
    let mut ui = Ui::new(s, build, false)?;
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap_or_default();
        let line = line.trim();
        let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
        let rest = rest.trim().trim_matches('"');
        let nums: Vec<f32> = rest.split_whitespace().filter_map(|x| x.parse::<f32>().ok()).collect();
        match cmd {
            "dump" => {
                let mut out = format!("window {:?} {}x{}\n", ui.s.title, ui.w as i64, ui.h as i64);
                let st = ui.paint_state();
                for (i, c) in ui.tree.children.iter().enumerate() {
                    dump(c, 1, &mut vec![i], &st, &mut out);
                }
                print!("{out}");
            }
            "click" => match find_button(&ui.tree, rest, &mut vec![]) {
                Some(p) => {
                    let (x, y) = centre(&ui, &p);
                    ui.press(x, y, Mods::default())?;
                    ui.release(x, y)?;
                }
                None => println!("(there's no button \"{rest}\" to click)"),
            },
            "check" => match nums.first().and_then(|&n| find_nth(&ui.tree, "checkbox", n as usize)) {
                Some(p) => {
                    let (x, y) = centre(&ui, &p);
                    ui.press(x, y, Mods::default())?;
                    ui.release(x, y)?;
                }
                None => println!("(there's no checkbox {rest})"),
            },
            "slide" => match (nums.first().and_then(|&n| find_nth(&ui.tree, "slider", n as usize)), nums.get(1)) {
                (Some(p), Some(&v)) => {
                    let n = node_at(&ui.tree, &p).unwrap();
                    let l = &n.look;
                    let f = if l.to == l.from { 0.0 } else { (v - l.from) / (l.to - l.from) };
                    let x = n.x + THUMB + f.clamp(0.0, 1.0) * (n.w - 2.0 * THUMB);
                    let y = n.y + n.h / 2.0;
                    ui.press(x, y, Mods::default())?;
                    ui.release(x, y)?;
                }
                _ => println!("(there's no slider {rest})"),
            },
            "focus" => {
                let mut it = rest.split_whitespace();
                let kind = it.next().unwrap_or("input");
                let nth = it.next().and_then(|x| x.parse::<usize>().ok()).unwrap_or(1);
                match find_nth(&ui.tree, kind, nth) {
                    Some(p) => ui.set_focus(Some(p), true),
                    None => println!("(there's no {kind} {nth})"),
                }
            }
            "type" => ui.typed(rest)?,
            "key" => {
                // A shortcut in the view goes first, as in a window.
                if let Ok(s) = parse_shortcut(rest) {
                    let m = Mods { shift: s.shift, command: s.command, word: s.command, alt: s.alt };
                    if ui.shortcut(&s.key, m)? {
                        continue;
                    }
                }
                match parse_key(rest) {
                    Some((nav, mods)) => ui.nav(nav, mods)?,
                    None if parse_shortcut(rest).is_ok() => {}
                    None => println!("(Lust doesn't know the key {rest})"),
                }
            }
            "press" | "move" | "release" | "dblclick" if nums.len() >= 2 => {
                let (x, y) = (nums[0], nums[1]);
                match cmd {
                    "press" => ui.press(x, y, Mods::default())?,
                    "move" => ui.moved(x, y)?,
                    "release" => ui.release(x, y)?,
                    _ => {
                        for _ in 0..2 {
                            ui.press(x, y, Mods::default())?;
                            ui.release(x, y)?;
                        }
                    }
                }
            }
            "wait" => ui.wait(nums.first().copied().unwrap_or(0.0) as f64)?,
            "scroll" if nums.len() >= 2 => {
                let mut all = Vec::new();
                fn every_scroll(n: &Laid, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
                    if n.kind == "scroll" {
                        out.push(path.clone());
                    }
                    for (i, c) in n.children.iter().enumerate() {
                        path.push(i);
                        every_scroll(c, path, out);
                        path.pop();
                    }
                }
                every_scroll(&ui.tree, &mut vec![], &mut all);
                match all.get((nums[0] as usize).saturating_sub(1)).and_then(|p| node_at(&ui.tree, p)) {
                    Some(n) => {
                        let (x, y) = (n.x + 4.0, n.y + 4.0);
                        ui.wheel(x, y, nums[1]);
                    }
                    None => println!("(there's no scroll {})", nums[0]),
                }
            }
            "hover" => {
                ui.st.hover = find_button(&ui.tree, rest, &mut vec![]).and_then(|p| node_at(&ui.tree, &p)).map(|b| (b.x + 1.0, b.y + 1.0));
            }
            "size" if nums.len() >= 2 => ui.resize(nums[0], nums[1])?,
            // Paints the window N times without saving, to measure how long painting takes.
            "paint" => {
                let n = nums.first().copied().unwrap_or(1.0) as usize;
                let t = std::time::Instant::now();
                for _ in 0..n {
                    render(&ui.tree, (ui.w * 2.0) as usize, (ui.h * 2.0) as usize, 2.0, &ui.paint_state(), ui.s.background);
                }
                eprintln!("painted {n} times in {} ms", t.elapsed().as_millis());
            }
            "snapshot" => {
                let scale = 2.0;
                let c = render(&ui.tree, (ui.w * scale) as usize, (ui.h * scale) as usize, scale, &ui.paint_state(), ui.s.background);
                std::fs::write(rest, png(&c)).map_err(|e| format!("can't write {rest}: {e}"))?;
            }
            _ => {}
        }
    }
    Ok(())
}

// ----- a small PNG writer, for snapshots -----

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xffffffffu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb88320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

fn png(c: &Canvas) -> Vec<u8> {
    let mut raw = Vec::with_capacity((c.w * 3 + 1) * c.h);
    for y in 0..c.h {
        raw.push(0);
        for x in 0..c.w {
            let p = c.px[y * c.w + x];
            raw.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
        }
    }
    // zlib with stored (uncompressed) blocks: simple, and fine for snapshots.
    let mut z = vec![0x78, 0x01];
    let chunks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, ch) in chunks.iter().enumerate() {
        z.push(if i + 1 == chunks.len() { 1 } else { 0 });
        let len = ch.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(ch);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut chunk = |kind: &[u8], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(c.w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(c.h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    out
}
