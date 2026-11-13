// Views, continued: radio buttons, switches, number boxes, icons, and SVG pictures.
//
// This is part of the same module as native_ui.rs (lust includes both, and built programs get
// both, one after the other), kept apart so neither file grows too long. The icons' shapes are
// in icons.rs, made from Lucide's.

// ----- radio buttons -----

/// A radio group's buttons: one for each choice, laid out down (or across), the one that's the
/// value picked. Their look comes from the group's, so `[small, purple]` on it changes them all.
fn radio_group(n: &Node, laid: &mut Laid) {
    let now = match &n.value {
        Some(SV::Text(t)) => t.clone(),
        _ => String::new(),
    };
    if laid.look.across && laid.look.gap == 6.0 {
        laid.look.gap = 16.0;
    }
    let group = laid.look.clone();
    laid.children = n
        .options
        .iter()
        .enumerate()
        .map(|(i, o)| {
            let mut b = lay(&Node::new("radiobtn"));
            b.text = o.clone();
            b.index = i;
            b.value = Some(SV::Truth(*o == now));
            b.action = laid.action;
            b.look.accent = group.accent;
            b.look.color = group.color;
            b.look.size = group.size;
            b.look.weight = group.weight;
            b.look.font = group.font;
            b.look.pad = [1.0, 0.0, 1.0, 26.0];
            b.look.min_h = 20.0;
            b.look.wrap = false;
            if laid.action.is_some() {
                let mut over = b.look.clone();
                over.border = Some(nt(0x868e96));
                b.hover = Some(over);
            }
            b
        })
        .collect();
}

/// The round button on the left of a radio button: an empty ring, or filled with a dot in it.
fn paint_radio_button(c: &mut Canvas, n: &Laid, l: &Look) {
    let (x, y) = (n.x, n.y + ((n.h - 18.0) / 2.0).round());
    if matches!(n.value, Some(SV::Truth(true))) {
        c.rect(x, y, 18.0, 18.0, 9.0, l.accent, 1.0);
        c.rect(x + 5.0, y + 5.0, 8.0, 8.0, 4.0, 0xffffff, 1.0);
    } else {
        c.rect(x, y, 18.0, 18.0, 9.0, l.background.unwrap_or(nt(0xffffff)), 1.0);
        c.shape(x, y, 18.0, 18.0, 9.0, Some(1.5), l.border.unwrap_or(nt(0xadb5bd)), 1.0);
    }
}

/// The ring round a focused radio group: round the picked button, or the first if none is.
fn radio_ring(c: &mut Canvas, n: &Laid) {
    let Some(b) = n.children.iter().find(|b| matches!(b.value, Some(SV::Truth(true)))).or(n.children.first()) else { return };
    let (x, y) = (b.x, b.y + ((b.h - 18.0) / 2.0).round());
    let r = 9.0 + RING;
    c.shape(x - RING, y - RING, 18.0 + 2.0 * RING, 18.0 + 2.0 * RING, r, Some(RING), mix(b.look.accent, nt(0xffffff), 0.65), 1.0);
}

// ----- checkboxes and switches, with words -----

const SWITCH_W: f32 = 36.0;
const SWITCH_H: f32 = 20.0;
/// How long a switch's knob takes to slide across.
const SWITCH_SECS: f64 = 0.15;

/// A checkbox or switch: without words, just the box or the switch; with words, those on the
/// left and the words after them, all of it clicked.
fn with_words(laid: &mut Laid) {
    let (w, h) = if laid.kind == "switch" { (SWITCH_W, SWITCH_H) } else { (18.0, 18.0) };
    let l = &mut laid.look;
    if laid.text.is_empty() {
        if l.width == Size::Auto {
            l.width = Size::Fixed(w);
        }
        if l.height == Size::Auto {
            l.height = Size::Fixed(h);
        }
    } else {
        l.pad[3] += w + 8.0;
        l.min_h = l.min_h.max(h);
        l.wrap = false;
    }
}

/// A switch: a pill that's grey when off and coloured when on, with a white knob that slides to
/// the right when it's on. `knob` is where the knob is while it slides, from 0 to 1.
fn paint_switch(c: &mut Canvas, n: &Laid, l: &Look, ring: bool, knob: Option<f32>) {
    let on = matches!(n.value, Some(SV::Truth(true)));
    let (x, y) = (n.x, n.y + ((n.h - SWITCH_H) / 2.0).round());
    let t = knob.unwrap_or(if on { 1.0 } else { 0.0 });
    if ring {
        let r = SWITCH_H / 2.0 + RING;
        c.shape(x - RING, y - RING, SWITCH_W + 2.0 * RING, SWITCH_H + 2.0 * RING, r, Some(RING), mix(l.accent, nt(0xffffff), 0.65), 1.0);
    }
    c.rect(x, y, SWITCH_W, SWITCH_H, SWITCH_H / 2.0, mix(nt(0xced4da), l.accent, t), 1.0);
    let kx = x + 2.0 + (SWITCH_W - SWITCH_H) * t;
    c.rect(kx, y + 3.0, 16.0, 16.0, 8.0, 0x000000, 0.12);
    c.rect(kx, y + 2.0, 16.0, 16.0, 8.0, 0xffffff, 1.0);
}

// ----- number boxes -----

/// How wide a number box's up and down arrows are, on its right.
const SPIN_W: f32 = 26.0;

/// A number as a number box shows it: 42, 0.5, -3. Tiny leftovers of adding decimals (0.1 + 0.2)
/// are rounded away.
fn number_text(x: f64, whole: bool) -> String {
    if whole || (x.fract() == 0.0 && x.abs() < 1e15) {
        return format!("{}", x.round() as i64);
    }
    let x = if x.abs() < 1e6 { (x * 1e9).round() / 1e9 } else { x };
    format!("{x}")
}

/// What's typed in a number box as the number it means, if it's one the box can keep: written
/// with digits (a minus or plus in front, and for a float a decimal point), whole for an int,
/// and between its min and max.
fn number_parse(t: &str, l: &Look, whole: bool) -> Option<f64> {
    let t = t.trim();
    let body = t.strip_prefix(['-', '+']).unwrap_or(t);
    let (int, frac) = match body.split_once('.') {
        Some((a, b)) => (a, Some(b)),
        None => (body, None),
    };
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    if !digits(int) || !frac.is_none_or(digits) || (int.is_empty() && frac.is_none_or(|f| f.is_empty())) {
        return None;
    }
    if whole && frac.is_some() {
        return None;
    }
    let x: f64 = t.parse().ok()?;
    // An int has to fit in an int, and keep every digit.
    if !x.is_finite() || (whole && x.abs() > 9.007_199_254_740_991e15) {
        return None;
    }
    (x >= l.min && x <= l.max).then_some(x)
}

/// A number box's number moved `steps` steps (up for more, down for less), kept between its min
/// and max.
fn number_step(x: f64, steps: f64, l: &Look, whole: bool) -> f64 {
    let step = if l.num_step > 0.0 && l.num_step.is_finite() { l.num_step } else { 1.0 };
    let mut v = x + steps * step;
    if v.abs() < 1e6 {
        v = (v * 1e9).round() / 1e9;
    }
    if whole {
        v = v.round();
    }
    // A min or max with a fraction keeps a whole number inside it.
    let (lo, hi) = if whole { (l.min.ceil(), l.max.floor()) } else { (l.min, l.max) };
    if lo <= hi {
        v = v.clamp(lo, hi);
    }
    v
}

/// A number box: drawn as an input, with a red edge when what's typed can't be kept, and its up
/// and down arrows on the right.
fn paint_number(c: &mut Canvas, n: &Laid, l: &Look, focused: bool, st: &PaintState) {
    paint_input(c, n, focused, st);
    if n.bad {
        c.shape(n.x, n.y, n.w, n.h, l.radius, Some(1.5), 0xff8787, 1.0);
    }
    let x0 = n.x + n.w - SPIN_W;
    let col = mix(l.color, nt(0xffffff), 0.35);
    c.rect(x0, n.y + 5.0, 1.0, (n.h - 10.0).max(0.0), 0.0, nt(0xdee2e6), 1.0);
    let cx = x0 + SPIN_W / 2.0;
    let (up, down) = (n.y + n.h * 0.32, n.y + n.h * 0.68);
    c.line(cx - 3.5, up + 1.5, cx, up - 1.5, 1.5, col);
    c.line(cx, up - 1.5, cx + 3.5, up + 1.5, 1.5, col);
    c.line(cx - 3.5, down - 1.5, cx, down + 1.5, 1.5, col);
    c.line(cx, down + 1.5, cx + 3.5, down - 1.5, 1.5, col);
}

impl<'a, 'b> Ui<'a, 'b> {
    /// New text in an input, text box or number box. A number box keeps what's typed on show,
    /// and changes its field only when it's a number that fits.
    fn set_text(&mut self, path: &[usize], text: String) -> Result<(), String> {
        let Some(n) = node_at(&self.tree, path) else { return Ok(()) };
        if n.kind == "datePicker" {
            return self.set_date_text(path, text);
        }
        if n.kind != "number" {
            return self.run(path, SV::Text(text));
        }
        let (parsed, now) = (number_parse(&text, &n.look, n.whole), n.num);
        self.draft = Some((path.to_vec(), text));
        match parsed {
            Some(v) if v != now => self.run(path, SV::Num(v)),
            _ => {
                self.relayout();
                Ok(())
            }
        }
    }

    /// Moves a number box's number on by some steps (back, below zero), from the field's number
    /// (not what's typed), and shows it.
    fn spin(&mut self, path: &[usize], steps: f64) -> Result<(), String> {
        let Some(n) = node_at(&self.tree, path) else { return Ok(()) };
        if n.action.is_none() || steps == 0.0 {
            return Ok(());
        }
        let (v, now) = (number_step(n.num, steps, &n.look, n.whole), n.num);
        let typed = self.draft.as_ref().is_some_and(|(p, _)| p == path);
        if typed {
            self.draft = None;
        }
        if v != now {
            self.run(path, SV::Num(v))?;
        } else if typed {
            self.relayout();
        }
        if self.st.focus.as_deref() == Some(path) {
            let end = self.focused().map(|n| value_text(n).len()).unwrap_or(0);
            self.st.caret = Caret::at(end);
        }
        Ok(())
    }

    /// The mouse wheel over a number box that has focus moves its number a step a notch: up for
    /// more, as on Windows. Gives None otherwise, so the wheel scrolls instead (and a scroll down
    /// a long form doesn't change a number on the way past).
    fn wheel_number(&mut self, x: f32, y: f32, dy: f32) -> Option<Result<(), String>> {
        let path = self.hit_at(x, y)?;
        if node_at(&self.tree, &path)?.kind != "number" || self.st.focus.as_ref() != Some(&path) {
            return None;
        }
        let was = if self.wheel_acc.0.as_ref() == Some(&path) { self.wheel_acc.1 } else { 0.0 };
        let acc = was + dy;
        let steps = (acc / 48.0).trunc();
        self.wheel_acc = (Some(path.clone()), acc - steps * 48.0);
        Some(self.spin(&path, -steps as f64))
    }

    /// A switch was flipped from the mouse or keyboard: in a window, its knob slides.
    fn flipped(&mut self, path: &[usize]) {
        if self.blink.is_none() || node_at(&self.tree, path).is_none_or(|n| n.kind != "switch") {
            return;
        }
        let now = self.now();
        self.flips.retain(|_, &mut f| now - f < SWITCH_SECS);
        self.flips.insert(path.to_vec(), now);
    }
}

// ----- icons -----

/// How much room an icon takes before a button's words: the icon, and a gap if there are words.
fn icon_room(n: &Laid) -> f32 {
    if n.kind != "button" || n.look.icon.is_empty() {
        return 0.0;
    }
    button_icon_size(&n.look) + if n.text.is_empty() { 0.0 } else { 6.0 }
}

/// An icon on a button is a little bigger than its words.
fn button_icon_size(l: &Look) -> f32 {
    (l.size * 1.2).round()
}

/// An icon's shapes, by its name or another name for it.
fn icon_find(name: &str) -> Option<&'static str> {
    let name = ICON_ALIASES.iter().find(|(a, _)| *a == name).map(|(_, to)| *to).unwrap_or(name);
    ICONS.iter().find(|(n, _)| *n == name).map(|(_, d)| *d)
}

/// One shape of an icon or SVG path, as straight lines: its parts, each a list of points, and
/// whether it's filled in or drawn as a line.
#[derive(Debug)]
struct Shape {
    filled: bool,
    parts: Vec<Vec<(f32, f32)>>,
}

thread_local! {
    static ICON_SHAPES: std::cell::RefCell<HashMap<String, Option<std::rc::Rc<Vec<Shape>>>>> = std::cell::RefCell::new(HashMap::new());
}

/// An icon's shapes, worked out once.
fn icon_shapes(name: &str) -> Option<std::rc::Rc<Vec<Shape>>> {
    if let Some(s) = ICON_SHAPES.with(|c| c.borrow().get(name).cloned()) {
        return s;
    }
    let shapes = icon_find(name).map(|d| {
        std::rc::Rc::new(
            d.split(';')
                .map(|p| match p.strip_prefix('F') {
                    Some(rest) => Shape { filled: true, parts: path_parts(rest) },
                    None => Shape { filled: false, parts: path_parts(p) },
                })
                .collect(),
        )
    });
    ICON_SHAPES.with(|c| c.borrow_mut().insert(name.to_string(), shapes.clone()));
    shapes
}

/// SVG path data ("M3 6h18", "a2 2 0 0 1-2 2") as straight lines: curves and arcs are broken
/// into short pieces. Each M starts a new part. What can't be read ends the path there.
fn path_parts(d: &str) -> Vec<Vec<(f32, f32)>> {
    let b = d.as_bytes();
    let mut i = 0;
    let mut parts: Vec<Vec<(f32, f32)>> = Vec::new();
    let (mut cur, mut start) = ((0.0f32, 0.0f32), (0.0f32, 0.0f32));
    let mut cmd = 0u8;
    // The last curve's second control point, for S and T, which mirror it.
    let mut ctrl: Option<(f32, f32)> = None;
    let sep = |b: &[u8], i: &mut usize| {
        while *i < b.len() && (b[*i].is_ascii_whitespace() || b[*i] == b',') {
            *i += 1;
        }
    };
    let number = |b: &[u8], i: &mut usize| -> Option<f32> {
        sep(b, i);
        let s = *i;
        if *i < b.len() && (b[*i] == b'-' || b[*i] == b'+') {
            *i += 1;
        }
        let (mut digits, mut dot) = (false, false);
        while *i < b.len() {
            match b[*i] {
                b'0'..=b'9' => digits = true,
                b'.' if !dot => dot = true,
                _ => break,
            }
            *i += 1;
        }
        if digits && *i < b.len() && (b[*i] == b'e' || b[*i] == b'E') {
            let save = *i;
            *i += 1;
            if *i < b.len() && (b[*i] == b'-' || b[*i] == b'+') {
                *i += 1;
            }
            let e0 = *i;
            while *i < b.len() && b[*i].is_ascii_digit() {
                *i += 1;
            }
            if *i == e0 {
                *i = save;
            }
        }
        if !digits {
            *i = s;
            return None;
        }
        std::str::from_utf8(&b[s..*i]).ok()?.parse::<f32>().ok().filter(|x| x.is_finite())
    };
    // An arc's flags are one digit each, and can be written together: "a1 1 0 011 1".
    let flag = |b: &[u8], i: &mut usize| -> Option<bool> {
        sep(b, i);
        let f = match b.get(*i)? {
            b'0' => false,
            b'1' => true,
            _ => return None,
        };
        *i += 1;
        Some(f)
    };
    let mut guard = 0usize;
    loop {
        guard += 1;
        sep(b, &mut i);
        if i >= b.len() || guard > 100_000 {
            break;
        }
        if b[i].is_ascii_alphabetic() {
            cmd = b[i];
            i += 1;
            if cmd == b'Z' || cmd == b'z' {
                if let Some(p) = parts.last_mut() {
                    p.push(start);
                }
                cur = start;
                ctrl = None;
                continue;
            }
        } else if cmd == 0 || cmd == b'Z' || cmd == b'z' {
            break;
        }
        let rel = cmd.is_ascii_lowercase();
        let (ox, oy) = if rel { cur } else { (0.0, 0.0) };
        // A drawing command after Z (or at the start) starts from where the pen is.
        let ensure = |parts: &mut Vec<Vec<(f32, f32)>>, cur: (f32, f32)| {
            if parts.last().is_none_or(|p| p.last() != Some(&cur)) {
                parts.push(vec![cur]);
            }
        };
        let before = i;
        match cmd.to_ascii_uppercase() {
            b'M' => {
                let (Some(x), Some(y)) = (number(b, &mut i), number(b, &mut i)) else { break };
                cur = (ox + x, oy + y);
                start = cur;
                parts.push(vec![cur]);
                // Pairs after the first are lines.
                cmd = if rel { b'l' } else { b'L' };
                ctrl = None;
            }
            b'L' => {
                let (Some(x), Some(y)) = (number(b, &mut i), number(b, &mut i)) else { break };
                ensure(&mut parts, cur);
                cur = (ox + x, oy + y);
                parts.last_mut().unwrap().push(cur);
                ctrl = None;
            }
            b'H' => {
                let Some(x) = number(b, &mut i) else { break };
                ensure(&mut parts, cur);
                cur = (ox + x, cur.1);
                parts.last_mut().unwrap().push(cur);
                ctrl = None;
            }
            b'V' => {
                let Some(y) = number(b, &mut i) else { break };
                ensure(&mut parts, cur);
                cur = (cur.0, oy + y);
                parts.last_mut().unwrap().push(cur);
                ctrl = None;
            }
            b'C' | b'S' => {
                let c1 = if cmd.eq_ignore_ascii_case(&b'C') {
                    let (Some(x), Some(y)) = (number(b, &mut i), number(b, &mut i)) else { break };
                    (ox + x, oy + y)
                } else {
                    match ctrl {
                        Some((cx, cy)) => (2.0 * cur.0 - cx, 2.0 * cur.1 - cy),
                        None => cur,
                    }
                };
                let (Some(x2), Some(y2), Some(x), Some(y)) = (number(b, &mut i), number(b, &mut i), number(b, &mut i), number(b, &mut i)) else { break };
                let (c2, end) = ((ox + x2, oy + y2), (ox + x, oy + y));
                ensure(&mut parts, cur);
                let p = parts.last_mut().unwrap();
                for k in 1..=16 {
                    let t = k as f32 / 16.0;
                    let u = 1.0 - t;
                    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                    p.push((
                        w[0] * cur.0 + w[1] * c1.0 + w[2] * c2.0 + w[3] * end.0,
                        w[0] * cur.1 + w[1] * c1.1 + w[2] * c2.1 + w[3] * end.1,
                    ));
                }
                cur = end;
                ctrl = Some(c2);
            }
            b'Q' | b'T' => {
                let c1 = if cmd.eq_ignore_ascii_case(&b'Q') {
                    let (Some(x), Some(y)) = (number(b, &mut i), number(b, &mut i)) else { break };
                    (ox + x, oy + y)
                } else {
                    match ctrl {
                        Some((cx, cy)) => (2.0 * cur.0 - cx, 2.0 * cur.1 - cy),
                        None => cur,
                    }
                };
                let (Some(x), Some(y)) = (number(b, &mut i), number(b, &mut i)) else { break };
                let end = (ox + x, oy + y);
                ensure(&mut parts, cur);
                let p = parts.last_mut().unwrap();
                for k in 1..=12 {
                    let t = k as f32 / 12.0;
                    let u = 1.0 - t;
                    p.push((u * u * cur.0 + 2.0 * u * t * c1.0 + t * t * end.0, u * u * cur.1 + 2.0 * u * t * c1.1 + t * t * end.1));
                }
                cur = end;
                ctrl = Some(c1);
            }
            b'A' => {
                let (Some(rx), Some(ry), Some(rot)) = (number(b, &mut i), number(b, &mut i), number(b, &mut i)) else { break };
                let (Some(large), Some(sweep)) = (flag(b, &mut i), flag(b, &mut i)) else { break };
                let (Some(x), Some(y)) = (number(b, &mut i), number(b, &mut i)) else { break };
                let end = (ox + x, oy + y);
                ensure(&mut parts, cur);
                arc(parts.last_mut().unwrap(), cur, rx, ry, rot, large, sweep, end);
                cur = end;
                ctrl = None;
            }
            _ => break,
        }
        if i == before {
            break;
        }
    }
    parts
}

/// An SVG arc from `p0` to `p1` as short lines, added to `out` (SVG's own way of finding its
/// centre, from its appendix F.6.5).
#[allow(clippy::too_many_arguments)]
fn arc(out: &mut Vec<(f32, f32)>, p0: (f32, f32), rx: f32, ry: f32, rot: f32, large: bool, sweep: bool, p1: (f32, f32)) {
    let (mut rx, mut ry) = (rx.abs() as f64, ry.abs() as f64);
    let (x0, y0, x1, y1) = (p0.0 as f64, p0.1 as f64, p1.0 as f64, p1.1 as f64);
    if (x0 - x1).abs() < 1e-9 && (y0 - y1).abs() < 1e-9 {
        return;
    }
    if rx < 1e-9 || ry < 1e-9 {
        out.push(p1);
        return;
    }
    let phi = (rot as f64).to_radians();
    let (cs, sn) = (phi.cos(), phi.sin());
    let (dx, dy) = ((x0 - x1) / 2.0, (y0 - y1) / 2.0);
    let (xp, yp) = (cs * dx + sn * dy, -sn * dx + cs * dy);
    let lambda = xp * xp / (rx * rx) + yp * yp / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let num = rx * rx * ry * ry - rx * rx * yp * yp - ry * ry * xp * xp;
    let den = rx * rx * yp * yp + ry * ry * xp * xp;
    let mut coef = if den > 0.0 { (num / den).max(0.0).sqrt() } else { 0.0 };
    if large == sweep {
        coef = -coef;
    }
    let (cxp, cyp) = (coef * rx * yp / ry, -coef * ry * xp / rx);
    let (cx, cy) = (cs * cxp - sn * cyp + (x0 + x1) / 2.0, sn * cxp + cs * cyp + (y0 + y1) / 2.0);
    let angle = |ux: f64, uy: f64, vx: f64, vy: f64| (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    let t1 = angle(1.0, 0.0, (xp - cxp) / rx, (yp - cyp) / ry);
    let mut dt = angle((xp - cxp) / rx, (yp - cyp) / ry, (-xp - cxp) / rx, (-yp - cyp) / ry);
    if !sweep && dt > 0.0 {
        dt -= std::f64::consts::TAU;
    } else if sweep && dt < 0.0 {
        dt += std::f64::consts::TAU;
    }
    let steps = ((dt.abs() / (std::f64::consts::PI / 24.0)).ceil() as usize).clamp(1, 96);
    for k in 1..steps {
        let t = t1 + dt * k as f64 / steps as f64;
        let (ct, st) = (t.cos(), t.sin());
        out.push(((cx + rx * ct * cs - ry * st * sn) as f32, (cy + rx * ct * sn + ry * st * cs) as f32));
    }
    out.push(p1);
}

/// How far a point is from the line between a and b.
fn to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let (qx, qy) = (a.0 + t * dx - p.0, a.1 + t * dy - p.1);
    (qx * qx + qy * qy).sqrt()
}

/// Draws an icon `size` big with its top left at x, y, in a colour. Its lines are 1/12 of its
/// size wide with round ends, smooth at the edges and sharp at any size. An icon Lust doesn't
/// have shows as an empty grey box.
fn paint_icon(c: &mut Canvas, name: &str, x: f32, y: f32, size: f32, colour: u32) {
    let Some(shapes) = icon_shapes(name) else {
        c.shape(x + 1.0, y + 1.0, (size - 2.0).max(0.0), (size - 2.0).max(0.0), 3.0, Some(1.0), nt(0xadb5bd), 1.0);
        return;
    };
    let k = c.scale * size / 24.0;
    let (ox, oy) = (x * c.scale, y * c.scale);
    paint_shapes(c, &shapes, |p| (ox + p.0 * k, oy + p.1 * k), k, colour);
}

/// Paints shapes whose points `to_px` turns into pixels, lines `half` pixels either side of
/// their middle.
fn paint_shapes(c: &mut Canvas, shapes: &[Shape], to_px: impl Fn((f32, f32)) -> (f32, f32), half: f32, colour: u32) {
    let mut lines: Vec<((f32, f32), (f32, f32))> = Vec::new();
    let mut fills: Vec<Vec<((f32, f32), (f32, f32))>> = Vec::new();
    for s in shapes {
        let mut edges = Vec::new();
        for part in &s.parts {
            let pts: Vec<(f32, f32)> = part.iter().map(|&p| to_px(p)).collect();
            if s.filled {
                for w in 0..pts.len() {
                    edges.push((pts[w], pts[(w + 1) % pts.len()]));
                }
            } else if pts.len() == 1 {
                lines.push((pts[0], pts[0]));
            } else {
                for w in pts.windows(2) {
                    lines.push((w[0], w[1]));
                }
            }
        }
        if !edges.is_empty() {
            fills.push(edges);
        }
    }
    let all = lines.iter().chain(fills.iter().flatten());
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (a, b) in all {
        x0 = x0.min(a.0).min(b.0);
        y0 = y0.min(a.1).min(b.1);
        x1 = x1.max(a.0).max(b.0);
        y1 = y1.max(a.1).max(b.1);
    }
    if x0 > x1 {
        return;
    }
    let pad = half + 1.0;
    let [bx0, by0, bx1, by1] = c.bounds();
    let (px0, px1) = (((x0 - pad).floor() as i64).max(bx0), ((x1 + pad).ceil() as i64).min(bx1));
    let (py0, py1) = (((y0 - pad).floor() as i64).max(by0), ((y1 + pad).ceil() as i64).min(by1));
    for py in py0..py1 {
        let fy = py as f32 + 0.5;
        // Only the lines near this row can touch it.
        let near: Vec<&((f32, f32), (f32, f32))> = lines.iter().filter(|(a, b)| fy >= a.1.min(b.1) - pad && fy <= a.1.max(b.1) + pad).collect();
        for px in px0..px1 {
            let p = (px as f32 + 0.5, fy);
            let mut cover = 0.0f32;
            if !near.is_empty() {
                let mut d = f32::MAX;
                for (a, b) in &near {
                    if p.0 < a.0.min(b.0) - pad || p.0 > a.0.max(b.0) + pad {
                        continue;
                    }
                    d = d.min(to_segment(p, *a, *b));
                }
                cover = (half + 0.5 - d).clamp(0.0, 1.0);
            }
            for edges in &fills {
                // Inside by the non-zero rule, softened by how far the edge is.
                let mut wind = 0;
                let mut d = f32::MAX;
                for &(a, b) in edges {
                    d = d.min(to_segment(p, a, b));
                    if (a.1 <= p.1) != (b.1 <= p.1) {
                        let t = (p.1 - a.1) / (b.1 - a.1);
                        if a.0 + t * (b.0 - a.0) > p.0 {
                            wind += if b.1 > a.1 { 1 } else { -1 };
                        }
                    }
                }
                let f = if wind != 0 { (0.5 + d).min(1.0) } else { (0.5 - d).max(0.0) };
                cover = cover.max(f);
            }
            if cover > 0.0 {
                c.blend(px, py, colour, cover);
            }
        }
    }
}

// ----- SVG pictures -----
//
// SVG files are read and drawn by resvg (Apache-2.0 or MIT), which knows all of SVG's shapes,
// colours, gradients and transforms. A picture is drawn again for each size it's shown at, so
// it stays sharp.

/// Whether a picture's file is an SVG, by its name.
fn is_svg(path: &str) -> bool {
    path.rsplit('.').next().is_some_and(|e| e.eq_ignore_ascii_case("svg"))
}

thread_local! {
    // Each SVG file is read once; a file that isn't there or isn't an SVG is remembered as missing.
    static SVGS: std::cell::RefCell<HashMap<String, Option<std::rc::Rc<resvg::usvg::Tree>>>> = std::cell::RefCell::new(HashMap::new());
    // The SVGs drawn lately, at the sizes they were shown at.
    static SVG_DRAWN: std::cell::RefCell<HashMap<(String, u32, u32), std::rc::Rc<Picture>>> = std::cell::RefCell::new(HashMap::new());
}

fn svg_tree(path: &str) -> Option<std::rc::Rc<resvg::usvg::Tree>> {
    if let Some(t) = SVGS.with(|c| c.borrow().get(path).cloned()) {
        return t;
    }
    let tree = std::fs::read(path)
        .ok()
        .and_then(|bytes| resvg::usvg::Tree::from_data(&bytes, &resvg::usvg::Options::default()).ok())
        .map(std::rc::Rc::new);
    SVGS.with(|c| c.borrow_mut().insert(path.to_string(), tree.clone()));
    tree
}

/// An SVG drawn into pixels `w` by `h`, stretched to fill them.
fn svg_pixels(tree: &resvg::usvg::Tree, w: u32, h: u32) -> Option<Picture> {
    // Kept to a size that can't run out of memory.
    let (w, h) = (w.clamp(1, 8192), h.clamp(1, 8192));
    let mut pm = resvg::tiny_skia::Pixmap::new(w, h)?;
    let size = tree.size();
    let t = resvg::tiny_skia::Transform::from_scale(w as f32 / size.width(), h as f32 / size.height());
    resvg::render(tree, t, &mut pm.as_mut());
    // tiny-skia keeps colours multiplied by their see-through; pictures here keep them apart.
    let px = pm
        .data()
        .chunks_exact(4)
        .map(|c| match c[3] {
            0 => [0, 0, 0, 0],
            a => {
                let un = |x: u8| ((x as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8;
                [un(c[0]), un(c[1]), un(c[2]), a]
            }
        })
        .collect();
    Some(Picture { w: w as usize, h: h as usize, px })
}

/// An SVG file at its own size, for laying it out.
fn read_svg(path: &str) -> Option<Picture> {
    let tree = svg_tree(path)?;
    let size = tree.size();
    svg_pixels(&tree, size.width().round().max(1.0) as u32, size.height().round().max(1.0) as u32)
}

/// An SVG file drawn `w` by `h` pixels.
fn svg_at(path: &str, w: u32, h: u32) -> Option<std::rc::Rc<Picture>> {
    let key = (path.to_string(), w, h);
    if let Some(p) = SVG_DRAWN.with(|c| c.borrow().get(&key).cloned()) {
        return Some(p);
    }
    let tree = svg_tree(path)?;
    let p = std::rc::Rc::new(svg_pixels(&tree, w, h)?);
    SVG_DRAWN.with(|c| {
        let mut c = c.borrow_mut();
        // Sizes a window was resized through aren't kept for ever.
        if c.len() > 64 {
            c.clear();
        }
        c.insert(key, p.clone());
    });
    Some(p)
}

#[cfg(test)]
mod pieces_tests {
    use super::*;

    fn look(min: f64, max: f64, step: f64) -> Look {
        let mut l = base_look("number");
        l.min = min;
        l.max = max;
        l.num_step = step;
        l
    }

    #[test]
    fn numbers_typed() {
        let l = look(f64::NEG_INFINITY, f64::INFINITY, 1.0);
        assert_eq!(number_parse("42", &l, true), Some(42.0));
        assert_eq!(number_parse(" -7 ", &l, true), Some(-7.0));
        assert_eq!(number_parse("+3", &l, true), Some(3.0));
        assert_eq!(number_parse("2.5", &l, true), None);
        assert_eq!(number_parse("2.5", &l, false), Some(2.5));
        assert_eq!(number_parse(".5", &l, false), Some(0.5));
        assert_eq!(number_parse("5.", &l, false), Some(5.0));
        for bad in ["", "-", ".", "1x", "1e5", "inf", "NaN", "1.2.3", "--1", "١٢"] {
            assert_eq!(number_parse(bad, &l, false), None, "{bad}");
        }
        assert_eq!(number_parse("99999999999999999999", &l, true), None);
        let l = look(0.0, 10.0, 1.0);
        assert_eq!(number_parse("10", &l, true), Some(10.0));
        assert_eq!(number_parse("11", &l, true), None);
        assert_eq!(number_parse("-1", &l, true), None);
    }

    #[test]
    fn numbers_stepped() {
        assert_eq!(number_step(5.0, 3.0, &look(0.0, 10.0, 2.0), true), 10.0);
        assert_eq!(number_step(5.0, -9.0, &look(0.0, 10.0, 2.0), true), 0.0);
        assert_eq!(number_step(0.2, 1.0, &look(f64::NEG_INFINITY, f64::INFINITY, 0.1), false), 0.3);
        // A step of 0 or less moves by 1, and a fractional min keeps whole numbers inside it.
        assert_eq!(number_step(1.0, 1.0, &look(f64::NEG_INFINITY, f64::INFINITY, 0.0), true), 2.0);
        assert_eq!(number_step(0.0, -1.0, &look(0.5, 9.5, 1.0), true), 1.0);
        // min above max (made while running) leaves the number where it lands.
        assert_eq!(number_step(0.0, 1.0, &look(5.0, 1.0, 1.0), true), 1.0);
        assert_eq!(number_text(0.1 + 0.2, false), "0.3");
        assert_eq!(number_text(2.0, false), "2");
        assert_eq!(number_text(-3.0, true), "-3");
    }

    #[test]
    fn paths_read() {
        let p = path_parts("M3 6h18");
        assert_eq!(p, vec![vec![(3.0, 6.0), (21.0, 6.0)]]);
        // Relative moves, implicit lines, and closing.
        let p = path_parts("m5 12 7-7 7 7z");
        assert_eq!(p[0], vec![(5.0, 12.0), (12.0, 5.0), (19.0, 12.0), (5.0, 12.0)]);
        // A circle as two arcs ends where it started, and stays on its circle.
        let p = path_parts("M2 12A10 10 0 1 0 22 12A10 10 0 1 0 2 12Z");
        for &(x, y) in &p[0] {
            let r = ((x - 12.0).powi(2) + (y - 12.0).powi(2)).sqrt();
            assert!((r - 10.0).abs() < 1e-3, "{x} {y}");
        }
        // Compact arc flags, and numbers run together.
        let p = path_parts("M0 0a1 1 0 011 1l.5.5-1-1");
        assert!(p[0].contains(&(1.0, 1.0)));
        assert_eq!(*p[0].last().unwrap(), (0.5, 0.5));
        // Nonsense ends the path, without a panic.
        for bad in ["", "X", "M", "M1", "L1 2", "M1 2 A", "M1 2c", "Z", "M1 2 a1 1 0 2 0 3 3", "M1e 2", "M 1e999 2"] {
            let _ = path_parts(bad);
        }
        // Every icon can be read.
        for (name, _) in ICONS {
            let s = icon_shapes(name).unwrap();
            assert!(s.iter().all(|s| !s.parts.is_empty()), "{name}");
        }
        assert!(icon_shapes("home").is_some());
        assert!(icon_shapes("nope").is_none());
    }

    #[test]
    fn paths_fuzzed() {
        // Bytes from a fixed seed: whatever path data comes in, reading it never panics.
        let mut seed = 12345u64;
        let alphabet = b"MmLlHhVvCcSsQqTtAaZz0123456789.-+e, ";
        for _ in 0..2000 {
            let len = (seed % 40) as usize;
            let mut s = String::new();
            for _ in 0..len {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                s.push(alphabet[((seed >> 33) % alphabet.len() as u64) as usize] as char);
            }
            let _ = path_parts(&s);
            let mut c = Canvas::new(48, 48, 2.0);
            paint_shapes(&mut c, &[Shape { filled: true, parts: path_parts(&s) }], |p| p, 1.0, 0);
        }
    }
}
