// Screens for Lust: views drawn by Lust itself, from the pixels up.
//
// A view is turned into a tree of Nodes each time it's drawn. This file lays the tree out,
// paints it into a buffer of pixels, opens the window and passes clicks back. Only three small
// libraries are borrowed: winit opens the window, softbuffer puts our pixels in it, and fontdue
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
    Nested(Vec<(String, SV)>),
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: String,
    pub text: String,
    pub style: Vec<(String, SV)>,
    pub children: Vec<Node>,
    /// Which of the view's actions a click runs.
    pub action: Option<usize>,
}

impl Node {
    pub fn new(kind: &str) -> Node {
        Node { kind: kind.to_string(), text: String::new(), style: vec![], children: vec![], action: None }
    }
}

pub type Action = Box<dyn FnMut() -> Result<(), String>>;

// ----- fonts -----

pub const FONT_FILES: [&str; 4] = ["Inter-Regular.otf", "Inter-Medium.otf", "Inter-SemiBold.otf", "Inter-Bold.otf"];

pub static FONTS: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/Inter-Regular.otf"),
    include_bytes!("../assets/fonts/Inter-Medium.otf"),
    include_bytes!("../assets/fonts/Inter-SemiBold.otf"),
    include_bytes!("../assets/fonts/Inter-Bold.otf"),
];

thread_local! {
    static LOADED: Vec<fontdue::Font> = FONTS
        .iter()
        .map(|b| fontdue::Font::from_bytes(*b, fontdue::FontSettings::default()).expect("Lust's font"))
        .collect();
    static GLYPHS: std::cell::RefCell<HashMap<(usize, char, u32), (fontdue::Metrics, Vec<u8>)>> = std::cell::RefCell::new(HashMap::new());
}

fn face(weight: f32) -> usize {
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

/// How wide a line of text is, in pixels at this size.
fn text_width(text: &str, size: f32, weight: f32) -> f32 {
    LOADED.with(|fonts| {
        let f = &fonts[face(weight)];
        let mut w = 0.0;
        let mut prev: Option<char> = None;
        for ch in text.chars() {
            if let Some(p) = prev {
                w += f.horizontal_kern(p, ch, size).unwrap_or(0.0);
            }
            w += f.metrics(ch, size).advance_width;
            prev = Some(ch);
        }
        w
    })
}

/// (ascent, height of a line) at this size.
fn line_metrics(size: f32, weight: f32) -> (f32, f32) {
    LOADED.with(|fonts| match fonts[face(weight)].horizontal_line_metrics(size) {
        Some(m) => (m.ascent, (m.ascent - m.descent + m.line_gap).ceil()),
        None => (size * 0.8, (size * 1.25).ceil()),
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
    };
    match kind {
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
            other => {
                // A colour on its own: a button's colour, or text's.
                if let Some(c) = colour(other) {
                    if kind == "button" {
                        l.background = Some(c);
                        l.color = if light(c) { INK } else { 0xffffff };
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
fn wrap_lines(text: &str, size: f32, weight: f32, max_w: f32, wrap: bool) -> Vec<String> {
    let fits = |s: &str| text_width(s, size, weight) <= max_w + 0.5;
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
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    children: Vec<Laid>,
}

fn lay(n: &Node) -> Laid {
    let mut look = base_look(&n.kind);
    apply(&mut look, &n.kind, &n.style);
    let hover = match hover_style(&n.style) {
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
        action: n.action,
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
        children: n.children.iter().map(lay).collect(),
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
    if s == Size::Fill || c.kind == "space" {
        Some(1.0)
    } else {
        None
    }
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
            let lines = wrap_lines(&n.text, l.size, l.weight, inner, l.wrap);
            let (_, lh) = line_metrics(l.size, l.weight);
            let w = lines.iter().map(|s| text_width(s, l.size, l.weight)).fold(0.0, f32::max);
            (w.ceil(), lh * lines.len() as f32)
        }
        "space" => (0.0, 0.0),
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
        n.lines = wrap_lines(&n.text, l.size, l.weight, iw, l.wrap);
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
    let mut top = lay(root);
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
        Canvas { w, h, scale, alpha: 1.0, px: vec![0xffffff; w * h] }
    }

    fn blend(&mut self, x: i64, y: i64, c: u32, a: f32) {
        let a = a * self.alpha;
        if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 || a <= 0.0 {
            return;
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
        for py in (y0.floor() as i64)..(y1.ceil() as i64) {
            for px in (x0.floor() as i64)..(x1.ceil() as i64) {
                let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                let mut cover = (0.5 - outside(fx, fy, x0, y0, x1, y1, r)).clamp(0.0, 1.0);
                if let Some(b) = b {
                    let inner = outside(fx, fy, x0 + b, y0 + b, x1 - b, y1 - b, (r - b).max(0.0));
                    cover *= 1.0 - (0.5 - inner).clamp(0.0, 1.0);
                }
                self.blend(px, py, c, cover * alpha);
            }
        }
    }

    fn text(&mut self, text: &str, x: f32, y: f32, size: f32, weight: f32, c: u32) {
        let s = self.scale;
        let px_size = size * s;
        let (ascent, _) = line_metrics(px_size, weight);
        let fi = face(weight);
        let mut pen = x * s;
        let base = (y * s + ascent).round();
        let mut prev: Option<char> = None;
        for ch in text.chars() {
            let (m, bmp) = GLYPHS.with(|g| {
                g.borrow_mut()
                    .entry((fi, ch, px_size.to_bits()))
                    .or_insert_with(|| LOADED.with(|f| f[fi].rasterize(ch, px_size)))
                    .clone()
            });
            if let Some(p) = prev {
                pen += LOADED.with(|f| f[fi].horizontal_kern(p, ch, px_size).unwrap_or(0.0));
            }
            let gx = (pen + m.xmin as f32).round() as i64;
            let gy = base as i64 - m.ymin as i64 - m.height as i64;
            for row in 0..m.height {
                for col in 0..m.width {
                    let a = bmp[row * m.width + col] as f32 / 255.0;
                    self.blend(gx + col as i64, gy + row as i64, c, a);
                }
            }
            pen += m.advance_width;
            prev = Some(ch);
        }
    }
}

fn paint(c: &mut Canvas, n: &Laid, hover: Option<(f32, f32)>) {
    let over = hover.map(|(hx, hy)| hx >= n.x && hy >= n.y && hx < n.x + n.w && hy < n.y + n.h).unwrap_or(false);
    let l = if over { n.hover.as_ref().unwrap_or(&n.look) } else { &n.look };
    let was = c.alpha;
    c.alpha *= l.opacity;
    if l.shadow {
        for i in 1..=6 {
            let e = i as f32;
            c.rect(n.x - e + 0.0, n.y - e + 3.0, n.w + 2.0 * e, n.h + 2.0 * e, l.radius + e, 0x000000, 0.012);
        }
    }
    if let Some(bg) = l.background {
        c.rect(n.x, n.y, n.w, n.h, l.radius, bg, 1.0);
    }
    if let Some(b) = l.border {
        c.shape(n.x, n.y, n.w, n.h, l.radius, Some(l.border_w), b, 1.0);
    }
    if shows_text(&n.kind) {
        let (_, lh) = line_metrics(l.size, l.weight);
        let inner_w = n.w - l.pad[1] - l.pad[3];
        let block = lh * n.lines.len() as f32;
        let top = n.y + l.pad[0] + ((n.h - l.pad[0] - l.pad[2] - block) / 2.0).max(0.0);
        for (i, line) in n.lines.iter().enumerate() {
            let tw = text_width(line, l.size, l.weight);
            let tx = n.x + l.pad[3] + offset(l.halign, inner_w - tw);
            let ty = top + lh * i as f32;
            c.text(line, tx, ty, l.size, l.weight, l.color);
            if l.strike {
                c.rect(tx, ty + lh * 0.55, tw, (l.size / 14.0).max(1.0), 0.0, l.color, 1.0);
            }
        }
    }
    for ch in &n.children {
        paint(c, ch, hover);
    }
    c.alpha = was;
}

fn render(tree: &Laid, w: usize, h: usize, scale: f32, hover: Option<(f32, f32)>, background: u32) -> Canvas {
    let mut c = Canvas::new(w, h, scale);
    c.px.iter_mut().for_each(|p| *p = background);
    paint(&mut c, tree, hover);
    c
}

/// The deepest node with an action at this point.
fn hit(n: &Laid, x: f32, y: f32) -> Option<usize> {
    if x < n.x || y < n.y || x >= n.x + n.w || y >= n.y + n.h {
        return None;
    }
    for c in n.children.iter().rev() {
        if let Some(a) = hit(c, x, y) {
            return Some(a);
        }
    }
    n.action
}

fn wants_hand(n: &Laid, x: f32, y: f32) -> bool {
    hit(n, x, y).is_some()
}

// ----- the window -----

pub struct Settings {
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub background: u32,
}

pub fn settings(style: &[(String, SV)]) -> Settings {
    let mut s = Settings { title: "Lust".into(), width: 640.0, height: 480.0, background: 0xffffff };
    for (k, v) in style {
        match (k.as_str(), v) {
            ("title", SV::Text(t)) => s.title = t.clone(),
            ("width", SV::Num(n)) => s.width = *n,
            ("height", SV::Num(n)) => s.height = *n,
            ("background", SV::Text(t)) => s.background = colour(t).unwrap_or(0xffffff),
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

mod windowed {
    use super::*;
    use std::num::NonZeroU32;
    use std::rc::Rc;
    use winit::application::ApplicationHandler;
    use winit::event::{ElementState, MouseButton, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, EventLoop};
    use winit::window::{CursorIcon, Window, WindowId};

    struct App<'a, 'b> {
        s: Settings,
        build: &'a mut Build<'b>,
        window: Option<Rc<Window>>,
        surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
        tree: Option<(Laid, Vec<Action>)>,
        mouse: Option<(f32, f32)>,
        pressed: Option<usize>,
        error: Option<String>,
    }

    impl<'a, 'b> App<'a, 'b> {
        fn logical(&self) -> (f32, f32, f32) {
            let w = self.window.as_ref().unwrap();
            let scale = w.scale_factor() as f32;
            let size = w.inner_size();
            (size.width as f32 / scale, size.height as f32 / scale, scale)
        }

        fn rebuild(&mut self) {
            match (self.build)() {
                Ok((node, actions)) => {
                    let (w, h, _) = self.logical();
                    self.tree = Some((lay_out(&node, w, h), actions));
                }
                Err(e) => self.error = Some(e),
            }
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
    }

    impl<'a, 'b> ApplicationHandler for App<'a, 'b> {
        fn resumed(&mut self, el: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let attrs = Window::default_attributes()
                .with_title(self.s.title.clone())
                .with_inner_size(winit::dpi::LogicalSize::new(self.s.width, self.s.height));
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
            self.rebuild();
        }

        fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
            if self.error.is_some() {
                el.exit();
                return;
            }
            match event {
                WindowEvent::CloseRequested => el.exit(),
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => self.rebuild(),
                WindowEvent::CursorMoved { position, .. } => {
                    let (_, _, scale) = self.logical();
                    let p = (position.x as f32 / scale, position.y as f32 / scale);
                    self.mouse = Some(p);
                    if let (Some(w), Some((t, _))) = (&self.window, &self.tree) {
                        w.set_cursor(if wants_hand(t, p.0, p.1) { CursorIcon::Pointer } else { CursorIcon::Default });
                        w.request_redraw();
                    }
                }
                WindowEvent::CursorLeft { .. } => {
                    self.mouse = None;
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
                WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                    let Some((x, y)) = self.mouse else { return };
                    let at = self.tree.as_ref().and_then(|(t, _)| hit(t, x, y));
                    match state {
                        ElementState::Pressed => self.pressed = at,
                        ElementState::Released => {
                            if at.is_some() && at == self.pressed {
                                if let Some((_, actions)) = &mut self.tree {
                                    if let Err(e) = (actions[at.unwrap()])() {
                                        self.error = Some(e);
                                        el.exit();
                                        return;
                                    }
                                }
                                self.rebuild();
                            }
                            self.pressed = None;
                        }
                    }
                }
                WindowEvent::RedrawRequested => {
                    let (lw, lh, scale) = self.logical();
                    let (Some(win), Some(surface)) = (&self.window, &mut self.surface) else { return };
                    let size = win.inner_size();
                    let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return };
                    if surface.resize(w, h).is_err() {
                        return;
                    }
                    if let Some((t, _)) = &self.tree {
                        if (t.w - lw).abs() > 0.5 || (t.h - lh).abs() > 0.5 {
                            win.request_redraw();
                        }
                    }
                    let Some((tree, _)) = &self.tree else { return };
                    let c = render(tree, size.width as usize, size.height as usize, scale, self.mouse, self.s.background);
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
        let mut app = App { s, build, window: None, surface: None, tree: None, mouse: None, pressed: None, error: None };
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

// ----- test mode: commands instead of a window -----

fn dump(n: &Laid, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    let text = if n.text.is_empty() { String::new() } else { format!(" {:?}", n.text) };
    out.push_str(&format!("{pad}{}{text} {},{} {}x{}\n", n.kind, n.x as i64, n.y as i64, n.w as i64, n.h as i64));
    for c in &n.children {
        dump(c, depth + 1, out);
    }
}

fn find_button<'a>(n: &'a Laid, text: &str) -> Option<&'a Laid> {
    if n.action.is_some() && n.text == text {
        return Some(n);
    }
    n.children.iter().find_map(|c| find_button(c, text))
}

fn test(s: Settings, build: &mut Build) -> Result<(), String> {
    use std::io::BufRead;
    let (mut w, mut h) = (s.width as f32, s.height as f32);
    let (node, mut actions) = build()?;
    let mut tree = lay_out(&node, w, h);
    let mut hover: Option<(f32, f32)> = None;
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap_or_default();
        let line = line.trim();
        let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
        let rest = rest.trim().trim_matches('"');
        match cmd {
            "dump" => {
                let mut out = format!("window {:?} {}x{}\n", s.title, w as i64, h as i64);
                for c in &tree.children {
                    dump(c, 1, &mut out);
                }
                print!("{out}");
            }
            "click" => match find_button(&tree, rest).and_then(|b| b.action) {
                Some(a) => {
                    actions[a]()?;
                    let (n, a2) = build()?;
                    tree = lay_out(&n, w, h);
                    actions = a2;
                }
                None => println!("(there's no button \"{rest}\" to click)"),
            },
            "hover" => {
                hover = find_button(&tree, rest).map(|b| (b.x + 1.0, b.y + 1.0));
            }
            "size" => {
                let mut it = rest.split_whitespace().filter_map(|x| x.parse::<f32>().ok());
                if let (Some(a), Some(b)) = (it.next(), it.next()) {
                    w = a;
                    h = b;
                    tree = lay_out(&node_again(build, &mut actions)?, w, h);
                }
            }
            "snapshot" => {
                let scale = 2.0;
                let c = render(&tree, (w * scale) as usize, (h * scale) as usize, scale, hover, s.background);
                std::fs::write(rest, png(&c)).map_err(|e| format!("can't write {rest}: {e}"))?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn node_again(build: &mut Build, actions: &mut Vec<Action>) -> Result<Node, String> {
    let (n, a) = build()?;
    *actions = a;
    Ok(n)
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
