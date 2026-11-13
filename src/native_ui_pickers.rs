// Views, continued: date pickers and colour pickers, and the pop-ups they open.
//
// This is part of the same module as native_ui.rs (lust includes them all, and built programs get
// them one after the other), kept apart so no file grows too long. A picker's pop-up is a layer
// over the view, like a dropdown's list, opened, placed and closed the same way; what's inside it
// (a month's days, a square of colours) is drawn and clicked here.

// ----- dates -----
//
// Dates are text like "2026-11-13", as Time:date() gives them. Here they're counted as days since
// 1 January 1970, the same way native_rt.rs and runtime.lua count them.

/// Days since 1970-01-01 of a year, month (1 to 12) and day.
fn cal_days_from(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The year, month (1 to 12) and day of a day since 1970-01-01.
fn cal_civil(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// A date written like "2026-11-13" as days since 1970, if it's a real one (Time:isDate).
fn cal_day_of(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' || !b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()) {
        return None;
    }
    let (y, m, d): (i64, i64, i64) = (s[0..4].parse().ok()?, s[5..7].parse().ok()?, s[8..10].parse().ok()?);
    let z = cal_days_from(y, m, d);
    (cal_civil(z) == (y, m, d)).then_some(z)
}

/// A day since 1970 written as a date.
fn cal_text(z: i64) -> String {
    let (y, m, d) = cal_civil(z);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Which day of the week, from 0 for Monday to 6 for Sunday: weeks start on Monday.
fn cal_weekday(z: i64) -> i64 {
    (z + 3).rem_euclid(7)
}

/// How many days a month has.
fn cal_month_len(y: i64, m: i64) -> i64 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The same day `n` months later (earlier, below zero), or the month's last day if it's shorter.
fn cal_add_months(z: i64, n: i64) -> i64 {
    let (y, m, d) = cal_civil(z);
    let k = y * 12 + (m - 1) + n;
    let (ny, nm) = (k.div_euclid(12), k.rem_euclid(12) + 1);
    cal_days_from(ny, nm, d.min(cal_month_len(ny, nm)))
}

/// The first day shown in a month's grid of six weeks: the Monday on or before its first day.
fn cal_grid_start(z: i64) -> i64 {
    let (y, m, _) = cal_civil(z);
    let first = cal_days_from(y, m, 1);
    first - cal_weekday(first)
}

const MONTH_NAMES: [&str; 12] =
    ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAY_NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

thread_local! {
    /// Today, set by a test (`today 2026-11-13`) so it doesn't change from day to day.
    static TODAY: std::cell::Cell<Option<i64>> = const { std::cell::Cell::new(None) };
}

fn set_today(s: &str) {
    TODAY.with(|t| t.set(cal_day_of(s.trim())));
}

/// Today, for a date picker: a test's, or the one its program gave it, by the computer's clock.
fn today_of(n: &Laid) -> i64 {
    TODAY.with(|t| t.get()).or_else(|| cal_day_of(&n.key)).unwrap_or(0)
}

/// Whether a date picker can keep this text: no date (empty), or a real date between its min and max.
fn date_fits(t: &str, l: &Look) -> bool {
    t.is_empty() || cal_day_of(t).is_some_and(|d| d as f64 >= l.min && d as f64 <= l.max)
}

/// Whether a day can be chosen: between the min and max.
fn day_allowed(z: i64, l: &Look) -> bool {
    z as f64 >= l.min && z as f64 <= l.max
}

/// A day kept between a date picker's min and max.
fn day_clamped(z: i64, l: &Look) -> i64 {
    let mut z = z;
    if l.min.is_finite() && (z as f64) < l.min {
        z = l.min as i64;
    }
    if l.max.is_finite() && (z as f64) > l.max {
        z = l.max as i64;
    }
    z
}

// ----- colours -----

/// Hue (0 to 360), saturation and brightness (0 to 1) as red, green and blue (0 to 1).
fn hsv_to_rgb(h: f64, s: f64, v: f64) -> [f64; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let i = h.floor();
    let f = h - i;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    match i as i64 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

/// Red, green and blue (0 to 1) as hue (0 to 360), saturation and brightness (0 to 1). A grey has
/// no hue of its own, and black no saturation: those come back as 0.
fn rgb_to_hsv(r: f64, g: f64, b: f64) -> [f64; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let s = if max > 0.0 { d / max } else { 0.0 };
    let h = if d <= 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    [h, s, max]
}

/// A part as a byte, the way Lust's :hex() rounds it.
fn rgba_byte(x: f32) -> u32 {
    (x as f64 * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u32
}

/// A colour as hex text, as Lust's :hex() writes it: "#rrggbb", and "#rrggbbaa" when it isn't
/// solid (and `alpha` allows it).
fn rgba_hex(c: [f32; 4], alpha: bool) -> String {
    let mut s = format!("#{:02x}{:02x}{:02x}", rgba_byte(c[0]), rgba_byte(c[1]), rgba_byte(c[2]));
    if alpha && c[3] != 1.0 {
        s.push_str(&format!("{:02x}", rgba_byte(c[3])));
    }
    s
}

/// A colour to draw with.
fn rgba_u32(c: [f32; 4]) -> u32 {
    (rgba_byte(c[0]) << 16) | (rgba_byte(c[1]) << 8) | rgba_byte(c[2])
}

/// A colour from bytes, as Colour:hex and the colour words make it.
fn rgba_bytes(r: u32, g: u32, b: u32, a: u32) -> [f32; 4] {
    [(r as f64 / 255.0) as f32, (g as f64 / 255.0) as f32, (b as f64 / 255.0) as f32, (a as f64 / 255.0) as f32]
}

/// Hex typed in a colour picker: "#rrggbb", "#rgb", and with `alpha` "#rrggbbaa" and "#rgba",
/// with or without the #.
fn hex_parse(t: &str, alpha: bool) -> Option<[f32; 4]> {
    let t = t.trim();
    let h = t.strip_prefix('#').unwrap_or(t);
    if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let full: String = match h.len() {
        3 => h.chars().flat_map(|c| [c, c]).chain("ff".chars()).collect(),
        4 if alpha => h.chars().flat_map(|c| [c, c]).collect(),
        6 => format!("{h}ff"),
        8 if alpha => h.to_string(),
        _ => return None,
    };
    let part = |i: usize| u32::from_str_radix(&full[i..i + 2], 16).unwrap_or(0);
    Some(rgba_bytes(part(0), part(2), part(4), part(6)))
}

/// The colours a colour picker offers unless told otherwise: Lust's own colour words, at their
/// middle shade.
const SWATCH_WORDS: [&str; 12] = ["white", "gray5", "black", "red5", "orange5", "yellow5", "green5", "teal5", "blue5", "indigo5", "purple5", "pink5"];

fn default_swatches() -> Vec<[f32; 4]> {
    SWATCH_WORDS
        .iter()
        .map(|w| {
            let c = named(w).unwrap_or(0);
            rgba_bytes((c >> 16) & 255, (c >> 8) & 255, c & 255, 255)
        })
        .collect()
}

fn swatches_of(l: &Look) -> Vec<[f32; 4]> {
    l.swatches.clone().unwrap_or_else(default_swatches)
}

// ----- what an open picker is showing -----

/// The parts of an open colour picker, in the order Tab goes through them.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Part {
    Square,
    Hue,
    Alpha,
    Hex,
    Swatches,
}

/// What an open calendar shows: a month's days, a year's months, or a decade's years.
#[derive(Clone, Copy, Debug, PartialEq)]
enum CalView {
    Days,
    Months,
    Years,
}

#[derive(Clone, Debug)]
struct Picker {
    /// The date or colour picker that opened it.
    owner: Vec<usize>,
    colour: bool,
    /// A calendar: the day picked with the keyboard (its month shows), and what it shows.
    cursor: i64,
    view: CalView,
    /// A colour: its hue, saturation and brightness while the picker is open (so dragging the hue
    /// of a grey doesn't jump back), red, green, blue and see-through as they'll be written, the
    /// colour it had when it opened (Escape puts it back), and the one last written.
    hsv: [f64; 3],
    rgb: [f32; 3],
    a: f32,
    start: [f32; 4],
    last: [f32; 4],
    part: Part,
    /// The hex box: what's typed in it, and its caret.
    hex: String,
    caret: Caret,
    /// The swatch picked with the keyboard.
    swatch: usize,
}

/// How wide a date picker's calendar icon is, on its right.
const DATE_ICON_W: f32 = 30.0;

// The calendar's size and its parts' places, from its top left.
const CAL_PAD: f32 = 10.0;
const CAL_CELL_W: f32 = 34.0;
const CAL_CELL_H: f32 = 30.0;
const CAL_W: f32 = 2.0 * CAL_PAD + 7.0 * CAL_CELL_W;
const CAL_HEAD_H: f32 = 32.0;
const CAL_NAMES_H: f32 = 22.0;
const CAL_GRID_Y: f32 = CAL_PAD + CAL_HEAD_H + 4.0;
const CAL_GRID_H: f32 = CAL_NAMES_H + 6.0 * CAL_CELL_H;
const CAL_FOOT_Y: f32 = CAL_GRID_Y + CAL_GRID_H + 8.0;
const CAL_H: f32 = CAL_FOOT_Y + 30.0 + CAL_PAD;

// The colour pop-up's size and its parts' places.
const COL_PAD: f32 = 12.0;
const COL_IN: f32 = 216.0;
const COL_W: f32 = COL_IN + 2.0 * COL_PAD;
const COL_SQ_H: f32 = 150.0;
const COL_STRIP_H: f32 = 14.0;
const SWATCH: f32 = 20.0;
const SWATCHES_A_ROW: usize = 9;

/// A spot in an open calendar.
#[derive(Clone, Copy, Debug, PartialEq)]
enum CalSpot {
    Prev,
    Next,
    Title,
    Day(i64),
    /// A month of the cursor's year, 0 to 11, or a year.
    Month(i64),
    Year(i64),
    Today,
}

/// The calendar's header buttons: ‹, the title, and ›, as x, y, w, h from its top left.
fn cal_head_rects() -> [[f32; 4]; 3] {
    let s = CAL_HEAD_H;
    [[CAL_PAD, CAL_PAD, s, s], [CAL_PAD + s + 4.0, CAL_PAD, CAL_W - 2.0 * (CAL_PAD + s + 4.0), s], [CAL_W - CAL_PAD - s, CAL_PAD, s, s]]
}

/// A day's cell in the month's grid (0 to 41).
fn cal_day_rect(i: usize) -> [f32; 4] {
    let (col, row) = ((i % 7) as f32, (i / 7) as f32);
    [CAL_PAD + col * CAL_CELL_W, CAL_GRID_Y + CAL_NAMES_H + row * CAL_CELL_H, CAL_CELL_W, CAL_CELL_H]
}

/// A month's or year's cell (0 to 11), four across and three down.
fn cal_big_rect(i: usize) -> [f32; 4] {
    let (w, h) = ((CAL_W - 2.0 * CAL_PAD) / 4.0, CAL_GRID_H / 3.0);
    [CAL_PAD + (i % 4) as f32 * w, CAL_GRID_Y + (i / 4) as f32 * h, w, h]
}

fn cal_today_rect() -> [f32; 4] {
    [CAL_PAD, CAL_FOOT_Y, CAL_W - 2.0 * CAL_PAD, 30.0]
}

/// The first year in a calendar's grid of years: the year before the cursor's decade.
fn decade_start(z: i64) -> i64 {
    cal_civil(z).0.div_euclid(10) * 10 - 1
}

/// The calendar's title: "November 2026", "2026", or "2020–2029".
fn cal_title(p: &Picker) -> String {
    let (y, m, _) = cal_civil(p.cursor);
    match p.view {
        CalView::Days => format!("{} {y}", MONTH_NAMES[m as usize - 1]),
        CalView::Months => format!("{y}"),
        CalView::Years => {
            let d = decade_start(p.cursor) + 1;
            format!("{d}–{}", d + 9)
        }
    }
}

/// What's at a point in an open calendar placed at n.
fn cal_hit(n: &Laid, p: &Picker, x: f32, y: f32) -> Option<CalSpot> {
    let (x, y) = (x - n.x, y - n.y);
    let inside = |r: [f32; 4]| x >= r[0] && y >= r[1] && x < r[0] + r[2] && y < r[1] + r[3];
    let [prev, title, next] = cal_head_rects();
    if inside(prev) {
        return Some(CalSpot::Prev);
    }
    if inside(next) {
        return Some(CalSpot::Next);
    }
    if inside(title) {
        return Some(CalSpot::Title);
    }
    if inside(cal_today_rect()) {
        return Some(CalSpot::Today);
    }
    match p.view {
        CalView::Days => {
            let start = cal_grid_start(p.cursor);
            (0..42).find(|&i| inside(cal_day_rect(i))).map(|i| CalSpot::Day(start + i as i64))
        }
        CalView::Months => (0..12).find(|&i| inside(cal_big_rect(i))).map(|i| CalSpot::Month(i as i64)),
        CalView::Years => (0..12).find(|&i| inside(cal_big_rect(i))).map(|i| CalSpot::Year(decade_start(p.cursor) + i as i64)),
    }
}

/// Where a spot of an open calendar is, from its top left: x, y, w, h.
fn cal_spot_rect(p: &Picker, s: CalSpot) -> Option<[f32; 4]> {
    let [prev, title, next] = cal_head_rects();
    Some(match s {
        CalSpot::Prev => prev,
        CalSpot::Next => next,
        CalSpot::Title => title,
        CalSpot::Today => cal_today_rect(),
        CalSpot::Day(z) => {
            let i = z - cal_grid_start(p.cursor);
            if !(0..42).contains(&i) || p.view != CalView::Days {
                return None;
            }
            cal_day_rect(i as usize)
        }
        CalSpot::Month(m) if p.view == CalView::Months && (0..12).contains(&m) => cal_big_rect(m as usize),
        CalSpot::Year(y) if p.view == CalView::Years => {
            let i = y - decade_start(p.cursor);
            if !(0..12).contains(&i) {
                return None;
            }
            cal_big_rect(i as usize)
        }
        _ => return None,
    })
}

/// The colour pop-up's parts, from its top left: the square, the hue strip, the alpha strip (if
/// it has one), the hex box, and where the swatches start; and how tall it is.
fn col_rects(alpha: bool, swatches: usize) -> ([[f32; 4]; 5], f32) {
    let sq = [COL_PAD, COL_PAD, COL_IN, COL_SQ_H];
    let hue = [COL_PAD, sq[1] + COL_SQ_H + 12.0, COL_IN, COL_STRIP_H];
    let al = [COL_PAD, hue[1] + COL_STRIP_H + 10.0, COL_IN, if alpha { COL_STRIP_H } else { 0.0 }];
    let hex_y = if alpha { al[1] + COL_STRIP_H + 12.0 } else { hue[1] + COL_STRIP_H + 12.0 };
    let hex = [COL_PAD, hex_y, COL_IN, 30.0];
    let rows = swatches.div_ceil(SWATCHES_A_ROW).max(if swatches == 0 { 0 } else { 1 });
    let sw_h = rows as f32 * SWATCH + rows.saturating_sub(1) as f32 * 6.0;
    let sw = [COL_PAD, hex_y + 30.0 + 12.0, COL_IN, sw_h];
    let h = sw[1] + sw_h + if rows == 0 { 0.0 } else { COL_PAD };
    ([sq, hue, al, hex, sw], h.max(hex_y + 30.0 + COL_PAD))
}

/// A swatch's place, from the pop-up's top left.
fn swatch_rect(i: usize, sw: [f32; 4]) -> [f32; 4] {
    let gap = (COL_IN - SWATCHES_A_ROW as f32 * SWATCH) / (SWATCHES_A_ROW - 1) as f32;
    let (col, row) = ((i % SWATCHES_A_ROW) as f32, (i / SWATCHES_A_ROW) as f32);
    [sw[0] + col * (SWATCH + gap), sw[1] + row * (SWATCH + 6.0), SWATCH, SWATCH]
}

/// A spot in an open colour pop-up.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ColSpot {
    Part(Part),
    Swatch(usize),
}

fn col_hit(n: &Laid, x: f32, y: f32) -> Option<ColSpot> {
    let (x, y) = (x - n.x, y - n.y);
    let sw = swatches_of(&n.look);
    let (r, _) = col_rects(n.look.alpha, sw.len());
    let inside = |r: [f32; 4], grow: f32| x >= r[0] - grow && y >= r[1] - grow && x < r[0] + r[2] + grow && y < r[1] + r[3] + grow;
    if inside(r[0], 0.0) {
        return Some(ColSpot::Part(Part::Square));
    }
    // The strips are thin, so they can be grabbed a little above and below.
    if inside(r[1], 5.0) {
        return Some(ColSpot::Part(Part::Hue));
    }
    if n.look.alpha && inside(r[2], 5.0) {
        return Some(ColSpot::Part(Part::Alpha));
    }
    if inside(r[3], 0.0) {
        return Some(ColSpot::Part(Part::Hex));
    }
    (0..sw.len()).find(|&i| inside(swatch_rect(i, r[4]), 0.0)).map(ColSpot::Swatch)
}

// ----- laying out -----

/// A date or colour picker, closed. A date picker keeps today's date (from its program) in its
/// key, and its field's own date in its text (what's typed in it can differ); a colour picker
/// shows its colour's hex, and its edge darkens under the mouse.
fn picker_lay(n: &Node, laid: &mut Laid) {
    if n.kind == "datePicker" {
        laid.key = n.text.clone();
        laid.text = match &n.value {
            Some(SV::Text(t)) => t.clone(),
            _ => String::new(),
        };
        return;
    }
    let c = match n.value {
        Some(SV::Rgba(c)) => c,
        _ => [0.0, 0.0, 0.0, 1.0],
    };
    laid.text = rgba_hex(c, true);
    if laid.action.is_some() {
        let mut over = laid.look.clone();
        over.border = Some(nt(0x868e96));
        laid.hover = Some(over);
    }
}

/// How wide a colour picker's words are: room for the longest hex it can show, so it doesn't
/// change size as the colour changes.
fn picker_text_w(n: &Laid) -> f32 {
    let l = &n.look;
    let widest = text_width("#dddddd", l.size, l.face()).max(text_width(&n.text, l.size, l.face()));
    if l.alpha {
        widest.max(text_width("#dddddddd", l.size, l.face()))
    } else {
        widest
    }
    .ceil()
}

/// A picker's pop-up, placed below its owner (above, near the window's bottom), like a dropdown's list.
fn picker_box(owner: &Laid, p: &Picker, w: f32, h: f32) -> Laid {
    let mut b = lay(&Node::new(if p.colour { "colourpop" } else { "calendar" }));
    b.look.background = Some(nt(0xffffff));
    b.look.border = Some(nt(0xdee2e6));
    b.look.radius = 8.0;
    b.look.shadow = true;
    b.look.accent = owner.look.accent;
    b.look.min = owner.look.min;
    b.look.max = owner.look.max;
    b.look.alpha = owner.look.alpha;
    b.look.swatches = owner.look.swatches.clone();
    b.action = owner.action;
    b.key = owner.key.clone();
    // A calendar shows the field's own date, not what's typed in the box.
    b.value = if p.colour { owner.value.clone() } else { Some(SV::Text(owner.text.clone())) };
    let (bw, bh) = if p.colour {
        (COL_W, col_rects(owner.look.alpha, swatches_of(&owner.look).len()).1)
    } else {
        (CAL_W, CAL_H)
    };
    b.text = if p.colour { rgba_hex([p.rgb[0], p.rgb[1], p.rgb[2], p.a], owner.look.alpha) } else { cal_title(p) };
    let below = h - (owner.y + owner.h + 4.0) - 8.0;
    let above = owner.y - 12.0;
    // Below, or above when there's more room there; moved up to fit inside the window if it can.
    let y = if bh <= below || below >= above { owner.y + owner.h + 4.0 } else { owner.y - 4.0 - bh };
    let y = y.min(h - 8.0 - bh).max(8.0);
    let x = owner.x.min(w - 8.0 - bw).max(8.0);
    b.x = x.round();
    b.y = y.round();
    b.w = bw;
    b.h = bh;
    b
}

// ----- painting -----

/// Paints a picker's own parts, after its background and edge: a date picker's text and icon, a
/// colour picker's swatch and hex, and what's inside an open calendar or colour pop-up.
fn paint_picker(c: &mut Canvas, n: &Laid, l: &Look, focused: bool, st: &PaintState) {
    match n.kind.as_str() {
        "datePicker" => {
            paint_input(c, n, focused, st);
            if n.bad {
                c.shape(n.x, n.y, n.w, n.h, l.radius, Some(1.5), 0xff8787, 1.0);
            }
            let size = 16.0;
            let ix = n.x + n.w - DATE_ICON_W + (DATE_ICON_W - size) / 2.0 - 2.0;
            paint_icon(c, "calendar", ix, n.y + (n.h - size) / 2.0, size, mix(l.color, nt(0xffffff), 0.3));
        }
        "colourPicker" => {
            let col = match n.value {
                Some(SV::Rgba(c)) => c,
                _ => [0.0, 0.0, 0.0, 1.0],
            };
            let s = 20.0f32.min(n.h - 8.0).max(8.0);
            let (sx, sy) = (n.x + 9.0, n.y + ((n.h - s) / 2.0).round());
            paint_swatch(c, [sx, sy, s, s], col, 4.0);
            let (_, lh) = line_metrics(l.size, l.face());
            c.text(&n.text, n.x + l.pad[3], n.y + ((n.h - lh) / 2.0).round(), l.size, l.face(), l.color);
        }
        "calendar" => {
            if let Some(p) = st.picker.as_ref().filter(|p| !p.colour) {
                paint_calendar(c, n, p, st);
            }
        }
        "colourpop" => {
            if let Some(p) = st.picker.as_ref().filter(|p| p.colour) {
                paint_colourpop(c, n, p, st);
            }
        }
        _ => {}
    }
}

/// A colour in a rounded box, with a checkerboard behind it when it's see-through, and a thin edge.
fn paint_swatch(c: &mut Canvas, r: [f32; 4], col: [f32; 4], radius: f32) {
    if col[3] < 1.0 {
        paint_checker(c, r, radius);
    }
    c.rect(r[0], r[1], r[2], r[3], radius, rgba_u32(col), col[3]);
    c.shape(r[0], r[1], r[2], r[3], radius, Some(1.0), 0x000000, 0.15);
}

/// Grey and white squares, to show see-through colours against.
fn paint_checker(c: &mut Canvas, r: [f32; 4], radius: f32) {
    c.rect(r[0], r[1], r[2], r[3], radius, 0xffffff, 1.0);
    let was = c.clip_to(r[0], r[1], r[2], r[3]);
    let k = 5.0;
    let (cols, rows) = ((r[2] / k).ceil() as i64, (r[3] / k).ceil() as i64);
    for j in 0..rows {
        for i in 0..cols {
            if (i + j) % 2 == 1 {
                c.rect(r[0] + i as f32 * k, r[1] + j as f32 * k, k, k, 0.0, 0xd0d4d8, 1.0);
            }
        }
    }
    c.clip = was;
}

/// Text in the middle of a box.
fn text_centred(c: &mut Canvas, t: &str, r: [f32; 4], size: f32, face: usize, col: u32) {
    let (_, lh) = line_metrics(size, face);
    let tw = text_width(t, size, face);
    c.text(t, (r[0] + (r[2] - tw) / 2.0).round(), (r[1] + (r[3] - lh) / 2.0).round(), size, face, col);
}

fn paint_calendar(c: &mut Canvas, n: &Laid, p: &Picker, st: &PaintState) {
    let l = &n.look;
    let ink = nt(INK);
    let bg = nt(0xffffff);
    let soft = mix(ink, bg, 0.6);
    let face = l.face();
    let bold = l.font.unwrap_or_else(|| weight_face(600.0));
    let at = |r: [f32; 4]| [n.x + r[0], n.y + r[1], r[2], r[3]];
    let hover = st.hover.and_then(|(x, y)| cal_hit(n, p, x, y));
    let hover_bg = nt(0xf1f3f5);
    let cursor_bg = nt(0xe7f5ff);
    let [prev, title, next] = cal_head_rects();
    for (r, spot) in [(prev, CalSpot::Prev), (title, CalSpot::Title), (next, CalSpot::Next)] {
        let r = at(r);
        if hover == Some(spot) && !(spot == CalSpot::Title && p.view == CalView::Years) {
            c.rect(r[0], r[1], r[2], r[3], 6.0, hover_bg, 1.0);
        }
    }
    let ip = at(prev);
    paint_icon(c, "chevron-left", ip[0] + 8.0, ip[1] + 8.0, 16.0, ink);
    let inx = at(next);
    paint_icon(c, "chevron-right", inx[0] + 8.0, inx[1] + 8.0, 16.0, ink);
    text_centred(c, &cal_title(p), at(title), 14.0, bold, ink);
    let chosen = match &n.value {
        Some(SV::Text(t)) => cal_day_of(t),
        _ => None,
    };
    let today = today_of(n);
    let accent = l.accent;
    match p.view {
        CalView::Days => {
            for (i, d) in DAY_NAMES.iter().enumerate() {
                let r = at([CAL_PAD + i as f32 * CAL_CELL_W, CAL_GRID_Y, CAL_CELL_W, CAL_NAMES_H]);
                text_centred(c, d, r, 12.0, face, nt(MUTED));
            }
            let start = cal_grid_start(p.cursor);
            let (_, month, _) = cal_civil(p.cursor);
            for i in 0..42 {
                let z = start + i as i64;
                let r = at(cal_day_rect(i));
                let (cx, cy) = (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0);
                let d = 26.0;
                let dot = [cx - d / 2.0, cy - d / 2.0, d, d];
                let (_, m, day) = cal_civil(z);
                let allowed = day_allowed(z, l);
                let mut col = if m == month { ink } else { soft };
                if !allowed {
                    col = mix(ink, bg, 0.75);
                }
                if Some(z) == chosen {
                    c.rect(dot[0], dot[1], d, d, d / 2.0, accent, 1.0);
                    col = 0xffffff;
                } else if z == p.cursor {
                    c.rect(dot[0], dot[1], d, d, d / 2.0, cursor_bg, 1.0);
                } else if hover == Some(CalSpot::Day(z)) && allowed {
                    c.rect(dot[0], dot[1], d, d, d / 2.0, hover_bg, 1.0);
                }
                if z == today {
                    let ring = if Some(z) == chosen { mix(accent, 0x000000, 0.35) } else { accent };
                    c.shape(dot[0], dot[1], d, d, d / 2.0, Some(1.5), ring, 1.0);
                }
                // The day the keyboard is on gets a ring of its own when it's also the one chosen.
                if z == p.cursor && Some(z) == chosen {
                    c.shape(dot[0] - 2.0, dot[1] - 2.0, d + 4.0, d + 4.0, d / 2.0 + 2.0, Some(1.5), mix(accent, bg, 0.5), 1.0);
                }
                text_centred(c, &day.to_string(), r, 13.0, if z == today { bold } else { face }, col);
            }
        }
        CalView::Months | CalView::Years => {
            let (cy, cm, _) = cal_civil(p.cursor);
            let (ty, tm, _) = cal_civil(today);
            let picked = chosen.map(cal_civil);
            for i in 0..12usize {
                let r = at(cal_big_rect(i));
                let cell = [r[0] + 3.0, r[1] + (r[3] - 34.0) / 2.0, r[2] - 6.0, 34.0];
                let (label, here, now, chose, outer) = if p.view == CalView::Months {
                    let m = i as i64 + 1;
                    (MONTH_NAMES[i][..3].to_string(), m == cm, ty == cy && tm == m, picked.is_some_and(|(y, mm, _)| y == cy && mm == m), false)
                } else {
                    let y = decade_start(p.cursor) + i as i64;
                    (y.to_string(), y == cy, y == ty, picked.is_some_and(|(py, _, _)| py == y), i == 0 || i == 11)
                };
                let spot = if p.view == CalView::Months { CalSpot::Month(i as i64) } else { CalSpot::Year(decade_start(p.cursor) + i as i64) };
                let mut col = if outer { soft } else { ink };
                if chose {
                    c.rect(cell[0], cell[1], cell[2], cell[3], 8.0, accent, 1.0);
                    col = 0xffffff;
                } else if here {
                    c.rect(cell[0], cell[1], cell[2], cell[3], 8.0, cursor_bg, 1.0);
                } else if hover == Some(spot) {
                    c.rect(cell[0], cell[1], cell[2], cell[3], 8.0, hover_bg, 1.0);
                }
                if now {
                    c.shape(cell[0], cell[1], cell[2], cell[3], 8.0, Some(1.5), accent, 1.0);
                }
                if here && chose {
                    c.shape(cell[0] - 2.0, cell[1] - 2.0, cell[2] + 4.0, cell[3] + 4.0, 10.0, Some(1.5), mix(accent, bg, 0.5), 1.0);
                }
                text_centred(c, &label, cell, 13.0, face, col);
            }
        }
    }
    // A line, and Today under it.
    c.rect(n.x + CAL_PAD, n.y + CAL_FOOT_Y - 5.0, CAL_W - 2.0 * CAL_PAD, 1.0, 0.0, nt(0xe9ecef), 1.0);
    let tr = at(cal_today_rect());
    let can = day_allowed(today, l);
    if hover == Some(CalSpot::Today) && can {
        c.rect(tr[0], tr[1], tr[2], tr[3], 6.0, hover_bg, 1.0);
    }
    text_centred(c, "Today", tr, 13.0, bold, if can { accent } else { mix(ink, bg, 0.75) });
}

fn paint_colourpop(c: &mut Canvas, n: &Laid, p: &Picker, st: &PaintState) {
    let l = &n.look;
    let sw = swatches_of(l);
    let (r, _) = col_rects(l.alpha, sw.len());
    let at = |r: [f32; 4]| [n.x + r[0], n.y + r[1], r[2], r[3]];
    let ring = mix(l.accent, nt(0xffffff), 0.5);
    let [h, s, v] = p.hsv;
    // The square: saturation from left to right, brightness from bottom to top, at this hue.
    let sq = at(r[0]);
    paint_gradient(c, sq, |fx, fy| {
        let [r, g, b] = hsv_to_rgb(h, fx as f64, 1.0 - fy as f64);
        (rgba_u32([r as f32, g as f32, b as f32, 1.0]), 1.0)
    });
    c.shape(sq[0], sq[1], sq[2], sq[3], 4.0, Some(1.0), 0x000000, 0.12);
    let (mx, my) = (sq[0] + s as f32 * sq[2], sq[1] + (1.0 - v as f32) * sq[3]);
    c.shape(mx - 7.0, my - 7.0, 14.0, 14.0, 7.0, Some(3.0), 0x000000, 0.35);
    c.shape(mx - 6.0, my - 6.0, 12.0, 12.0, 6.0, Some(2.0), 0xffffff, 1.0);
    // The hue strip, and the alpha strip.
    let hue = at(r[1]);
    paint_gradient(c, hue, |fx, _| {
        let [r, g, b] = hsv_to_rgb(fx as f64 * 360.0, 1.0, 1.0);
        (rgba_u32([r as f32, g as f32, b as f32, 1.0]), 1.0)
    });
    strip_knob(c, hue, (h / 360.0) as f32);
    if l.alpha {
        let al = at(r[2]);
        paint_checker(c, al, 0.0);
        let solid = rgba_u32([p.rgb[0], p.rgb[1], p.rgb[2], 1.0]);
        paint_gradient(c, al, |fx, _| (solid, fx));
        strip_knob(c, al, p.a);
    }
    // The hex box.
    let hx = at(r[3]);
    c.rect(hx[0], hx[1], hx[2], hx[3], 6.0, nt(0xffffff), 1.0);
    let bad = hex_parse(&p.hex, l.alpha).is_none();
    let edge = if bad {
        0xff8787
    } else if p.part == Part::Hex {
        l.accent
    } else {
        nt(0xced4da)
    };
    c.shape(hx[0], hx[1], hx[2], hx[3], 6.0, Some(if p.part == Part::Hex || bad { 1.5 } else { 1.0 }), edge, 1.0);
    let face = l.face();
    let (_, lh) = line_metrics(14.0, face);
    let (tx, ty) = (hx[0] + 10.0, hx[1] + ((hx[3] - lh) / 2.0).round());
    // While an input method composes in the hex box, its text shows at the caret, underlined.
    let composing = st.preedit.as_ref().filter(|_| p.part == Part::Hex);
    let (hex, caret, pre) = match composing {
        Some((pre, cursor)) => {
            let (shown, a, b, at) = with_preedit(&p.hex, &p.caret, pre, *cursor);
            (shown, Caret::at(at), Some((a, b)))
        }
        None => (p.hex.clone(), p.caret.clone(), None),
    };
    if p.part == Part::Hex {
        let (a, b) = caret.range();
        let (a, b) = (a.min(hex.len()), b.min(hex.len()));
        if a != b {
            let x0 = tx + text_width(&hex[..a], 14.0, face);
            let x1 = tx + text_width(&hex[..b], 14.0, face);
            c.rect(x0, ty, x1 - x0, lh, 0.0, if dark() { 0x1f4f7d } else { 0xb4d5fe }, 1.0);
        }
        if st.caret_on || a != b {
            let x = tx + text_width(&hex[..caret.caret.min(hex.len())], 14.0, face);
            c.rect(x - 0.5, ty + 1.0, 1.5, lh - 2.0, 0.0, nt(INK), 1.0);
        }
    }
    c.text(&hex, tx, ty, 14.0, face, nt(INK));
    if let Some((a, b)) = pre {
        preedit_line(c, tx + text_width(&hex[..a], 14.0, face), tx + text_width(&hex[..b], 14.0, face), ty + lh, nt(INK));
    }
    // A little swatch of the colour as it is now, on the right of the hex box.
    let now = [p.rgb[0], p.rgb[1], p.rgb[2], if l.alpha { p.a } else { 1.0 }];
    paint_swatch(c, [hx[0] + hx[2] - 26.0, hx[1] + 5.0, 20.0, 20.0], now, 4.0);
    // The swatches.
    let hover = st.hover.and_then(|(x, y)| col_hit(n, x, y));
    for (i, col) in sw.iter().enumerate() {
        let sr = at(swatch_rect(i, r[4]));
        if hover == Some(ColSpot::Swatch(i)) {
            c.rect(sr[0] - 2.0, sr[1] - 2.0, sr[2] + 4.0, sr[3] + 4.0, 6.0, nt(0xe9ecef), 1.0);
        }
        let shown = if l.alpha { *col } else { [col[0], col[1], col[2], 1.0] };
        paint_swatch(c, sr, shown, 4.0);
        if shown == now {
            c.shape(sr[0] - 3.0, sr[1] - 3.0, sr[2] + 6.0, sr[3] + 6.0, 7.0, Some(1.5), nt(0x868e96), 1.0);
        }
        if p.part == Part::Swatches && p.swatch == i {
            c.shape(sr[0] - 4.0, sr[1] - 4.0, sr[2] + 8.0, sr[3] + 8.0, 8.0, Some(2.0), l.accent, 1.0);
        }
    }
    // A ring round the part the keys move.
    let part_rect = match p.part {
        Part::Square => Some(sq),
        Part::Hue => Some(hue),
        Part::Alpha if l.alpha => Some(at(r[2])),
        _ => None,
    };
    if let Some(pr) = part_rect {
        c.shape(pr[0] - RING, pr[1] - RING, pr[2] + 2.0 * RING, pr[3] + 2.0 * RING, 4.0 + RING, Some(2.0), ring, 1.0);
    }
}

/// Fills a box pixel by pixel: `at` gives the colour and how solid it is at a place across and
/// down it, each from 0 to 1.
fn paint_gradient(c: &mut Canvas, r: [f32; 4], at: impl Fn(f32, f32) -> (u32, f32)) {
    let k = c.scale;
    let [bx0, by0, bx1, by1] = c.bounds();
    let (x0, y0) = ((r[0] * k).round() as i64, (r[1] * k).round() as i64);
    let (x1, y1) = (((r[0] + r[2]) * k).round() as i64, ((r[1] + r[3]) * k).round() as i64);
    let (w, h) = ((x1 - x0).max(1) as f32, (y1 - y0).max(1) as f32);
    for py in y0.max(by0)..y1.min(by1) {
        let fy = ((py - y0) as f32 + 0.5) / h;
        for px in x0.max(bx0)..x1.min(bx1) {
            let fx = ((px - x0) as f32 + 0.5) / w;
            let (col, a) = at(fx, fy);
            c.blend(px, py, col, a);
        }
    }
}

/// The knob on a strip: a white bar with a dark edge, at a place from 0 to 1 along it.
fn strip_knob(c: &mut Canvas, r: [f32; 4], f: f32) {
    let x = r[0] + f.clamp(0.0, 1.0) * r[2];
    c.rect(x - 4.0, r[1] - 3.0, 8.0, r[3] + 6.0, 3.0, 0x000000, 0.3);
    c.rect(x - 3.0, r[1] - 2.0, 6.0, r[3] + 4.0, 2.0, 0xffffff, 1.0);
}

// ----- test mode -----

/// What `dump` adds for a picker: whether it's open, and for an open calendar or colour
/// pop-up what it shows.
fn picker_dump(n: &Laid, path: &[usize], st: &PaintState) -> String {
    let Some(p) = &st.picker else { return String::new() };
    match n.kind.as_str() {
        "datePicker" | "colourPicker" if p.owner == path => {
            if p.colour {
                " (open)".to_string()
            } else {
                format!(" (open {})", cal_title(p))
            }
        }
        "calendar" => {
            let view = match p.view {
                CalView::Days => "",
                CalView::Months => ", months",
                CalView::Years => ", years",
            };
            format!(" (cursor {}, today {}{view})", cal_text(p.cursor), cal_text(today_of(n)))
        }
        "colourpop" => {
            let part = match p.part {
                Part::Square => "square",
                Part::Hue => "hue",
                Part::Alpha => "alpha",
                Part::Hex => "hex",
                Part::Swatches => "swatches",
            };
            let mut s = format!(
                " (hue {}, saturation {}%, brightness {}%",
                p.hsv[0].round() as i64,
                (p.hsv[1] * 100.0).round() as i64,
                (p.hsv[2] * 100.0).round() as i64
            );
            if n.look.alpha {
                s.push_str(&format!(", alpha {}%", (p.a as f64 * 100.0).round() as i64));
            }
            if p.part == Part::Hex {
                s.push_str(&format!(", typed {:?}", p.hex));
                if let Some((pre, _)) = &st.preedit {
                    s.push_str(&format!(", composing {pre:?}"));
                }
            }
            s.push_str(&format!(", at {part})"));
            s
        }
        _ => String::new(),
    }
}

// ----- what happens -----

impl<'a, 'b> Ui<'a, 'b> {
    /// The open picker's pop-up, as a layer over the view. Its owner's colour is followed when the
    /// program changes it while the picker is open.
    fn picker_layer(&mut self, tree: &Laid) -> Option<Laid> {
        let (w, h) = (self.w, self.h);
        let p = self.picker.as_mut()?;
        let want = if p.colour { "colourPicker" } else { "datePicker" };
        let Some(owner) = node_at(tree, &p.owner).filter(|n| n.kind == want && n.action.is_some()) else {
            self.picker = None;
            return None;
        };
        if p.colour {
            if let Some(SV::Rgba(now)) = owner.value {
                if now != p.last {
                    p.take_colour(now, owner.look.alpha);
                }
            }
        }
        let b = picker_box(owner, p, w, h);
        Some(layer("picker", w, h, b))
    }

    /// Opens a date or colour picker's pop-up, or closes it if it's open.
    fn open_picker(&mut self, path: &[usize]) {
        if self.picker.as_ref().is_some_and(|p| p.owner == path) {
            self.close_picker();
            return;
        }
        let Some(n) = node_at(&self.tree, path) else { return };
        if n.action.is_none() {
            return;
        }
        let colour = n.kind == "colourPicker";
        let mut p = Picker {
            owner: path.to_vec(),
            colour,
            cursor: 0,
            view: CalView::Days,
            hsv: [0.0, 0.0, 0.0],
            rgb: [0.0; 3],
            a: 1.0,
            start: [0.0, 0.0, 0.0, 1.0],
            last: [0.0, 0.0, 0.0, 1.0],
            part: Part::Square,
            hex: String::new(),
            caret: Caret::default(),
            swatch: 0,
        };
        if colour {
            let c = match n.value {
                Some(SV::Rgba(c)) => c,
                _ => [0.0, 0.0, 0.0, 1.0],
            };
            p.start = c;
            p.take_colour(c, n.look.alpha);
            let sw = swatches_of(&n.look);
            p.swatch = sw.iter().position(|s| *s == c).unwrap_or(0);
        } else {
            // The field's date, or today; kept between min and max.
            let day = cal_day_of(value_text(n)).or_else(|| cal_day_of(&n.text)).unwrap_or_else(|| today_of(n));
            p.cursor = day_clamped(day, &n.look);
        }
        self.popup = None;
        self.picker = Some(p);
        self.relayout();
    }

    /// Closes the open picker, keeping what it chose.
    fn close_picker(&mut self) {
        if self.picker.take().is_some() {
            self.relayout();
        }
    }

    /// The open picker's pop-up, and its place in the tree.
    fn picker_node(&self) -> Option<(Vec<usize>, &Laid)> {
        let k = self.tree.children.iter().rposition(|c| c.kind == "layer" && c.text == "picker")?;
        Some((vec![k, 0], self.tree.children[k].children.first()?))
    }

    /// Whether this point is inside the open picker's pop-up.
    fn in_picker(&self, x: f32, y: f32) -> bool {
        self.picker_node().is_some_and(|(_, b)| x >= b.x && y >= b.y && x < b.x + b.w && y < b.y + b.h)
    }

    /// Whether typing goes into the open picker: its hex box.
    fn picker_takes_text(&self) -> bool {
        self.picker.as_ref().is_some_and(|p| p.colour && p.part == Part::Hex)
    }

    /// The mouse went down while a picker is open: outside it, it closes (keeping what it chose)
    /// and that's all; inside, a calendar acts when it's let go, and a colour pop-up's square and
    /// strips follow the mouse straight away.
    fn picker_press(&mut self, x: f32, y: f32) -> Result<(), String> {
        let Some((path, b)) = self.picker_node().map(|(p, b)| (p, b.clone())) else {
            self.picker = None;
            return Ok(());
        };
        if !(x >= b.x && y >= b.y && x < b.x + b.w && y < b.y + b.h) {
            self.pressed = None;
            self.close_picker();
            return Ok(());
        }
        self.pressed = Some(path);
        let colour = self.picker.as_ref().is_some_and(|p| p.colour);
        if !colour {
            return Ok(());
        }
        match col_hit(&b, x, y) {
            Some(ColSpot::Part(part @ (Part::Square | Part::Hue | Part::Alpha))) => {
                if let Some(p) = self.picker.as_mut() {
                    p.part = part;
                }
                self.drag = Drag::Picker(part);
                self.picker_drag(part, x, y)?;
            }
            Some(ColSpot::Part(Part::Hex)) => {
                let (r, _) = col_rects(b.look.alpha, swatches_of(&b.look).len());
                if let Some(p) = self.picker.as_mut() {
                    let i = place_at(&p.hex, x - (b.x + r[3][0] + 10.0), 14.0, b.look.face());
                    p.part = Part::Hex;
                    p.caret = Caret::at(i);
                }
                self.wake();
                self.relayout();
            }
            _ => {}
        }
        Ok(())
    }

    /// The mouse moved with the button held in a colour pop-up's square or strip.
    fn picker_drag(&mut self, part: Part, x: f32, y: f32) -> Result<(), String> {
        let Some((_, b)) = self.picker_node() else { return Ok(()) };
        let (r, _) = col_rects(b.look.alpha, swatches_of(&b.look).len());
        let (bx, by) = (b.x, b.y);
        let alpha = b.look.alpha;
        let frac = |rr: [f32; 4]| (((x - bx - rr[0]) / rr[2]).clamp(0.0, 1.0) as f64, ((y - by - rr[1]) / rr[3]).clamp(0.0, 1.0) as f64);
        let Some(p) = self.picker.as_mut() else { return Ok(()) };
        match part {
            Part::Square => {
                let (fx, fy) = frac(r[0]);
                p.hsv[1] = fx;
                p.hsv[2] = 1.0 - fy;
                p.from_hsv(alpha);
            }
            Part::Hue => {
                // The far right end is 360°, the same red as 0°, kept just under so the knob stays there.
                p.hsv[0] = (frac(r[1]).0 * 360.0).min(359.999);
                p.from_hsv(alpha);
            }
            Part::Alpha if alpha => {
                p.a = frac(r[2]).0 as f32;
                p.hex = rgba_hex([p.rgb[0], p.rgb[1], p.rgb[2], p.a], true);
            }
            _ => return Ok(()),
        }
        self.write_colour()
    }

    /// The mouse was let go in an open picker, where it went down.
    fn picker_release(&mut self, x: f32, y: f32) -> Result<(), String> {
        let Some((_, b)) = self.picker_node().map(|(p, b)| (p, b.clone())) else { return Ok(()) };
        let Some(p) = self.picker.clone() else { return Ok(()) };
        if p.colour {
            if let Some(ColSpot::Swatch(i)) = col_hit(&b, x, y) {
                return self.pick_swatch(i);
            }
            return Ok(());
        }
        let Some(spot) = cal_hit(&b, &p, x, y) else { return Ok(()) };
        let l = b.look.clone();
        match spot {
            CalSpot::Day(z) => {
                if day_allowed(z, &l) {
                    return self.choose_day(z);
                }
                Ok(())
            }
            CalSpot::Today => {
                let t = today_of(&b);
                if day_allowed(t, &l) {
                    return self.choose_day(t);
                }
                Ok(())
            }
            CalSpot::Prev | CalSpot::Next => {
                let sign = if spot == CalSpot::Prev { -1 } else { 1 };
                let months = match p.view {
                    CalView::Days => 1,
                    CalView::Months => 12,
                    CalView::Years => 120,
                };
                self.move_cursor(cal_add_months(p.cursor, sign * months));
                Ok(())
            }
            CalSpot::Title => {
                if let Some(q) = self.picker.as_mut() {
                    q.view = match q.view {
                        CalView::Days => CalView::Months,
                        _ => CalView::Years,
                    };
                }
                self.relayout();
                Ok(())
            }
            CalSpot::Month(m) => {
                let (y, _, d) = cal_civil(p.cursor);
                let z = cal_days_from(y, m + 1, d.min(cal_month_len(y, m + 1)));
                if let Some(q) = self.picker.as_mut() {
                    q.view = CalView::Days;
                }
                self.move_cursor(z);
                Ok(())
            }
            CalSpot::Year(y) => {
                let (_, m, d) = cal_civil(p.cursor);
                let z = cal_days_from(y, m, d.min(cal_month_len(y, m)));
                if let Some(q) = self.picker.as_mut() {
                    q.view = CalView::Months;
                }
                self.move_cursor(z);
                Ok(())
            }
        }
    }

    /// Moves the calendar's cursor (and the month it shows), kept between min and max.
    fn move_cursor(&mut self, z: i64) {
        let Some(l) = self.picker_node().map(|(_, b)| b.look.clone()) else { return };
        if let Some(p) = self.picker.as_mut() {
            p.cursor = day_clamped(z, &l);
        }
        self.relayout();
    }

    /// A day was chosen: the calendar closes and the date picker's field gets it.
    fn choose_day(&mut self, z: i64) -> Result<(), String> {
        let Some(p) = self.picker.take() else { return Ok(()) };
        let owner = p.owner.clone();
        if self.draft.as_ref().is_some_and(|(d, _)| *d == owner) {
            self.draft = None;
        }
        let text = cal_text(z);
        let now = node_at(&self.tree, &owner).map(|n| value_text(n).to_string());
        if now.as_deref() != Some(text.as_str()) {
            self.run(&owner, SV::Text(text))?;
        } else {
            self.relayout();
        }
        if self.st.focus.as_ref() == Some(&owner) {
            let end = self.focused().map(|n| value_text(n).len()).unwrap_or(0);
            self.st.caret = Caret::at(end);
        }
        Ok(())
    }

    /// Writes the open colour picker's colour to its field, if it changed, and runs its block.
    fn write_colour(&mut self) -> Result<(), String> {
        let Some(p) = self.picker.as_mut() else { return Ok(()) };
        let alpha = node_at(&self.tree, &p.owner).is_some_and(|n| n.look.alpha);
        let c = [p.rgb[0], p.rgb[1], p.rgb[2], if alpha { p.a } else { 1.0 }];
        let owner = p.owner.clone();
        let now = node_at(&self.tree, &owner).and_then(|n| match n.value {
            Some(SV::Rgba(c)) => Some(c),
            _ => None,
        });
        if now == Some(c) {
            self.relayout();
            return Ok(());
        }
        p.last = c;
        self.run(&owner, SV::Rgba(c))
    }

    /// A swatch was clicked or picked with the keys: the colour becomes it.
    fn pick_swatch(&mut self, i: usize) -> Result<(), String> {
        let Some((_, b)) = self.picker_node() else { return Ok(()) };
        let alpha = b.look.alpha;
        let Some(c) = swatches_of(&b.look).get(i).copied() else { return Ok(()) };
        if let Some(p) = self.picker.as_mut() {
            p.swatch = i;
            p.part = Part::Swatches;
            p.take_colour(c, alpha);
        }
        self.write_colour()
    }

    /// Typing while a picker is open: into a colour pop-up's hex box, when it has the keys.
    fn picker_edit(&mut self, k: Key, mods: Mods) -> Result<(), String> {
        let Some(p) = self.picker.as_mut() else { return Ok(()) };
        if !(p.colour && p.part == Part::Hex) {
            return Ok(());
        }
        self.wake();
        let Some(p) = self.picker.as_mut() else { return Ok(()) };
        let old = p.hex.clone();
        match p.caret.key(&old, &k, mods.shift, mods.word) {
            Some(new) => self.set_hex(new),
            None => {
                self.relayout();
                Ok(())
            }
        }
    }

    /// New text in the hex box: when it's a colour, the colour becomes it.
    fn set_hex(&mut self, text: String) -> Result<(), String> {
        let Some((_, b)) = self.picker_node() else { return Ok(()) };
        let alpha = b.look.alpha;
        let Some(p) = self.picker.as_mut() else { return Ok(()) };
        p.hex = text.chars().take(12).collect();
        p.caret.fit(&p.hex);
        match hex_parse(&p.hex, alpha) {
            Some(c) => {
                let keep = p.hex.clone();
                let caret = p.caret.clone();
                p.take_colour(c, alpha);
                p.hex = keep;
                p.caret = caret;
                self.write_colour()
            }
            None => {
                self.relayout();
                Ok(())
            }
        }
    }

    /// Keys while a picker is open. Returns whether the key was used.
    fn picker_key(&mut self, k: &Nav, mods: Mods) -> Result<bool, String> {
        let Some(p) = self.picker.clone() else { return Ok(false) };
        if p.colour {
            return self.colour_key(k, mods, p);
        }
        let Some(l) = self.picker_node().map(|(_, b)| b.look.clone()) else { return Ok(false) };
        let z = p.cursor;
        match (k, p.view) {
            (Nav::Escape, _) => self.close_picker(),
            // Alt+Up closes it, as Alt+Down opened it.
            (Nav::Up, _) if mods.alt => self.close_picker(),
            (Nav::Tab, _) => {
                self.close_picker();
                return Ok(false);
            }
            (Nav::Enter, CalView::Days) => {
                if day_allowed(z, &l) {
                    self.choose_day(z)?;
                }
            }
            (Nav::Enter, view) => {
                if let Some(q) = self.picker.as_mut() {
                    q.view = if view == CalView::Years { CalView::Months } else { CalView::Days };
                }
                self.relayout();
            }
            (Nav::Edit(Key::Left), CalView::Days) => self.move_cursor(z - 1),
            (Nav::Edit(Key::Right), CalView::Days) => self.move_cursor(z + 1),
            (Nav::Up, CalView::Days) => self.move_cursor(z - 7),
            (Nav::Down, CalView::Days) => self.move_cursor(z + 7),
            (Nav::Edit(Key::Home), CalView::Days) => self.move_cursor(z - cal_weekday(z)),
            (Nav::Edit(Key::End), CalView::Days) => self.move_cursor(z + 6 - cal_weekday(z)),
            (Nav::PageUp | Nav::PageDown, view) => {
                let sign = if *k == Nav::PageUp { -1 } else { 1 };
                let months = match view {
                    CalView::Days if mods.shift => 12,
                    CalView::Days => 1,
                    CalView::Months => 12,
                    CalView::Years => 120,
                };
                self.move_cursor(cal_add_months(z, sign * months));
            }
            // In the grids of months and years, the arrows move one along, or a row of four.
            (Nav::Edit(Key::Left) | Nav::Edit(Key::Right) | Nav::Up | Nav::Down, view) => {
                let step = match k {
                    Nav::Edit(Key::Left) => -1,
                    Nav::Edit(Key::Right) => 1,
                    Nav::Up => -4,
                    _ => 4,
                };
                let per = if view == CalView::Years { 12 } else { 1 };
                self.move_cursor(cal_add_months(z, step * per));
            }
            _ => {}
        }
        Ok(true)
    }

    /// Keys while a colour pop-up is open.
    fn colour_key(&mut self, k: &Nav, mods: Mods, p: Picker) -> Result<bool, String> {
        let Some(b) = self.picker_node().map(|(_, b)| b.clone()) else { return Ok(false) };
        let alpha = b.look.alpha;
        let count = swatches_of(&b.look).len();
        match k {
            // Escape puts back the colour it had when it opened.
            Nav::Escape => {
                let owner = p.owner.clone();
                self.picker = None;
                let now = node_at(&self.tree, &owner).and_then(|n| match n.value {
                    Some(SV::Rgba(c)) => Some(c),
                    _ => None,
                });
                if now.is_some() && now != Some(p.start) {
                    return self.run(&owner, SV::Rgba(p.start)).map(|_| true);
                }
                self.relayout();
            }
            Nav::Enter => self.close_picker(),
            // Tab goes round the parts: the square, the hue, the alpha, the hex box, the swatches.
            Nav::Tab => {
                let mut order = vec![Part::Square, Part::Hue];
                if alpha {
                    order.push(Part::Alpha);
                }
                order.push(Part::Hex);
                if count > 0 {
                    order.push(Part::Swatches);
                }
                let at = order.iter().position(|x| *x == p.part).unwrap_or(0);
                let next = if mods.shift { (at + order.len() - 1) % order.len() } else { (at + 1) % order.len() };
                if let Some(q) = self.picker.as_mut() {
                    q.part = order[next];
                    if q.part == Part::Hex {
                        q.caret = Caret { anchor: 0, caret: q.hex.len() };
                    }
                }
                self.wake();
                self.relayout();
            }
            Nav::Edit(e) if p.part == Part::Hex => return self.picker_edit(e.clone(), mods).map(|_| true),
            Nav::Paste if p.part == Part::Hex => {
                if let Some(t) = self.clip_get() {
                    self.picker_edit(Key::Type(t.replace(['\r', '\n'], "")), Mods::default())?;
                }
            }
            Nav::Copy | Nav::Cut if p.part == Part::Hex => {
                let sel = p.caret.selected(&p.hex).to_string();
                if !sel.is_empty() {
                    self.clip_set(&sel);
                    if *k == Nav::Cut {
                        self.picker_edit(Key::Back, Mods::default())?;
                    }
                }
            }
            Nav::Edit(Key::Left | Key::Right) | Nav::Up | Nav::Down => {
                let fast = mods.shift;
                let dir = match k {
                    Nav::Edit(Key::Left) | Nav::Down => -1.0,
                    _ => 1.0,
                };
                match p.part {
                    Part::Square => {
                        let d = if fast { 0.1 } else { 0.01 };
                        if let Some(q) = self.picker.as_mut() {
                            if matches!(k, Nav::Edit(_)) {
                                q.hsv[1] = (q.hsv[1] + dir * d).clamp(0.0, 1.0);
                            } else {
                                q.hsv[2] = (q.hsv[2] + dir * d).clamp(0.0, 1.0);
                            }
                            q.from_hsv(alpha);
                        }
                        self.write_colour()?;
                    }
                    Part::Hue => {
                        let d = if fast { 10.0 } else { 1.0 };
                        if let Some(q) = self.picker.as_mut() {
                            q.hsv[0] = (q.hsv[0] + dir * d).clamp(0.0, 359.999);
                            q.from_hsv(alpha);
                        }
                        self.write_colour()?;
                    }
                    Part::Alpha => {
                        let d = if fast { 0.1 } else { 0.01 };
                        if let Some(q) = self.picker.as_mut() {
                            q.a = ((q.a as f64 + dir * d).clamp(0.0, 1.0) * 100.0).round() as f32 / 100.0;
                            q.hex = rgba_hex([q.rgb[0], q.rgb[1], q.rgb[2], q.a], true);
                        }
                        self.write_colour()?;
                    }
                    Part::Swatches if count > 0 => {
                        let step: i64 = match k {
                            Nav::Edit(Key::Left) => -1,
                            Nav::Edit(Key::Right) => 1,
                            Nav::Up => -(SWATCHES_A_ROW as i64),
                            _ => SWATCHES_A_ROW as i64,
                        };
                        let next = (p.swatch as i64 + step).clamp(0, count as i64 - 1) as usize;
                        self.pick_swatch(next)?;
                    }
                    _ => {}
                }
            }
            Nav::Space if p.part == Part::Swatches => self.pick_swatch(p.swatch)?,
            _ => {}
        }
        Ok(true)
    }

    /// Where a test's `click Today`, `click ‹` or `click 2026` lands in an open calendar.
    fn picker_spot(&self, text: &str) -> Option<(f32, f32)> {
        let p = self.picker.as_ref().filter(|p| !p.colour)?;
        let (_, b) = self.picker_node()?;
        let spot = match text {
            "Today" => CalSpot::Today,
            "‹" | "<" => CalSpot::Prev,
            "›" | ">" => CalSpot::Next,
            t if t == cal_title(p) => CalSpot::Title,
            t if p.view == CalView::Months => CalSpot::Month(MONTH_NAMES.iter().position(|m| *m == t || m[..3] == *t)? as i64),
            t if p.view == CalView::Years => CalSpot::Year(t.parse().ok()?),
            _ => return None,
        };
        let r = cal_spot_rect(p, spot)?;
        Some((b.x + r[0] + r[2] / 2.0, b.y + r[1] + r[3] / 2.0))
    }

    /// Where a test's `pickday 14`, `pickcolour 0.5 0.2`, `hue 0.6`, `alpha 0.5` or `swatch 3` lands.
    fn picker_test_spot(&self, cmd: &str, nums: &[f32]) -> Option<(f32, f32)> {
        let p = self.picker.as_ref()?;
        let (_, b) = self.picker_node()?;
        let first = *nums.first()?;
        if cmd == "pickday" {
            if p.colour {
                return None;
            }
            let (y, m, _) = cal_civil(p.cursor);
            let d = first as i64;
            if d < 1 || d > cal_month_len(y, m) {
                return None;
            }
            let r = cal_spot_rect(p, CalSpot::Day(cal_days_from(y, m, d)))?;
            return Some((b.x + r[0] + r[2] / 2.0, b.y + r[1] + r[3] / 2.0));
        }
        if !p.colour {
            return None;
        }
        let sw = swatches_of(&b.look);
        let (r, _) = col_rects(b.look.alpha, sw.len());
        // A place along a part, by fraction: the far end is just inside it.
        let along = |rr: [f32; 4], f: f32| b.x + rr[0] + f.clamp(0.0, 1.0) * (rr[2] - 0.01);
        match cmd {
            "pickcolour" => {
                let fy = *nums.get(1)?;
                Some((along(r[0], first), b.y + r[0][1] + fy.clamp(0.0, 1.0) * (r[0][3] - 0.01)))
            }
            "hue" => Some((along(r[1], first), b.y + r[1][1] + r[1][3] / 2.0)),
            "alpha" if b.look.alpha => Some((along(r[2], first), b.y + r[2][1] + r[2][3] / 2.0)),
            "swatch" => {
                let i = (first as usize).checked_sub(1).filter(|&i| i < sw.len())?;
                let s = swatch_rect(i, r[4]);
                Some((b.x + s[0] + s[2] / 2.0, b.y + s[1] + s[3] / 2.0))
            }
            _ => None,
        }
    }

    /// Typed text in a date picker: a real date between its min and max (or nothing, for no
    /// date) goes to its field; anything else stays on show with a red edge, and the field keeps
    /// its date.
    fn set_date_text(&mut self, path: &[usize], text: String) -> Result<(), String> {
        let Some(n) = node_at(&self.tree, path) else { return Ok(()) };
        let fits = date_fits(&text, &n.look);
        let changed = value_text(n) != text;
        // The field's own date is kept in the piece's text.
        let now = n.text.clone();
        self.draft = Some((path.to_vec(), text.clone()));
        if fits && now != text {
            self.run(path, SV::Text(text))
        } else {
            if changed {
                self.relayout();
            }
            Ok(())
        }
    }
}

impl Picker {
    /// Takes a colour from outside (its field, a swatch, the hex box): its hue, saturation and
    /// brightness are worked out again, keeping the hue of a grey and the saturation of black so
    /// the knobs don't jump.
    fn take_colour(&mut self, c: [f32; 4], alpha: bool) {
        let [h, s, v] = rgb_to_hsv(c[0] as f64, c[1] as f64, c[2] as f64);
        let grey = s <= 0.0 || v <= 0.0;
        self.hsv = [if grey { self.hsv[0] } else { h }, if v <= 0.0 { self.hsv[1] } else { s }, v];
        self.rgb = [c[0], c[1], c[2]];
        self.a = c[3];
        self.last = c;
        self.hex = rgba_hex(c, alpha);
        self.caret = Caret::at(self.hex.len());
    }

    /// The colour from the hue, saturation and brightness, after the square or hue changed.
    fn from_hsv(&mut self, alpha: bool) {
        let [r, g, b] = hsv_to_rgb(self.hsv[0], self.hsv[1], self.hsv[2]);
        self.rgb = [r as f32, g as f32, b as f32];
        self.hex = rgba_hex([self.rgb[0], self.rgb[1], self.rgb[2], if alpha { self.a } else { 1.0 }], alpha);
        self.caret = Caret::at(self.hex.len());
    }
}

#[cfg(test)]
mod pickers_tests {
    use super::*;

    #[test]
    fn dates_counted() {
        // Every day from 1899 to 2101 goes to its date and back.
        let (lo, hi) = (cal_days_from(1899, 1, 1), cal_days_from(2101, 12, 31));
        for z in lo..=hi {
            let (y, m, d) = cal_civil(z);
            assert_eq!(cal_days_from(y, m, d), z);
            assert_eq!(cal_day_of(&cal_text(z)), Some(z));
        }
        // Leap years: 2024 and 2000 are, 1900 and 2100 aren't.
        assert!(cal_day_of("2024-02-29").is_some());
        assert!(cal_day_of("2000-02-29").is_some());
        assert!(cal_day_of("1900-02-29").is_none());
        assert!(cal_day_of("2100-02-29").is_none());
        assert_eq!(cal_month_len(1900, 2), 28);
        assert_eq!(cal_month_len(2100, 2), 28);
        assert_eq!(cal_month_len(2000, 2), 29);
        for bad in ["", "2026-13-01", "2026-00-10", "2026-04-31", "2026-1-01", "26-01-01", "2026/01/01", "２０２６-01-01", "2026-01-1x"] {
            assert!(cal_day_of(bad).is_none(), "{bad}");
        }
        // 1 January 1970 was a Thursday; 13 November 2026 is a Friday.
        assert_eq!(cal_weekday(0), 3);
        assert_eq!(cal_weekday(cal_day_of("2026-11-13").unwrap()), 4);
        assert_eq!(cal_weekday(cal_day_of("1900-01-01").unwrap()), 0);
        assert_eq!(cal_weekday(cal_day_of("2100-12-31").unwrap()), 4);
    }

    #[test]
    fn month_grids() {
        // A month starting on a Sunday (March 2026) has six days of February before it.
        let z = cal_day_of("2026-03-15").unwrap();
        assert_eq!(cal_text(cal_grid_start(z)), "2026-02-23");
        // A month starting on a Monday (June 2026) starts the grid itself.
        assert_eq!(cal_text(cal_grid_start(cal_day_of("2026-06-30").unwrap())), "2026-06-01");
        // November 2026 starts on a Sunday too.
        assert_eq!(cal_text(cal_grid_start(cal_day_of("2026-11-13").unwrap())), "2026-10-26");
        // February 1900 (not a leap year) and 2100 fit in the grid.
        assert_eq!(cal_text(cal_grid_start(cal_day_of("1900-02-10").unwrap())), "1900-01-29");
        assert_eq!(cal_text(cal_grid_start(cal_day_of("2100-02-10").unwrap())), "2100-02-01");
        // Moving by months keeps the day, or takes the month's last.
        let jan31 = cal_day_of("2026-01-31").unwrap();
        assert_eq!(cal_text(cal_add_months(jan31, 1)), "2026-02-28");
        assert_eq!(cal_text(cal_add_months(cal_day_of("2024-01-31").unwrap(), 1)), "2024-02-29");
        assert_eq!(cal_text(cal_add_months(cal_day_of("2024-02-29").unwrap(), 12)), "2025-02-28");
        assert_eq!(cal_text(cal_add_months(jan31, -1)), "2025-12-31");
        assert_eq!(cal_text(cal_add_months(cal_day_of("1900-03-31").unwrap(), -1)), "1900-02-28");
        assert_eq!(decade_start(cal_day_of("2026-11-13").unwrap()), 2019);
    }

    #[test]
    fn dates_kept_in_range() {
        let mut l = base_look("datePicker");
        assert!(date_fits("", &l));
        assert!(date_fits("2026-11-13", &l));
        assert!(!date_fits("2026-11-31", &l));
        l.min = cal_day_of("2026-01-01").unwrap() as f64;
        l.max = cal_day_of("2026-12-31").unwrap() as f64;
        assert!(date_fits("2026-01-01", &l));
        assert!(date_fits("2026-12-31", &l));
        assert!(!date_fits("2025-12-31", &l));
        assert!(!date_fits("2027-01-01", &l));
        assert_eq!(cal_text(day_clamped(cal_day_of("2030-05-05").unwrap(), &l)), "2026-12-31");
        assert_eq!(cal_text(day_clamped(cal_day_of("1999-05-05").unwrap(), &l)), "2026-01-01");
    }

    #[test]
    fn hsv_round_trips() {
        let close = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9);
        assert!(close(hsv_to_rgb(0.0, 1.0, 1.0), [1.0, 0.0, 0.0]));
        assert!(close(hsv_to_rgb(120.0, 1.0, 1.0), [0.0, 1.0, 0.0]));
        assert!(close(hsv_to_rgb(240.0, 1.0, 1.0), [0.0, 0.0, 1.0]));
        assert!(close(hsv_to_rgb(360.0, 1.0, 1.0), [1.0, 0.0, 0.0]));
        assert!(close(hsv_to_rgb(200.0, 0.0, 0.5), [0.5, 0.5, 0.5]));
        assert!(close(rgb_to_hsv(1.0, 0.0, 0.0), [0.0, 1.0, 1.0]));
        assert!(close(rgb_to_hsv(0.0, 0.0, 0.0), [0.0, 0.0, 0.0]));
        assert!(close(rgb_to_hsv(0.5, 0.5, 0.5), [0.0, 0.0, 0.5]));
        // Every byte colour on a coarse grid goes to HSV and back to the same bytes, and the f32
        // it was within a hair.
        for r in (0..=255).step_by(15) {
            for g in (0..=255).step_by(17) {
                for b in (0..=255).step_by(51) {
                    let c = rgba_bytes(r, g, b, 255);
                    let [h, s, v] = rgb_to_hsv(c[0] as f64, c[1] as f64, c[2] as f64);
                    let back = hsv_to_rgb(h, s, v);
                    let again = [back[0] as f32, back[1] as f32, back[2] as f32, 1.0];
                    assert_eq!(rgba_hex(again, true), rgba_hex(c, true));
                    for i in 0..3 {
                        assert!((again[i] - c[i]).abs() <= 1e-6, "{r} {g} {b}");
                    }
                }
            }
        }
    }

    #[test]
    fn hex_read_and_written() {
        assert_eq!(hex_parse("#3b82f6", false), Some(rgba_bytes(0x3b, 0x82, 0xf6, 255)));
        assert_eq!(hex_parse("3B82F6", false), Some(rgba_bytes(0x3b, 0x82, 0xf6, 255)));
        assert_eq!(hex_parse("#abc", false), Some(rgba_bytes(0xaa, 0xbb, 0xcc, 255)));
        assert_eq!(hex_parse("#abcd", true), Some(rgba_bytes(0xaa, 0xbb, 0xcc, 0xdd)));
        assert_eq!(hex_parse("#3b82f680", true), Some(rgba_bytes(0x3b, 0x82, 0xf6, 0x80)));
        // See-through only with [alpha].
        assert_eq!(hex_parse("#3b82f680", false), None);
        assert_eq!(hex_parse("#abcd", false), None);
        for bad in ["", "#", "#12", "#12345", "#1234567", "#ggg", "#123456789", "#+12345", "# 123456"] {
            assert_eq!(hex_parse(bad, true), None, "{bad}");
        }
        // Written as Lust's :hex() does.
        assert_eq!(rgba_hex(rgba_bytes(0x3b, 0x82, 0xf6, 255), true), "#3b82f6");
        assert_eq!(rgba_hex(rgba_bytes(0x3b, 0x82, 0xf6, 0x80), true), "#3b82f680");
        assert_eq!(rgba_hex(rgba_bytes(0x3b, 0x82, 0xf6, 0x80), false), "#3b82f6");
        assert_eq!(rgba_hex([0.4, 0.7, 1.0, 1.0], true), "#66b2ff");
        // The colour words at their middle shade, with exactly their bytes.
        let sw = default_swatches();
        assert_eq!(sw.len(), 12);
        assert_eq!(rgba_hex(sw[8], true), "#339af0");
    }

    #[test]
    fn grey_keeps_its_hue() {
        let mut p = Picker {
            owner: vec![],
            colour: true,
            cursor: 0,
            view: CalView::Days,
            hsv: [210.0, 0.5, 0.5],
            rgb: [0.0; 3],
            a: 1.0,
            start: [0.0; 4],
            last: [0.0; 4],
            part: Part::Square,
            hex: String::new(),
            caret: Caret::default(),
            swatch: 0,
        };
        p.take_colour([0.5, 0.5, 0.5, 1.0], false);
        assert_eq!(p.hsv[0], 210.0);
        p.take_colour([0.0, 0.0, 0.0, 1.0], false);
        assert_eq!((p.hsv[0], p.hsv[1]), (210.0, 0.0));
        assert_eq!(p.hex, "#000000");
    }
}
