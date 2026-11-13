// Views, continued: tables and lists, with rows made only while they're on the screen.
//
// This is part of the same module as native_ui.rs (lust includes both, and built programs get
// both, one after the other), kept apart so neither file grows too long.
//
// A table or list can hold 100,000 rows and stay quick, because the view's code asks here which
// rows are on the screen (list_range) and makes only those, plus a few spare. Every row is the
// same height, so where row 50,000 goes is a sum, not a walk past the 49,999 before it. What the
// window remembers about each table (how far it's scrolled, how it's sorted, the widths its
// columns were dragged to) is kept here by the table's place in the view, as tabs and splits are.

/// What a table or list node carries besides its pieces: which table it is (for what the window
/// remembers about it), how many rows it has, the first row made, and the row that's picked.
#[derive(Clone, Debug, Default)]
pub struct ListInfo {
    pub id: usize,
    pub count: usize,
    pub first: usize,
    pub pick: Option<usize>,
}

/// A value a column is sorted by: numbers as numbers, and text without minding capitals. A bool
/// or a state's option is a number (false 0, true 1; the option's place in its state).
#[derive(Clone, Debug, PartialEq)]
pub enum SortKey {
    Num(f64),
    Text(String),
}

/// What the window remembers about one table or list.
#[derive(Default)]
struct ListState {
    /// How far the rows are scrolled, and how tall the room for them was last time.
    scroll: f32,
    view_h: f32,
    /// Each row's height: given with rowHeight, or worked out from the first row (0 until then).
    row_h: f32,
    given_h: f32,
    count: usize,
    /// The rows made this time.
    built: (usize, usize),
    /// The column sorted by (from 0), and whether it's falling.
    sort: Option<(usize, bool)>,
    /// Widths the columns were dragged to.
    widths: HashMap<usize, f32>,
    /// The last sort: its keys, whether falling, and the order it gave; and the column and when.
    order: Option<(Vec<SortKey>, bool, Vec<usize>)>,
    sorted_at: (usize, u64),
    /// The picked row's place in the order shown, from the last time it was seen.
    pick: Option<usize>,
}

#[derive(Default)]
struct Lists {
    states: HashMap<usize, ListState>,
    /// Which table is which: by where it's written and its turn there, like a piece.
    ids: HashMap<(usize, usize), usize>,
    turns: HashMap<usize, usize>,
    /// A table's rows were made for the wrong place (a guessed row height or room), so the view
    /// should be made again.
    stale: bool,
}

thread_local! {
    static LISTS: std::cell::RefCell<Lists> = std::cell::RefCell::new(Lists::default());
    /// Counts the times Lust code ran (a click's block, a timer, a tween's step): only then can
    /// the rows have changed, so a sort is kept while this stays the same.
    static EPOCH: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn data_may_change() {
    EPOCH.with(|e| e.set(e.get() + 1));
}

/// Rows made above and below the ones on the screen, so a small scroll needs no new rows.
const SPARE: usize = 4;
/// A row's height before it's known, small so the first guess makes enough rows.
const GUESS_H: f32 = 20.0;
const HEADER_PAD: [f32; 4] = [7.0, 26.0, 7.0, 10.0];
const MIN_COLUMN: f32 = 40.0;

/// Each drawing of the view counts tables from the start again.
fn list_frame() {
    LISTS.with(|l| l.borrow_mut().turns.clear());
}

fn with_list<T>(id: usize, f: impl FnOnce(&mut ListState) -> T) -> T {
    LISTS.with(|l| f(l.borrow_mut().states.entry(id).or_default()))
}

/// A table or list written at `site` is being made, with `count` rows. Gives back which one it is.
pub fn list_begin(site: usize, count: usize, row_h: f64) -> usize {
    LISTS.with(|l| {
        let mut l = l.borrow_mut();
        let turn = l.turns.entry(site).or_insert(0);
        *turn += 1;
        let key = (site, *turn);
        let next = l.ids.len() + 1;
        let id = *l.ids.entry(key).or_insert(next);
        let st = l.states.entry(id).or_default();
        st.count = count;
        st.given_h = row_h.max(0.0) as f32;
        if st.given_h > 0.0 {
            st.row_h = st.given_h;
        }
        id
    })
}

/// The column the table is sorted by (from 1; 0 for none), whether it's falling, and whether the
/// last order still holds (no Lust code has run since, so the rows are as they were), so that
/// scrolling a sorted table doesn't read every row again.
pub fn list_sort(id: usize) -> (usize, bool, bool) {
    let now = EPOCH.with(|e| e.get());
    with_list(id, |st| match st.sort {
        Some((c, f)) => {
            let kept = st.sorted_at == (c + 1, now) && matches!(&st.order, Some((k, was, _)) if *was == f && k.len() == st.count);
            (c + 1, f, kept)
        }
        None => (0, false, false),
    })
}

/// The order the last sort gave, while it still holds.
pub fn list_kept(id: usize) -> Option<Vec<usize>> {
    with_list(id, |st| st.order.as_ref().map(|(_, _, o)| o.clone()))
}

/// The order to show the rows in, sorted by these keys: smallest first, or with `falling` largest
/// first. Rows with the same key keep the order they have. Kept while the keys stay the same.
pub fn list_order(id: usize, keys: Vec<SortKey>, falling: bool) -> Vec<usize> {
    let now = EPOCH.with(|e| e.get());
    with_list(id, |st| st.sorted_at = (st.sort.map(|(c, _)| c + 1).unwrap_or(0), now));
    if let Some(o) = with_list(id, |st| match &st.order {
        Some((k, f, o)) if *f == falling && *k == keys => Some(o.clone()),
        _ => None,
    }) {
        return o;
    }
    let order = sort_order(&keys, falling);
    with_list(id, |st| st.order = Some((keys, falling, order.clone())));
    order
}

fn sort_order(keys: &[SortKey], falling: bool) -> Vec<usize> {
    let lower: Vec<Option<String>> = keys
        .iter()
        .map(|k| match k {
            SortKey::Text(t) => Some(t.to_lowercase()),
            SortKey::Num(_) => None,
        })
        .collect();
    let mut order: Vec<usize> = (0..keys.len()).collect();
    order.sort_by(|&a, &b| {
        let c = match (&keys[a], &keys[b]) {
            (SortKey::Num(x), SortKey::Num(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
            _ => lower[a].cmp(&lower[b]).then_with(|| match (&keys[a], &keys[b]) {
                (SortKey::Text(x), SortKey::Text(y)) => x.cmp(y),
                _ => std::cmp::Ordering::Equal,
            }),
        };
        if falling {
            c.reverse()
        } else {
            c
        }
    });
    order
}

/// Where the picked row was last time, in the order shown (or -1), to look there first.
pub fn list_hint(id: usize) -> i64 {
    with_list(id, |st| st.pick.map(|p| p as i64).unwrap_or(-1))
}

/// Which rows to make: from the first up to (not including) the last.
pub fn list_range(id: usize) -> (usize, usize) {
    with_list(id, |st| {
        let r = needed(st);
        st.built = r;
        r
    })
}

/// The rows on the screen as things are now, with spare ones each side.
fn needed(st: &ListState) -> (usize, usize) {
    let rh = if st.row_h > 0.0 { st.row_h } else { GUESS_H };
    let vh = if st.view_h > 0.0 { st.view_h } else { 2000.0 };
    let max = (st.count as f32 * rh - vh).max(0.0);
    let scroll = st.scroll.clamp(0.0, max);
    let first = (scroll / rh).floor() as usize;
    let last = ((scroll + vh) / rh).ceil() as usize;
    (first.saturating_sub(SPARE).min(st.count), (last + SPARE).min(st.count))
}

/// Whether the rows made cover the ones that should show now.
fn covered(st: &ListState) -> bool {
    let (a, b) = needed(st);
    let (ba, bb) = st.built;
    a >= ba && b <= bb
}

fn take_stale() -> bool {
    LISTS.with(|l| std::mem::take(&mut l.borrow_mut().stale))
}

// ----- laying out -----

/// A table's pieces, as the view made them: its columns' headers go in a header row, and its
/// rows follow. A list has only rows.
fn list_lay(n: &Node, laid: &mut Laid) {
    laid.list = n.list.clone();
    let info = n.list.clone().unwrap_or_default();
    if laid.look.border.is_none() && n.kind == "table" {
        laid.look.border = Some(nt(0xdee2e6));
    }
    let stripes = laid.look.stripes;
    let children = std::mem::take(&mut laid.children);
    let mut head = lay(&Node::new("thead"));
    let mut rows = Vec::new();
    let sort = with_list(info.id, |st| st.sort);
    for c in children {
        if c.kind == "header" {
            let mut h = c;
            h.index = head.children.len();
            h.value = match sort {
                Some((col, f)) if col == h.index => Some(SV::Text(if f { "falling" } else { "rising" }.into())),
                _ => None,
            };
            head.children.push(h);
        } else {
            rows.push(c);
        }
    }
    for (k, r) in rows.iter_mut().enumerate() {
        let j = info.first + k;
        // A disabled table or list can't be picked from.
        if laid.look.disabled {
            r.action = None;
            r.enter = None;
        }
        r.index = j;
        let picked = info.pick == Some(j);
        r.value = Some(SV::Truth(picked));
        r.look.background = if picked {
            Some(picked_colour())
        } else if stripes && j % 2 == 1 {
            Some(nt(0xf8f9fa))
        } else {
            None
        };
        let mut over = r.look.clone();
        over.background = Some(match r.look.background {
            Some(b) if picked => mix(b, 0x1c7ed6, 0.12),
            _ => nt(0xf1f3f5),
        });
        r.hover = (r.action.is_some() || r.enter.is_some()).then_some(over);
    }
    if n.kind == "table" {
        laid.children = vec![head];
    }
    laid.children.extend(rows);
}

fn picked_colour() -> u32 {
    nt(0xe7f5ff)
}

/// The rows of a table or list, after its header.
fn first_row(n: &Laid) -> usize {
    (n.kind == "table") as usize
}

/// How tall a table's header row is.
fn header_height(n: &Laid) -> f32 {
    if n.kind != "table" {
        return 0.0;
    }
    n.children.first().map(|h| h.children.iter().map(|c| natural(c, f32::INFINITY).1).fold(0.0, f32::max)).unwrap_or(0.0).max(30.0)
}

/// A table's or list's own size when nothing says: as wide as it can be, and tall enough for a
/// few rows.
fn list_natural(n: &Laid, inner: f32) -> (f32, f32) {
    let id = n.list.as_ref().map(|l| l.id).unwrap_or(0);
    let (rh, count) = with_list(id, |st| (if st.row_h > 0.0 { st.row_h } else { 32.0 }, st.count));
    let w = if inner.is_finite() { inner } else { 400.0 };
    (w, header_height(n) + rh * count.min(8) as f32)
}

/// Each column's left edge and width, `iw` wide in all: columns given a width have it (or what it
/// was dragged to), and the others share the rest.
fn column_sizes(n: &Laid, iw: f32) -> Vec<(f32, f32)> {
    let Some(head) = n.children.first().filter(|h| h.kind == "thead") else { return vec![] };
    let id = n.list.as_ref().map(|l| l.id).unwrap_or(0);
    let dragged = with_list(id, |st| st.widths.clone());
    let fixed: Vec<Option<f32>> = head
        .children
        .iter()
        .enumerate()
        .map(|(i, h)| match (dragged.get(&i), h.look.width) {
            (Some(&w), _) => Some(w),
            (None, Size::Fixed(w)) => Some(w),
            _ => None,
        })
        .collect();
    let used: f32 = fixed.iter().flatten().sum();
    let free = fixed.iter().filter(|f| f.is_none()).count();
    let share = if free > 0 { ((iw - used) / free as f32).max(MIN_COLUMN) } else { 0.0 };
    let mut at = 0.0;
    fixed
        .iter()
        .map(|f| {
            let w = f.unwrap_or(share).max(MIN_COLUMN).round();
            let r = (at, w);
            at += w;
            r
        })
        .collect()
}

/// Places a table's header and the rows that were made, each where it is in the whole list.
fn place_list(n: &mut Laid, x: f32, y: f32, w: f32, h: f32) {
    let Some(info) = n.list.clone() else { return };
    let l = n.look.clone();
    let (ix, iy) = (x + l.pad[3], y + l.pad[0]);
    let (iw, ih) = ((w - l.pad[1] - l.pad[3]).max(0.0), (h - l.pad[0] - l.pad[2]).max(0.0));
    let hh = header_height(n);
    let first = first_row(n);
    // The row height: given, or worked out from the first row made.
    let mut rh = with_list(info.id, |st| st.row_h);
    if rh <= 0.0 {
        if let Some(r) = n.children.get(first) {
            let cols = column_sizes(n, iw);
            rh = if n.kind == "table" {
                r.children.iter().zip(&cols).map(|(c, &(_, cw))| natural(c, cw).1).fold(0.0, f32::max)
            } else {
                natural(r, iw).1
            }
            .max(24.0)
            .round();
        }
    }
    let view_h = (ih - hh).max(0.0);
    let stale = with_list(info.id, |st| {
        if rh > 0.0 {
            st.row_h = rh;
        }
        st.view_h = view_h;
        let max = (st.count as f32 * st.row_h.max(1.0) - view_h).max(0.0);
        st.scroll = st.scroll.clamp(0.0, max).round();
        !covered(st)
    });
    if stale {
        LISTS.with(|l| l.borrow_mut().stale = true);
    }
    let scroll = with_list(info.id, |st| st.scroll);
    n.content_h = hh + info.count as f32 * rh;
    n.scrolled = scroll;
    let cols = column_sizes(n, iw);
    if n.kind == "table" {
        let head = &mut n.children[0];
        head.x = ix;
        head.y = iy;
        head.w = iw;
        head.h = hh;
        for (c, &(cx, cw)) in head.children.iter_mut().zip(&cols) {
            place(c, ix + cx, iy, cw, hh);
        }
    }
    let top = iy + hh;
    for r in n.children.iter_mut().skip(first) {
        let ry = (top + r.index as f32 * rh - scroll).round();
        if n.kind == "table" {
            r.x = ix;
            r.y = ry;
            r.w = iw;
            r.h = rh;
            for (c, &(cx, cw)) in r.children.iter_mut().zip(&cols) {
                place(c, ix + cx, ry, cw, rh);
            }
        } else {
            place(r, ix, ry, iw, rh);
        }
    }
}

/// The room a table's or list's rows show in, below the header.
fn list_body(n: &Laid) -> (f32, f32, f32, f32) {
    let l = &n.look;
    let hh = if n.kind == "table" { n.children.first().map(|h| h.h).unwrap_or(0.0) } else { 0.0 };
    let (x, y) = (n.x + l.pad[3], n.y + l.pad[0] + hh);
    (x, y, (n.w - l.pad[1] - l.pad[3]).max(0.0), (n.h - l.pad[0] - l.pad[2] - hh).max(0.0))
}

/// Whether a row shows in the room for rows, even partly.
fn row_shows(n: &Laid, r: &Laid) -> bool {
    let (_, by, _, bh) = list_body(n);
    r.y + r.h > by + 0.5 && r.y < by + bh - 0.5
}

/// The scroll bar's knob beside a table's rows: where it starts and how tall it is.
fn list_knob(n: &Laid) -> Option<(f32, f32)> {
    let (_, by, _, bh) = list_body(n);
    let rows_h = n.content_h - (n.h - bh).max(0.0) + n.look.pad[0] + n.look.pad[2];
    let total = n.content_h - (by - n.y - n.look.pad[0]);
    let _ = rows_h;
    if total <= bh + 0.5 || bh <= 0.0 {
        return None;
    }
    let kh = (bh * bh / total).max(24.0).min(bh);
    let max = total - bh;
    Some((by + (n.scrolled / max).min(1.0) * (bh - kh), kh))
}

// ----- painting -----

/// A table's or list's rows, cut off at the room for them, then its header over them.
fn paint_list(c: &mut Canvas, n: &Laid, st: &PaintState, path: &mut Vec<usize>) {
    let (bx, by, bw, bh) = list_body(n);
    let was = c.clip_to(bx, by, bw, bh);
    let first = first_row(n);
    for (i, r) in n.children.iter().enumerate().skip(first) {
        if !row_shows(n, r) {
            continue;
        }
        path.push(i);
        paint(c, r, st, path);
        path.pop();
    }
    c.clip = was;
    if let Some((ky, kh)) = list_knob(n) {
        c.rect(n.x + n.w - n.look.pad[1] - 8.0, ky + 2.0, 5.0, kh - 4.0, 2.5, knob().0, knob().1);
    }
    if first == 1 {
        let head = &n.children[0];
        let was = c.clip_to(n.x, n.y, n.w, n.h);
        // Rounded at the top, inside the table's frame.
        let r = (n.look.radius - 1.0).max(0.0).min(head.h / 2.0);
        c.rect(head.x, head.y, head.w, head.h, r, nt(0xf8f9fa), 1.0);
        c.rect(head.x, head.y + head.h / 2.0, head.w, head.h / 2.0, 0.0, nt(0xf8f9fa), 1.0);
        c.rect(head.x, head.y + head.h - 1.0, head.w, 1.0, 0.0, nt(0xdee2e6), 1.0);
        path.push(0);
        for (i, h) in head.children.iter().enumerate() {
            path.push(i);
            paint(c, h, st, path);
            path.pop();
            // A thin line between the headers, where they're dragged to resize.
            if i + 1 < head.children.len() {
                c.rect(h.x + h.w - 1.0, h.y + 7.0, 1.0, (h.h - 14.0).max(0.0), 0.0, nt(0xdee2e6), 1.0);
            }
        }
        path.pop();
        c.clip = was;
    }
}

/// What's drawn on a row and a header besides their pieces: a row's line along its bottom, and
/// the arrow on the header a table is sorted by.
fn paint_list_part(c: &mut Canvas, n: &Laid, l: &Look) {
    match n.kind.as_str() {
        "tablerow" | "listrow" => c.rect(n.x, n.y + n.h - 1.0, n.w, 1.0, 0.0, nt(0xe9ecef), 1.0),
        "header" => {
            let icon = match &n.value {
                Some(SV::Text(t)) if t == "rising" => "arrow-up",
                Some(SV::Text(t)) if t == "falling" => "arrow-down",
                _ => return,
            };
            let size = 14.0;
            paint_icon(c, icon, n.x + n.w - size - 8.0, n.y + (n.h - size) / 2.0, size, l.color);
        }
        _ => {}
    }
}

// ----- the mouse and keys -----

/// Every table or list at this point, outermost first.
fn lists_at(n: &Laid, x: f32, y: f32, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if x < n.x || y < n.y || x >= n.x + n.w || y >= n.y + n.h {
        return;
    }
    if n.list.is_some() {
        out.push(path.clone());
    }
    for (i, c) in n.children.iter().enumerate() {
        path.push(i);
        lists_at(c, x, y, path, out);
        path.pop();
    }
}

/// The edge of a table's header at this point, to drag to make a column wider: the table's place
/// and the column.
fn header_edge(n: &Laid, x: f32, y: f32, path: &mut Vec<usize>) -> Option<(Vec<usize>, usize)> {
    if x < n.x - 4.0 || y < n.y || x >= n.x + n.w + 4.0 || y >= n.y + n.h {
        return None;
    }
    if n.kind == "table" {
        if let Some(head) = n.children.first() {
            if y >= head.y && y < head.y + head.h {
                for (i, h) in head.children.iter().enumerate() {
                    if (x - (h.x + h.w)).abs() <= 4.0 {
                        return Some((path.clone(), i));
                    }
                }
            }
        }
    }
    for (i, c) in n.children.iter().enumerate() {
        path.push(i);
        let found = header_edge(c, x, y, path);
        path.pop();
        if found.is_some() {
            return found;
        }
    }
    None
}

/// The table or list a path goes through, if any: how deep it is, and the row's place under it.
fn list_on_path(tree: &Laid, path: &[usize]) -> Option<(usize, ListInfo)> {
    let mut n = tree;
    let mut found = None;
    for (d, &i) in path.iter().enumerate() {
        if let Some(info) = &n.list {
            found = Some((d, info.clone()));
        }
        n = n.children.get(i)?;
    }
    found
}

impl<'a, 'b> Ui<'a, 'b> {
    /// Makes the view again until each table has made the rows it shows (a row's height or the
    /// room for the rows was only guessed at first).
    fn settle(&mut self) -> Result<(), String> {
        for _ in 0..3 {
            if !take_stale() {
                break;
            }
            self.remake()?;
        }
        take_stale();
        Ok(())
    }

    /// Makes the view again after a table's rows moved.
    fn remake(&mut self) -> Result<(), String> {
        self.rebuild_only()
    }

    /// Where what has focus (and what's held down, and a number box's typing) is in a table's
    /// rows, before the view is made again.
    fn rows_before(&self) -> [Option<(usize, usize, usize)>; 3] {
        let at = |p: Option<&Vec<usize>>| p.and_then(|p| list_on_path(&self.tree, p).map(|(d, i)| (d, i.id, i.first)));
        [at(self.st.focus.as_ref()), at(self.pressed.as_ref()), at(self.draft.as_ref().map(|d| &d.0))]
    }

    /// The rows made start somewhere else now, so what was in a row has a new place: it follows
    /// its row, or goes if its row isn't made any more.
    fn follow_rows(&mut self, was: [Option<(usize, usize, usize)>; 3]) {
        let tree = &self.tree;
        let after = |p: Option<&mut Vec<usize>>, was: Option<(usize, usize, usize)>| -> bool {
            let (Some((d, id, first)), Some(path)) = (was, p) else { return true };
            let Some(t) = node_at(tree, &path[..d]).filter(|n| n.list.as_ref().is_some_and(|i| i.id == id)) else { return false };
            let info = t.list.clone().unwrap_or_default();
            let lo = first_row(t) as i64;
            if (path[d] as i64) < lo {
                return true;
            }
            let at = path[d] as i64 + first as i64 - info.first as i64;
            if at >= lo && at < t.children.len() as i64 {
                path[d] = at as usize;
                true
            } else {
                false
            }
        };
        if !after(self.st.focus.as_mut(), was[0]) {
            self.st.focus = None;
        }
        if !after(self.pressed.as_mut(), was[1]) {
            self.pressed = None;
        }
        if !after(self.draft.as_mut().map(|d| &mut d.0), was[2]) {
            self.draft = None;
        }
    }

    /// Scrolls the table or list at `path` by `dy`. Its rows are made again only when ones that
    /// weren't made come into view.
    fn list_scroll(&mut self, path: &[usize], to: f32) -> Result<bool, String> {
        let Some(info) = node_at(&self.tree, path).and_then(|n| n.list.clone()) else { return Ok(false) };
        let moved = with_list(info.id, |st| {
            let max = (st.count as f32 * st.row_h.max(1.0) - st.view_h).max(0.0);
            let to = to.clamp(0.0, max).round();
            let moved = (to - st.scroll).abs() >= 0.5;
            st.scroll = to;
            moved
        });
        if !moved {
            return Ok(false);
        }
        if with_list(info.id, |st| covered(st)) {
            self.relayout();
        } else {
            self.remake()?;
            self.settle()?;
        }
        Ok(true)
    }

    /// The wheel over a table or list scrolls it. Returns None when there's none there that can
    /// scroll further, so the wheel goes to what's around it.
    fn wheel_list(&mut self, x: f32, y: f32, dy: f32) -> Option<Result<(), String>> {
        let mut found = Vec::new();
        for i in self.reach() {
            lists_at(&self.tree.children[i], x, y, &mut vec![i], &mut found);
        }
        for p in found.iter().rev() {
            let Some(n) = node_at(&self.tree, p) else { continue };
            match self.list_scroll(p, n.scrolled + dy) {
                Ok(true) => return Some(Ok(())),
                Ok(false) => continue,
                Err(e) => return Some(Err(e)),
            }
        }
        None
    }

    /// Picks the row shown `j`th (from 0) in the table or list at `path`: scrolls it into view,
    /// makes it if it wasn't made, runs its select, and with `open`, its open block too.
    fn pick_row(&mut self, path: &[usize], j: usize, open: bool) -> Result<(), String> {
        let Some(n) = node_at(&self.tree, path) else { return Ok(()) };
        let Some(info) = n.list.clone() else { return Ok(()) };
        if info.count == 0 {
            return Ok(());
        }
        let j = j.min(info.count - 1);
        // Into view: just above the top, or just below the bottom.
        let to = with_list(info.id, |st| {
            let rh = st.row_h.max(1.0);
            let (top, bottom) = (j as f32 * rh, (j + 1) as f32 * rh);
            if top < st.scroll {
                top
            } else if bottom > st.scroll + st.view_h {
                bottom - st.view_h
            } else {
                st.scroll
            }
        });
        self.list_scroll(path, to)?;
        let Some(n) = node_at(&self.tree, path) else { return Ok(()) };
        let Some(info) = n.list.clone() else { return Ok(()) };
        let k = first_row(n) + j.saturating_sub(info.first);
        let (action, enter) = match n.children.get(k).filter(|r| r.index == j) {
            Some(r) => (r.action, r.enter),
            None => return Ok(()),
        };
        with_list(info.id, |st| st.pick = Some(j));
        if let Some(a) = action {
            run_with(&mut self.actions[a], SV::Flag)?;
            self.rebuild()?;
        }
        if open {
            // The open block of the same row, made again after the pick.
            let enter = node_at(&self.tree, path)
                .and_then(|n| {
                    let first = n.list.as_ref().map(|i| i.first).unwrap_or(0);
                    n.children.get(first_row(n) + j.saturating_sub(first)).filter(|r| r.index == j)
                })
                .map(|r| r.enter)
                .unwrap_or(enter);
            if let Some(a) = enter {
                run_with(&mut self.actions[a], SV::Flag)?;
                self.rebuild()?;
            }
        }
        Ok(())
    }

    /// The keys on a table or list with focus: Up and Down move the pick, Home and End go to the
    /// first and last rows, Page Up and Page Down a page at a time, and Enter opens the picked row.
    fn list_key(&mut self, path: &[usize], k: &Nav) -> Result<bool, String> {
        let Some(n) = node_at(&self.tree, path) else { return Ok(false) };
        let Some(info) = n.list.clone() else { return Ok(false) };
        if info.count == 0 {
            return Ok(matches!(k, Nav::Up | Nav::Down | Nav::PageUp | Nav::PageDown | Nav::Edit(Key::Home | Key::End)));
        }
        let page = with_list(info.id, |st| ((st.view_h / st.row_h.max(1.0)).floor() as usize).saturating_sub(1).max(1));
        let last = info.count - 1;
        let to = match (k, info.pick) {
            (Nav::Enter, Some(p)) => return self.pick_row(path, p, true).map(|_| true),
            (Nav::Enter, None) => return Ok(true),
            (Nav::Edit(Key::Home), _) => 0,
            (Nav::Edit(Key::End), _) => last,
            (Nav::Up | Nav::Down | Nav::PageUp | Nav::PageDown, None) => 0,
            (Nav::Up, Some(p)) => p.saturating_sub(1),
            (Nav::Down, Some(p)) => (p + 1).min(last),
            (Nav::PageUp, Some(p)) => p.saturating_sub(page),
            (Nav::PageDown, Some(p)) => (p + page).min(last),
            _ => return Ok(false),
        };
        self.pick_row(path, to, false)?;
        Ok(true)
    }

    /// Clicking a sortable header sorts by it: rising, then falling, then the rows' own order.
    fn sort_by(&mut self, header: &[usize]) -> Result<(), String> {
        let Some((&col, rest)) = header.split_last() else { return Ok(()) };
        let Some(table) = rest.split_last().map(|(_, t)| t) else { return Ok(()) };
        let Some(info) = node_at(&self.tree, table).and_then(|n| n.list.clone()) else { return Ok(()) };
        with_list(info.id, |st| {
            st.sort = match st.sort {
                Some((c, false)) if c == col => Some((col, true)),
                Some((c, true)) if c == col => None,
                _ => Some((col, false)),
            };
        });
        self.remake()?;
        self.settle()
    }

    /// Dragging the edge of a header: the column is as wide as the drag says, at least 40.
    fn drag_column(&mut self, table: &[usize], col: usize, from_x: f32, start: f32, x: f32) {
        let Some(info) = node_at(&self.tree, table).and_then(|n| n.list.clone()) else { return };
        with_list(info.id, |st| st.widths.insert(col, (start + x - from_x).max(MIN_COLUMN).round()));
        self.relayout();
    }
}

/// `table 100000 rows, showing 41–58` for dump: how many rows, and which are on the screen.
fn list_dump_line(n: &Laid) -> String {
    let Some(info) = &n.list else { return String::new() };
    let shown: Vec<usize> = n.children.iter().skip(first_row(n)).filter(|r| row_shows(n, r)).map(|r| r.index + 1).collect();
    let rows = if info.count == 1 { "row" } else { "rows" };
    match (shown.first(), shown.last()) {
        (Some(a), Some(b)) => format!(" {} {rows}, showing {a}\u{2013}{b}", info.count),
        _ => format!(" {} {rows}", info.count),
    }
}

/// The text of a row on the screen, to click it in tests: the first text piece with these words
/// inside a table's or list's row that shows.
fn find_row_text(n: &Laid, text: &str, path: &mut Vec<usize>) -> Option<Vec<usize>> {
    fn inside(n: &Laid, text: &str, path: &mut Vec<usize>) -> Option<Vec<usize>> {
        if (n.kind == "text" || n.kind == "celltext") && n.text == text {
            return Some(path.clone());
        }
        for (i, c) in n.children.iter().enumerate() {
            path.push(i);
            let f = inside(c, text, path);
            path.pop();
            if f.is_some() {
                return f;
            }
        }
        None
    }
    for (i, c) in n.children.iter().enumerate() {
        path.push(i);
        let found = if n.list.is_some() && i >= first_row(n) {
            if row_shows(n, c) {
                inside(c, text, path)
            } else {
                None
            }
        } else {
            find_row_text(c, text, path)
        };
        path.pop();
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Runs a view's test with these commands instead of what's typed in (for lust's timing tests),
/// and gives back how long each took, after opening the window first.
pub fn test_commands(lines: Vec<String>) {
    TEST_LINES.with(|t| *t.borrow_mut() = Some(lines));
    TEST_TIMES.with(|t| t.borrow_mut().clear());
}

pub fn test_times() -> Vec<(String, std::time::Duration)> {
    TEST_TIMES.with(|t| t.borrow().clone())
}

#[cfg(test)]
mod list_tests {
    use super::*;

    #[test]
    fn sorting_keeps_equal_rows_in_order() {
        let keys = vec![SortKey::Num(2.0), SortKey::Num(1.0), SortKey::Num(2.0), SortKey::Num(0.5)];
        assert_eq!(sort_order(&keys, false), vec![3, 1, 0, 2]);
        assert_eq!(sort_order(&keys, true), vec![0, 2, 1, 3]);
        let names = ["bob", "Ada", "carl", "ada"].iter().map(|s| SortKey::Text(s.to_string())).collect::<Vec<_>>();
        assert_eq!(sort_order(&names, false), vec![1, 3, 0, 2]);
        assert!(sort_order(&[], false).is_empty());
    }

    #[test]
    fn the_rows_made_follow_the_scroll() {
        let mut st = ListState { row_h: 30.0, view_h: 300.0, count: 100_000, ..Default::default() };
        assert_eq!(needed(&st), (0, 14));
        st.scroll = 1_500_000.0;
        let (a, b) = needed(&st);
        assert!(a <= 50_000 && b >= 50_010 && b - a < 30, "{a} {b}");
        st.scroll = f32::MAX;
        assert_eq!(needed(&st).1, 100_000);
        st.count = 0;
        assert_eq!(needed(&st), (0, 0));
    }
}
