// ----- more windows, full screen, toasts, the tray icon and notifications -----
//
// A view can open second windows (`window settingsOpen { … }`), each a real window of the
// computer's with a Ui of its own: its own focus, Tab order, shortcuts, timers and toasts. They
// share the view's one maker, so a field changed in one window draws them all again. The main
// window's tree of nodes holds every `window`, and says which are open.
//
// Each window keeps its own tabs, branches and splits (Kept), swapped in while it's being used,
// and dark mode is set for whichever window is drawn.
//
// Toasts are small messages in a window's bottom-right corner (`toast "Saved";`). The tray icon
// and the computer's own notifications go through tray-icon and the Windows API on Windows, and
// D-Bus on Linux; on other systems they do nothing.

thread_local! {
    /// Toasts made by Lust code that no window has shown yet: the window that ran the code takes
    /// them when it's drawn again.
    static NEW_TOASTS: std::cell::RefCell<Vec<(String, Vec<(String, SV)>)>> = const { std::cell::RefCell::new(Vec::new()) };
    /// App:quit() was called while a window was open.
    static QUIT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// How many `App:open()`s are running (windows open).
    static OPEN: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// `toast "Saved" [green];`: shown by the window whose code made it. With no window open, there's
/// nowhere to show it, so it's let go.
pub fn toast(text: String, style: Vec<(String, SV)>) {
    if OPEN.with(|o| o.get()) > 0 {
        NEW_TOASTS.with(|t| t.borrow_mut().push((text, style)));
    }
}

/// `App:quit()`: every window closes, so `App:open()` returns and the program goes on to its end,
/// as when the main window's X is clicked. With no window open, there's nothing to close.
pub fn quit() {
    if OPEN.with(|o| o.get()) > 0 {
        QUIT.with(|q| q.set(true));
    }
}

fn quitting() -> bool {
    QUIT.with(|q| q.get())
}

/// `App:notify("Title", "Body")`: the computer's own notification. A view's tests print it.
pub fn notify(title: String, body: String) {
    if std::env::var("LUST_VIEW_TEST").is_ok() {
        println!("notify: {title} — {body}");
        return;
    }
    os_notify(&title, &body);
}

/// Opens the view's windows, in a test or for real, until the main one closes.
fn windows_open<'a, 'b>(s: Settings, shared: &'a Shared<'a, 'b>, step: Box<Step>) -> Result<(), String> {
    OPEN.with(|o| o.set(o.get() + 1));
    QUIT.with(|q| q.set(false));
    let r = if std::env::var("LUST_VIEW_TEST").is_ok() { test(s, shared, step) } else { windowed::run(s, shared, step) };
    OPEN.with(|o| o.set(o.get().saturating_sub(1)));
    QUIT.with(|q| q.set(false));
    NEW_TOASTS.with(|t| t.borrow_mut().clear());
    r
}

fn epoch() -> u64 {
    EPOCH.with(|e| e.get())
}

// ----- toasts -----

/// A toast: its words and styles, when it showed (on its window's clock), and for how long.
#[derive(Clone, Debug)]
struct Toast {
    text: String,
    style: Vec<(String, SV)>,
    start: f64,
    secs: f64,
}

/// How long a toast takes to fade in and out, and how many show at once.
const TOAST_IN: f64 = 0.15;
const TOAST_OUT: f64 = 0.3;
const TOAST_MAX: usize = 5;
const TOAST_SECS: f64 = 4.0;

impl Toast {
    fn end(&self) -> f64 {
        self.start + self.secs
    }

    /// How solid it is now: it fades in (in a window, not a test, whose clock stands still), and
    /// out at the end.
    fn solid(&self, now: f64, fade_in: bool) -> f32 {
        let a = if fade_in { ((now - self.start) / TOAST_IN).clamp(0.0, 1.0) } else { 1.0 };
        let b = ((self.end() - now) / TOAST_OUT).clamp(0.0, 1.0);
        a.min(b) as f32
    }
}

/// A toast's box: an icon if it has one, and its words, white on dark grey, or on its colour.
fn toast_box(t: &Toast, solid: f32) -> Laid {
    let mut row = Node::new("row");
    row.style = vec![
        ("padY".into(), SV::Num(10.0)),
        ("padX".into(), SV::Num(14.0)),
        ("gap".into(), SV::Num(8.0)),
        ("round".into(), SV::Num(8.0)),
        ("shadow".into(), SV::Flag),
        ("maxWidth".into(), SV::Num(320.0)),
    ];
    let icon = t.style.iter().find_map(|(k, v)| match (k.as_str(), v) {
        ("icon", SV::Text(i)) => Some(i.clone()),
        _ => None,
    });
    if let Some(i) = icon {
        let mut n = Node::new("icon");
        n.text = i;
        n.style = vec![("size".into(), SV::Num(16.0))];
        row.children.push(n);
    }
    let mut words = Node::new("text");
    words.text = t.text.clone();
    row.children.push(words);
    let mut b = lay(&row);
    // A colour word colours the toast, as it colours a button.
    let word = t.style.iter().find_map(|(k, v)| match v {
        SV::Flag if colour(k).is_some() => Some(k.clone()),
        _ => None,
    });
    let (bg, ink) = match word.and_then(|w| bg_colour(&w).or_else(|| colour(&w))) {
        Some(c) => (c, if light(c) { INK } else { 0xffffff }),
        None if dark() => (0x3f434a, 0xffffff),
        None => (0x212529, 0xffffff),
    };
    b.look.background = Some(bg);
    b.look.opacity = solid;
    for c in &mut b.children {
        c.look.color = ink;
    }
    b
}

impl<'a, 'b> Ui<'a, 'b> {
    /// Takes the toasts the code that just ran made.
    fn take_toasts(&mut self) {
        let new = NEW_TOASTS.with(|t| std::mem::take(&mut *t.borrow_mut()));
        let now = self.now();
        for (text, style) in new {
            let secs = style
                .iter()
                .find_map(|(k, v)| match (k.as_str(), v) {
                    ("seconds", SV::Num(n)) => Some(*n),
                    _ => None,
                })
                .unwrap_or(TOAST_SECS)
                .max(0.1);
            self.toasts.push(Toast { text, style, start: now, secs });
        }
        let extra = self.toasts.len().saturating_sub(TOAST_MAX);
        self.toasts.drain(..extra);
    }

    /// The toasts, newest at the bottom right, older ones above it.
    fn toast_layer(&self, w: f32, h: f32) -> Option<Laid> {
        if self.toasts.is_empty() {
            return None;
        }
        let now = self.now();
        let mut boxes = Vec::new();
        let mut y = h - 16.0;
        for t in self.toasts.iter().rev() {
            let mut b = toast_box(t, t.solid(now, self.test_clock.is_none()));
            MEASURED.with(|m| m.borrow_mut().clear());
            let room = (w - 32.0).max(40.0);
            let (bw, bh) = natural(&b, room.min(320.0));
            let (bw, bh) = (bw.ceil().min(room), bh.ceil());
            y -= bh;
            place(&mut b, (w - 16.0 - bw).round(), y.round(), bw, bh);
            MEASURED.with(|m| m.borrow_mut().clear());
            b.kind = "toast".into();
            b.text = t.text.clone();
            y -= 8.0;
            boxes.push(b);
        }
        boxes.reverse();
        let mut l = lay(&Node::new("layer"));
        l.text = "toast".into();
        l.w = w;
        l.h = h;
        l.children = boxes;
        Some(l)
    }

    /// A click on a toast puts it away.
    fn toast_click(&mut self, x: f32, y: f32) -> bool {
        let Some(l) = self.tree.children.iter().rfind(|c| c.kind == "layer" && c.text == "toast") else { return false };
        let Some(i) = l.children.iter().position(|b| x >= b.x && y >= b.y && x < b.x + b.w && y < b.y + b.h) else { return false };
        if i < self.toasts.len() {
            self.toasts.remove(i);
            self.relayout();
        }
        true
    }

    /// Forgets the toasts whose time is up. Gives back whether any went or are fading, so the
    /// window draws again.
    fn step_toasts(&mut self) -> bool {
        if self.toasts.is_empty() {
            return false;
        }
        let now = self.now();
        let before = self.toasts.len();
        self.toasts.retain(|t| t.end() > now + 1e-9);
        let fading = self.toasts.iter().any(|t| now < t.start + TOAST_IN || now > t.end() - TOAST_OUT);
        if self.toasts.len() != before || fading {
            self.relayout();
            return true;
        }
        false
    }

    /// When the toasts next change, on the window's clock: a frame away while one fades.
    fn toast_wake(&self) -> Option<f64> {
        let now = self.now();
        self.toasts
            .iter()
            .map(|t| {
                if now < t.start + TOAST_IN || now > t.end() - TOAST_OUT {
                    now + 1.0 / 60.0
                } else {
                    t.end() - TOAST_OUT
                }
            })
            .fold(None, |m: Option<f64>, d| Some(m.map_or(d, |m| m.min(d))))
    }
}

// ----- second windows -----

/// Every `window` in the view's tree, wherever it's written: what it's known by (where it's
/// written, and its turn there), its place in the tree, and whether its bool is true.
fn window_places(n: &Node, path: &mut Vec<usize>, turns: &mut HashMap<String, usize>, out: &mut Vec<(String, Vec<usize>, bool)>) {
    for (i, c) in n.children.iter().enumerate() {
        path.push(i);
        if c.kind == "window" {
            let t = turns.entry(c.text.clone()).or_insert(0);
            *t += 1;
            out.push((format!("{}#{}", c.text, t), path.clone(), matches!(c.value, Some(SV::Truth(true)))));
        }
        window_places(c, path, turns, out);
        path.pop();
    }
}

fn open_windows(root: &Node) -> Vec<(String, Vec<usize>)> {
    let mut all = Vec::new();
    window_places(root, &mut vec![], &mut HashMap::new(), &mut all);
    all.into_iter().filter(|(_, _, open)| *open).map(|(k, p, _)| (k, p)).collect()
}

fn take_at(mut n: Node, path: &[usize]) -> Option<Node> {
    match path.split_first() {
        None => Some(n),
        Some((&i, rest)) => {
            if i >= n.children.len() {
                return None;
            }
            let c = n.children.swap_remove(i);
            take_at(c, rest)
        }
    }
}

/// A second window's own view, out of the whole view's tree: its pieces, under a root holding
/// its settings. Dark mode and the font come from the main view unless it has its own. The X
/// sets its bool to false (the root's "shut").
fn window_root(root: Node, key: &str) -> Node {
    let mut out = Node::new("view");
    let main_style = root.style.clone();
    let mut places = Vec::new();
    window_places(&root, &mut vec![], &mut HashMap::new(), &mut places);
    let Some(w) = places.into_iter().find(|(k, _, _)| k == key).and_then(|(_, p, _)| take_at(root, &p)) else {
        return out;
    };
    out.style = w.style;
    for inherited in ["dark", "font"] {
        if !out.style.iter().any(|(k, _)| k == inherited) {
            if let Some(d) = main_style.iter().find(|(k, _)| k == inherited) {
                out.style.push(d.clone());
            }
        }
    }
    out.children = w.children;
    out.mouse = w.mouse;
    if let Some(a) = w.action {
        out.mouse.push(("shut".into(), a));
    }
    out
}

/// The main view's first tray, wherever it's written.
fn find_tray(n: &Node) -> Option<&Node> {
    if n.kind == "tray" {
        return Some(n);
    }
    n.children.iter().find_map(find_tray)
}

/// A tray's items, in order, with those from loops and questions.
fn tray_items<'n>(n: &'n Node, out: &mut Vec<&'n Node>) {
    for c in &n.children {
        if c.kind == "item" || c.kind == "divider" {
            out.push(c);
        } else {
            tray_items(c, out);
        }
    }
}

fn item_off(n: &Node) -> bool {
    n.style.iter().any(|(k, v)| k == "disabled" && !matches!(v, SV::Truth(false)))
}

fn tray_tip(n: &Node) -> String {
    n.style
        .iter()
        .find_map(|(k, v)| match (k.as_str(), v) {
            ("tip", SV::Text(t)) => Some(t.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// One open window: what it's known by ("" for the main one), its Ui, and its tabs, branches
/// and splits while another window is being used.
struct Win<'a, 'b> {
    key: String,
    ui: Ui<'a, 'b>,
    kept: Kept,
}

/// The view's open windows, the main one first.
struct Wins<'a, 'b> {
    list: Vec<Win<'a, 'b>>,
    shared: &'a Shared<'a, 'b>,
    windowed: bool,
    /// The last time Lust code ran, as the windows were last drawn.
    epoch: u64,
    /// The main window closed, or App:quit() was called.
    ended: bool,
}

impl<'a, 'b> Wins<'a, 'b> {
    fn new(s: Settings, shared: &'a Shared<'a, 'b>, windowed: bool, step: Box<Step>) -> Result<Wins<'a, 'b>, String> {
        let build: Box<Build<'a>> = Box::new(move || {
            let mut b = shared.borrow_mut();
            (&mut **b)()
        });
        let ui = Ui::new(s, build, windowed, step)?;
        let mut kept = Kept::default();
        KEPT.with(|k| std::mem::swap(&mut *k.borrow_mut(), &mut kept));
        let mut wins = Wins { list: vec![Win { key: String::new(), ui, kept }], shared, windowed, epoch: epoch(), ended: false };
        wins.sync(0)?;
        Ok(wins)
    }

    fn index(&self, key: &str) -> Option<usize> {
        self.list.iter().position(|w| w.key == key)
    }

    /// Makes window `i` the one being used: its tabs, branches and splits, and its dark mode.
    fn enter(&mut self, i: usize) {
        let w = &mut self.list[i];
        DARK.with(|d| d.set(w.ui.s.dark));
        KEPT.with(|k| std::mem::swap(&mut *k.borrow_mut(), &mut w.kept));
    }

    fn leave(&mut self, i: usize) {
        let w = &mut self.list[i];
        KEPT.with(|k| std::mem::swap(&mut *k.borrow_mut(), &mut w.kept));
    }

    fn with<R>(&mut self, i: usize, f: impl FnOnce(&mut Ui<'a, 'b>) -> R) -> R {
        self.enter(i);
        let r = f(&mut self.list[i].ui);
        self.leave(i);
        r
    }

    fn rebuild_all(&mut self) -> Result<(), String> {
        for i in 0..self.list.len() {
            self.with(i, |u| u.rebuild())?;
        }
        Ok(())
    }

    /// Runs one of window `i`'s actions, by its name on the window's root (close, shut,
    /// fullscreen), with a value, and draws the window again.
    fn run_root(&mut self, i: usize, name: &str, value: SV) -> Result<bool, String> {
        let Some(a) = self.list[i].ui.node.mouse.iter().find(|(k, _)| k == name).map(|x| x.1) else { return Ok(false) };
        self.with(i, |u| {
            run_with(&mut u.actions[a], value)?;
            u.rebuild()
        })?;
        Ok(true)
    }

    /// The X of window `i`: a second window's bool becomes false; the main window runs its
    /// `close { }` if it has one, or else ends.
    fn close(&mut self, i: usize) -> Result<(), String> {
        if i > 0 {
            self.run_root(i, "shut", SV::Truth(false))?;
        } else if !self.run_root(0, "close", SV::Flag)? {
            self.ended = true;
        }
        Ok(())
    }

    /// After something happened in window `i`: the others draw again if Lust code ran, toasts no
    /// window took go to the main one, windows Escape asked to close close, and second windows
    /// open and close to match their bools.
    fn sync(&mut self, from: usize) -> Result<(), String> {
        for _ in 0..8 {
            if quitting() {
                self.ended = true;
                return Ok(());
            }
            if epoch() != self.epoch {
                for j in 0..self.list.len() {
                    if j != from {
                        self.with(j, |u| u.rebuild())?;
                    }
                }
                self.epoch = epoch();
            }
            if NEW_TOASTS.with(|t| !t.borrow().is_empty()) {
                self.with(0, |u| {
                    u.take_toasts();
                    u.relayout();
                });
            }
            let asked = (1..self.list.len()).find(|&j| self.list[j].ui.close_asked);
            match asked {
                Some(j) => {
                    self.list[j].ui.close_asked = false;
                    self.run_root(j, "shut", SV::Truth(false))?;
                }
                None => break,
            }
        }
        self.list[0].ui.close_asked = false;
        let open = open_windows(&self.list[0].ui.node);
        self.list.retain(|w| w.key.is_empty() || open.iter().any(|(k, _)| *k == w.key));
        for (key, path) in open {
            if self.index(&key).is_none() {
                self.open(key, &path)?;
            }
        }
        Ok(())
    }

    /// Opens the second window known by `key`, written at `path` in the main window's tree.
    fn open(&mut self, key: String, path: &[usize]) -> Result<(), String> {
        let style = {
            let mut n = &self.list[0].ui.node;
            for &i in path {
                n = &n.children[i];
            }
            let mut st = n.style.clone();
            if !st.iter().any(|(k, _)| k == "dark") {
                if let Some(d) = self.list[0].ui.node.style.iter().find(|(k, _)| k == "dark") {
                    st.push(d.clone());
                }
            }
            st
        };
        let s = settings(&style);
        let shared = self.shared;
        let k = key.clone();
        let build: Box<Build<'a>> = Box::new(move || {
            let (root, actions) = {
                let mut b = shared.borrow_mut();
                (&mut **b)()?
            };
            Ok((window_root(root, &k), actions))
        });
        // Only the main window moves the tweens on.
        let step: Box<Step> = Box::new(|_| Ok(false));
        let mut ui = Ui::new(s, build, self.windowed, step)?;
        // A test's windows share one clock.
        if let Some(now) = self.list[0].ui.test_clock {
            ui.test_clock = Some(now);
            for t in &mut ui.timers {
                t.due = now + t.secs;
            }
        }
        let mut kept = Kept::default();
        KEPT.with(|k| std::mem::swap(&mut *k.borrow_mut(), &mut kept));
        self.list.push(Win { key, ui, kept });
        Ok(())
    }

    /// `in Settings click Save`: which window a test's command is for, and the command.
    fn target(&self, line: &str) -> Result<(usize, String), String> {
        let Some(rest) = line.strip_prefix("in ") else { return Ok((0, line.to_string())) };
        let rest = rest.trim_start();
        let (title, cmd) = match rest.strip_prefix('"') {
            Some(q) => match q.split_once('"') {
                Some((t, c)) => (t.to_string(), c.trim().to_string()),
                None => (q.to_string(), String::new()),
            },
            None => {
                let (t, c) = rest.split_once(' ').unwrap_or((rest, ""));
                (t.to_string(), c.trim().to_string())
            }
        };
        match self.list.iter().position(|w| w.ui.s.title == title) {
            Some(i) => Ok((i, cmd)),
            None => Err(format!("(there's no open window \"{title}\")")),
        }
    }

    /// What `dump` shows: each open window under a heading, and the tray.
    fn dump(&mut self) -> String {
        let mut out = String::new();
        for i in 0..self.list.len() {
            let ui = &self.list[i].ui;
            let mut flags = String::new();
            if ui.s.fullscreen {
                flags.push_str(" (fullscreen)");
            }
            if !ui.s.visible {
                flags.push_str(" (hidden)");
            }
            // At a scale, the pieces are where they'd be at any other: only the pixels differ.
            if let Some(k) = TEST_SCALE.with(|t| t.get()) {
                flags.push_str(&format!(" at scale {} ({}x{} pixels)", num_text(k as f64), (ui.w * k).round() as i64, (ui.h * k).round() as i64));
            }
            out.push_str(&format!("window {:?} {}x{}{flags}\n", ui.s.title, ui.w as i64, ui.h as i64));
            let st = ui.paint_state();
            for (j, c) in ui.tree.children.iter().enumerate() {
                dump(c, 1, &mut vec![j], &st, &mut out);
            }
            out.push_str(&ui.mouse_dump_lines());
            if i == 0 {
                if let Some(t) = find_tray(&ui.node) {
                    let mut items = Vec::new();
                    tray_items(t, &mut items);
                    let n = items.iter().filter(|i| i.kind == "item").count();
                    out.push_str(&format!("tray {:?} ({n} item{})\n", tray_tip(t), if n == 1 { "" } else { "s" }));
                }
            }
        }
        out
    }

    /// `tray click` and `tray item Quit`: the tray icon, as if clicked.
    fn tray(&mut self, what: &str) -> Result<(), String> {
        let Some(t) = find_tray(&self.list[0].ui.node) else {
            println!("(there's no tray)");
            return Ok(());
        };
        let action = if what == "click" {
            t.mouse.iter().find(|(k, _)| k == "click").map(|x| x.1)
        } else if let Some(name) = what.strip_prefix("item ") {
            let name = name.trim().trim_matches('"');
            let mut items = Vec::new();
            tray_items(t, &mut items);
            match items.iter().find(|i| i.kind == "item" && i.text == name) {
                Some(i) if item_off(i) => {
                    println!("(the tray's item \"{name}\" is disabled)");
                    return Ok(());
                }
                Some(i) => i.action,
                None => {
                    println!("(the tray has no item \"{name}\")");
                    return Ok(());
                }
            }
        } else {
            println!("(tray takes click, or item and its words)");
            return Ok(());
        };
        self.run_tray(action)
    }

    /// Runs a tray's block in the main window.
    fn run_tray(&mut self, action: Option<usize>) -> Result<(), String> {
        let Some(a) = action else { return Ok(()) };
        self.with(0, |u| {
            run_with(&mut u.actions[a], SV::Flag)?;
            u.rebuild()
        })?;
        self.sync(0)
    }

    /// `wait 2.5` in a test: the windows' shared clock moves on, running each window's timers and
    /// the tweens as they come due, in the order they come (a timer first, when both are due
    /// together). Toasts whose time is up go.
    fn wait(&mut self, secs: f64) -> Result<(), String> {
        let end = self.list[0].ui.now() + secs;
        loop {
            let mut best: Option<(f64, usize, bool)> = None;
            for (i, w) in self.list.iter().enumerate() {
                if let Some(d) = w.ui.next_timer() {
                    if best.is_none_or(|(b, _, _)| d < b) {
                        best = Some((d, i, false));
                    }
                }
            }
            if let Some(d) = self.list[0].ui.next_step() {
                if best.is_none_or(|(b, _, _)| d < b) {
                    best = Some((d, 0, true));
                }
            }
            let Some((d, i, step)) = best.filter(|(d, _, _)| *d <= end + 1e-9) else { break };
            for w in &mut self.list {
                w.ui.test_clock = Some(d.max(w.ui.now()));
            }
            self.with(i, |u| if step { u.step_tweens() } else { u.fire_timers().map(|_| ()) })?;
            self.sync(i)?;
            if self.ended {
                return Ok(());
            }
        }
        for i in 0..self.list.len() {
            self.list[i].ui.test_clock = Some(end);
            self.with(i, |u| u.step_toasts());
        }
        Ok(())
    }

    /// After a test's command in window `i`.
    fn after(&mut self, i: usize) -> Result<(), String> {
        self.sync(i.min(self.list.len().saturating_sub(1)))
    }
}

// ----- the windows themselves -----

mod windowed {
    use super::*;
    use std::num::NonZeroU32;
    use std::rc::Rc;
    use winit::application::ApplicationHandler;
    use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, StartCause, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::keyboard::{Key as WKey, ModifiersState, NamedKey};
    use winit::window::{CursorIcon, Fullscreen, Window, WindowId};

    /// A window of the computer's, and what it was last told.
    struct Os {
        window: Rc<Window>,
        surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
        mouse: Option<(f32, f32)>,
        /// Files dropped on the window: winit tells of them one at a time, and they're handed
        /// over together once it has told of them all.
        dropped: Vec<String>,
        title: String,
        full: bool,
        shown: bool,
        /// Whether the computer's input method is on (while something that takes typing has
        /// focus), and where it was last told the caret is.
        ime: bool,
        ime_at: Option<[f32; 4]>,
        /// The window's link to screen readers, the tree it was last sent (and what has focus),
        /// and each node as it was sent, so only nodes that changed are sent again.
        access: accesskit_winit::Adapter,
        sent: Option<(Vec<ANode>, u64)>,
        kit_sent: HashMap<u64, accesskit::Node>,
    }

    impl Os {
        fn logical(&self) -> (f32, f32, f32) {
            let scale = self.window.scale_factor() as f32;
            let size = self.window.inner_size();
            (size.width as f32 / scale, size.height as f32 / scale, scale)
        }
    }

    struct App<'a, 'b> {
        wins: Wins<'a, 'b>,
        os: std::collections::HashMap<String, Os>,
        mods: ModifiersState,
        error: Option<String>,
        tray: tray::Tray,
    }

    fn mods_of(m: ModifiersState) -> Mods {
        if cfg!(target_os = "macos") {
            Mods { shift: m.shift_key(), command: m.super_key(), word: m.alt_key(), alt: m.alt_key() }
        } else {
            Mods { shift: m.shift_key(), command: m.control_key(), word: m.control_key(), alt: m.alt_key() }
        }
    }

    fn fail(e: impl std::fmt::Display, what: &str) -> String {
        format!("{what}: {e}.")
    }

    fn open_os(el: &ActiveEventLoop, s: &Settings, main: bool) -> Result<Os, String> {
        let shown = !main || s.visible;
        // Sizes are logical: winit makes the window that many pixels times the screen's scale.
        // It opens hidden, so screen readers can be linked to it before it shows.
        let attrs = Window::default_attributes()
            .with_title(s.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(s.width, s.height))
            .with_min_inner_size(winit::dpi::LogicalSize::new(s.min_width.max(1.0), s.min_height.max(1.0)))
            .with_visible(false)
            .with_fullscreen(s.fullscreen.then_some(Fullscreen::Borderless(None)));
        let window = Rc::new(el.create_window(attrs).map_err(|e| fail(e, "can't open a window"))?);
        let access = kit::adapter(el, &window);
        if shown {
            window.set_visible(true);
        }
        let surface = softbuffer::Context::new(window.clone())
            .and_then(|ctx| softbuffer::Surface::new(&ctx, window.clone()))
            .map_err(|e| fail(e, "can't draw in the window"))?;
        Ok(Os {
            window,
            surface,
            mouse: None,
            dropped: vec![],
            title: s.title.clone(),
            full: s.fullscreen,
            shown,
            ime: false,
            ime_at: None,
            access,
            sent: None,
            kit_sent: HashMap::new(),
        })
    }

    impl<'a, 'b> App<'a, 'b> {
        fn key_of(&self, id: WindowId) -> Option<String> {
            self.os.iter().find(|(_, o)| o.window.id() == id).map(|(k, _)| k.clone())
        }

        fn redraw(&self, key: &str) {
            if let Some(o) = self.os.get(key) {
                o.window.request_redraw();
            }
        }

        fn redraw_all(&self) {
            for o in self.os.values() {
                o.window.request_redraw();
            }
        }

        fn stop(&mut self, e: String, el: &ActiveEventLoop) {
            self.error = Some(e);
            el.exit();
        }

        /// The computer's windows match the view's: second windows open and close, and titles,
        /// full screen and hiding follow the data.
        fn reconcile(&mut self, el: &ActiveEventLoop) {
            let keys: Vec<String> = self.wins.list.iter().map(|w| w.key.clone()).collect();
            self.os.retain(|k, _| keys.contains(k));
            for i in 0..self.wins.list.len() {
                let key = self.wins.list[i].key.clone();
                if !self.os.contains_key(&key) {
                    match open_os(el, &self.wins.list[i].ui.s, i == 0) {
                        Ok(o) => {
                            let (w, h, _) = o.logical();
                            self.os.insert(key.clone(), o);
                            if let Err(e) = self.wins.with(i, |u| u.resize(w, h)) {
                                return self.stop(e, el);
                            }
                        }
                        Err(e) => return self.stop(e, el),
                    }
                }
                let s = &self.wins.list[i].ui.s;
                let Some(o) = self.os.get_mut(&key) else { continue };
                if o.title != s.title {
                    o.title = s.title.clone();
                    o.window.set_title(&s.title);
                }
                if o.full != s.fullscreen {
                    o.full = s.fullscreen;
                    o.window.set_fullscreen(s.fullscreen.then_some(Fullscreen::Borderless(None)));
                }
                let shown = i > 0 || s.visible;
                if o.shown != shown {
                    o.shown = shown;
                    o.window.set_visible(shown);
                }
            }
        }

        /// After something happened in window `i` (`draw`: it should be drawn again).
        fn after(&mut self, el: &ActiveEventLoop, key: &str, r: Result<(), String>, draw: bool) {
            if let Err(e) = r {
                return self.stop(e, el);
            }
            let ran = self.wins.epoch != epoch();
            let i = self.wins.index(key).unwrap_or(0);
            if let Err(e) = self.wins.sync(i) {
                return self.stop(e, el);
            }
            if self.wins.ended {
                el.exit();
                return;
            }
            self.reconcile(el);
            self.tray.sync(&self.wins.list[0].ui.node);
            if ran {
                self.redraw_all();
            } else if draw {
                self.redraw(key);
            }
        }

        /// What happened in window `i`, already made the one being used. Gives back whether it
        /// should be drawn again.
        fn handle(&mut self, i: usize, key: &str, event: WindowEvent) -> Result<bool, String> {
            let mods = mods_of(self.mods);
            let App { wins, os, .. } = self;
            let ui = &mut wins.list[i].ui;
            let Some(o) = os.get_mut(key) else { return Ok(false) };
            match event {
                // Moved to a screen with another scale (or the scale changed): winit keeps the
                // window's logical size, so the view is laid out the same and drawn again sharp
                // at the new scale (letters, icons and SVG pictures are drawn again for it).
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    let (w, h, _) = o.logical();
                    ui.resize(w, h)?;
                    // Full screen left (or taken) by the computer's own means changes the field.
                    let full = o.window.fullscreen().is_some();
                    if full != o.full {
                        o.full = full;
                        ui.s.fullscreen = full;
                        if let Some(a) = ui.node.mouse.iter().find(|(k, _)| k == "fullscreen").map(|x| x.1) {
                            run_with(&mut ui.actions[a], SV::Truth(full))?;
                            ui.rebuild()?;
                        }
                    }
                    Ok(true)
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let (_, _, scale) = o.logical();
                    let p = (position.x as f32 / scale, position.y as f32 / scale);
                    o.mouse = Some(p);
                    let before = (ui.hovered(), ui.pointer(p.0, p.1));
                    let ran = ui.ran;
                    let r = ui.moved(p.0, p.1);
                    let dragging = ui.drag != Drag::None || ui.ran != ran;
                    let tip = ui.track_tip();
                    o.window.set_cursor(match ui.pointer(p.0, p.1) {
                        2 => CursorIcon::Text,
                        3 => CursorIcon::ColResize,
                        4 => CursorIcon::RowResize,
                        5 => CursorIcon::Crosshair,
                        1 => CursorIcon::Pointer,
                        _ => CursorIcon::Default,
                    });
                    r?;
                    // Only draw again when something under the mouse changed, or while dragging.
                    Ok(tip || dragging || before != (ui.hovered(), ui.pointer(p.0, p.1)) || ui.pressed.is_some())
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let Some((x, y)) = o.mouse else { return Ok(false) };
                    let (_, _, scale) = o.logical();
                    let dy = match delta {
                        MouseScrollDelta::LineDelta(_, lines) => -lines * 48.0,
                        MouseScrollDelta::PixelDelta(p) => -(p.y as f32) / scale,
                    };
                    // The wheel over a number box moves its number.
                    if let Some(r) = ui.wheel_number(x, y, dy) {
                        r?;
                        return Ok(true);
                    }
                    // The wheel over a table or list scrolls its rows.
                    if let Some(r) = ui.wheel_list(x, y, dy) {
                        let _ = ui.moved(x, y);
                        r?;
                        return Ok(true);
                    }
                    if ui.wheel(x, y, dy) {
                        let _ = ui.moved(x, y);
                        return Ok(true);
                    }
                    Ok(false)
                }
                WindowEvent::CursorLeft { .. } => {
                    o.mouse = None;
                    ui.st.hover = None;
                    ui.track_tip();
                    Ok(true)
                }
                WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                    let Some((x, y)) = o.mouse else { return Ok(false) };
                    match state {
                        ElementState::Pressed => ui.press(x, y, mods)?,
                        ElementState::Released => ui.release(x, y)?,
                    }
                    Ok(true)
                }
                // The right button picks the row under it when it goes down, and opens the
                // right-click menu when it's let go, as on Windows.
                WindowEvent::MouseInput { state, button: MouseButton::Right, .. } => {
                    let Some((x, y)) = o.mouse else { return Ok(false) };
                    match state {
                        ElementState::Pressed => ui.right_press(x, y)?,
                        ElementState::Released => ui.right_release(x, y)?,
                    }
                    Ok(true)
                }
                WindowEvent::HoveredFile(_) => {
                    let was = ui.files_over;
                    ui.files_over = true;
                    Ok(!was)
                }
                WindowEvent::HoveredFileCancelled => {
                    ui.files_over = false;
                    Ok(true)
                }
                WindowEvent::DroppedFile(path) => {
                    ui.files_over = false;
                    o.dropped.push(path.to_string_lossy().into_owned());
                    Ok(false)
                }
                WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                    key_pressed(ui, event, mods)?;
                    Ok(true)
                }
                // The input method's text being put together, and the text it finished.
                WindowEvent::Ime(Ime::Preedit(text, cursor)) => {
                    ui.ime_preedit(text, cursor);
                    Ok(true)
                }
                WindowEvent::Ime(Ime::Commit(text)) => {
                    ui.ime_commit(&text)?;
                    Ok(true)
                }
                WindowEvent::Ime(Ime::Disabled) => {
                    ui.st.preedit = None;
                    Ok(true)
                }
                _ => Ok(false),
            }
        }

        fn paint(&mut self, el: &ActiveEventLoop, key: &str) {
            let Some(i) = self.wins.index(key) else { return };
            self.wins.enter(i);
            let r = self.paint_in(i, key);
            self.wins.leave(i);
            if let Err(e) = r {
                self.stop(e, el);
            }
        }

        fn paint_in(&mut self, i: usize, key: &str) -> Result<(), String> {
            let App { wins, os, .. } = self;
            let ui = &mut wins.list[i].ui;
            let Some(o) = os.get_mut(key) else { return Ok(()) };
            // A table that came into view (a tab shown, a branch opened) makes its rows.
            if take_stale() {
                ui.rebuild()?;
            }
            let (lw, lh, scale) = o.logical();
            let size = o.window.inner_size();
            let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return Ok(()) };
            if o.surface.resize(w, h).is_err() {
                return Ok(());
            }
            if (ui.tree.w - lw).abs() > 0.5 || (ui.tree.h - lh).abs() > 0.5 || ui.step_fades() || ui.moving() {
                o.window.request_redraw();
            }
            let (st, bg) = (ui.paint_state(), ui.s.background);
            let c = render(&ui.shown(), size.width as usize, size.height as usize, scale, &st, bg);
            if let Ok(mut buf) = o.surface.buffer_mut() {
                let n = buf.len().min(c.px.len());
                buf[..n].copy_from_slice(&c.px[..n]);
                let _ = buf.present();
            }
            // The input method is on while something that takes typing has focus, and knows
            // where the caret is, so its list of choices shows beside it.
            let typing = ui.takes_typing();
            if typing != o.ime {
                o.ime = typing;
                o.ime_at = None;
                o.window.set_ime_allowed(typing);
                if !typing {
                    ui.st.preedit = None;
                }
            }
            if typing {
                let at = ui.caret_area();
                if at.is_some() && at != o.ime_at {
                    o.ime_at = at;
                    if let Some([x, y, w, h]) = at {
                        o.window.set_ime_cursor_area(winit::dpi::LogicalPosition::new(x, y), winit::dpi::LogicalSize::new(w, h));
                    }
                }
            }
            kit::send(o, ui, scale);
            Ok(())
        }
    }

    fn key_pressed(ui: &mut Ui, event: winit::event::KeyEvent, mods: Mods) -> Result<(), String> {
        // While an input method is putting text together, Enter, Escape, Backspace and the
        // arrows are its own: they don't reach the view's shortcuts, enter { } or the field.
        if ui.composing() {
            return Ok(());
        }
        // The view's shortcuts go first, unless what has focus uses the key.
        if let Some(name) = key_name(&event) {
            if ui.shortcut(&name, mods)? {
                return Ok(());
            }
        }
        let input = editable(&ui.focused_kind()) || ui.picker_takes_text();
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
                    return ui.nav(n, mods);
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
            // Cmd+Up and Cmd+Down go to the start and end of a text box's text on a Mac.
            WKey::Named(NamedKey::ArrowUp) if mac_line => Some(Nav::Edit(Key::Home)),
            WKey::Named(NamedKey::ArrowDown) if mac_line => Some(Nav::Edit(Key::End)),
            WKey::Named(NamedKey::ArrowUp) => Some(Nav::Up),
            WKey::Named(NamedKey::ArrowDown) => Some(Nav::Down),
            WKey::Named(NamedKey::PageUp) => Some(Nav::PageUp),
            WKey::Named(NamedKey::PageDown) => Some(Nav::PageDown),
            WKey::Named(NamedKey::Tab) => Some(Nav::Tab),
            WKey::Named(NamedKey::Enter) => Some(Nav::Enter),
            WKey::Named(NamedKey::Escape) => Some(Nav::Escape),
            WKey::Named(NamedKey::Space) if !input => Some(Nav::Space),
            // Shift+F10 and the Menu key open the right-click menu of what has focus.
            WKey::Named(NamedKey::F10) if mods.shift => Some(Nav::Menu),
            WKey::Named(NamedKey::ContextMenu) => Some(Nav::Menu),
            _ => None,
        };
        if let Some(n) = nav {
            return ui.nav(n, mods);
        }
        if input && !mods.command {
            if let Some(t) = &event.text {
                if !t.chars().any(|c| c.is_control()) {
                    return ui.typed(t);
                }
            }
        }
        Ok(())
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
            if self.os.is_empty() {
                self.reconcile(el);
                self.tray.sync(&self.wins.list[0].ui.node);
                self.redraw_all();
            }
        }

        fn new_events(&mut self, el: &ActiveEventLoop, cause: StartCause) {
            if !matches!(cause, StartCause::ResumeTimeReached { .. }) || self.error.is_some() {
                return;
            }
            let keys: Vec<String> = self.wins.list.iter().map(|w| w.key.clone()).collect();
            for key in keys {
                let Some(i) = self.wins.index(&key) else { continue };
                let ran = epoch();
                self.wins.enter(i);
                let r = (|| -> Result<bool, String> {
                    let ui = &mut self.wins.list[i].ui;
                    let mut draw = false;
                    if ui.tip_due().is_some_and(|t| t <= std::time::Instant::now()) {
                        ui.show_tip();
                        draw = true;
                    }
                    draw |= ui.fire_timers()?;
                    // Tweens move on a frame at a time, and the view is drawn again after each step.
                    let now = ui.now();
                    if ui.next_step().is_some_and(|d| d <= now + 0.001) {
                        ui.step_tweens()?;
                        draw = true;
                    }
                    draw |= ui.step_toasts();
                    Ok(draw)
                })();
                self.wins.leave(i);
                match r {
                    Ok(draw) if draw || epoch() != ran => self.after(el, &key, Ok(()), true),
                    Ok(_) => {}
                    Err(e) => return self.stop(e, el),
                }
                if self.error.is_some() || self.wins.ended {
                    return;
                }
            }
        }

        fn about_to_wait(&mut self, el: &ActiveEventLoop) {
            if self.error.is_some() {
                return;
            }
            // The tray icon was clicked, or one of its items chosen.
            for hit in tray::take() {
                let action = find_tray(&self.wins.list[0].ui.node).and_then(|t| match &hit {
                    tray::Hit::Click => t.mouse.iter().find(|(k, _)| k == "click").map(|x| x.1),
                    tray::Hit::Item(n) => {
                        let mut items = Vec::new();
                        tray_items(t, &mut items);
                        items.get(*n).filter(|i| i.kind == "item" && !item_off(i)).and_then(|i| i.action)
                    }
                });
                let r = self.wins.run_tray(action);
                self.after(el, "", r, true);
                if self.error.is_some() || self.wins.ended {
                    return;
                }
            }
            // What screen readers asked for: a tree, or something done to a piece.
            if kit::take_wanted() {
                for o in self.os.values_mut() {
                    o.sent = None;
                    o.kit_sent.clear();
                    o.window.request_redraw();
                }
            }
            for (id, req) in kit::take_asked() {
                let Some(key) = self.key_of(id) else { continue };
                let Some(i) = self.wins.index(&key) else { continue };
                let Some(o) = self.os.get(&key) else { continue };
                let Some(act) = kit::act_of(&req) else { continue };
                let target = o.sent.as_ref().and_then(|(t, _)| kit::target(t, req.target_node.0));
                let Some(node) = target else { continue };
                let r = self.wins.with(i, |u| u.access_do(&node, act));
                self.after(el, &key, r, true);
                if self.error.is_some() || self.wins.ended {
                    return;
                }
            }
            // All the files of one drop have been told of: the drop's block runs once, with them all.
            let keys: Vec<String> = self.wins.list.iter().map(|w| w.key.clone()).collect();
            let mut soon = false;
            for key in &keys {
                let paths = self.os.get_mut(key).map(|o| std::mem::take(&mut o.dropped)).unwrap_or_default();
                let Some(i) = self.wins.index(key) else { continue };
                if !paths.is_empty() {
                    let r = self.wins.with(i, |u| u.drop_files(paths));
                    self.after(el, key, r, true);
                }
                // A row dragged near the top or bottom of its list scrolls it, even with the mouse still.
                let Some(i) = self.wins.index(key) else { continue };
                if self.wins.list[i].ui.edge_speed() != 0.0 {
                    let r = self.wins.with(i, |u| u.edge_scroll());
                    self.after(el, key, r, true);
                    soon = true;
                }
                if self.error.is_some() || self.wins.ended {
                    return;
                }
            }
            if soon {
                el.set_control_flow(ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(16)));
                return;
            }
            // Wake for the caret's next blink, the next timer, tween step, tip or toast, whichever
            // is soonest, in any window.
            let mut wake: Option<std::time::Instant> = None;
            for w in &self.wins.list {
                let ui = &w.ui;
                let at = |d: Option<f64>| d.map(|d| ui.started + std::time::Duration::from_secs_f64(d.max(0.0)));
                let soonest = [ui.next_blink(), at(ui.next_timer()), at(ui.next_step()), ui.tip_due(), at(ui.toast_wake())].into_iter().flatten().min();
                wake = [wake, soonest].into_iter().flatten().min();
            }
            match wake {
                Some(t) => el.set_control_flow(ControlFlow::WaitUntil(t)),
                None => el.set_control_flow(ControlFlow::Wait),
            }
        }

        fn user_event(&mut self, _: &ActiveEventLoop, _: ()) {
            // The tray woke the loop: about_to_wait runs its blocks.
        }

        fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
            if self.error.is_some() {
                el.exit();
                return;
            }
            let Some(key) = self.key_of(id) else { return };
            let Some(i) = self.wins.index(&key) else { return };
            // Screen readers follow the window's focus and size.
            if let Some(o) = self.os.get_mut(&key) {
                o.access.process_event(&o.window, &event);
            }
            match event {
                WindowEvent::CloseRequested => {
                    let r = self.wins.close(i);
                    self.after(el, &key, r, true);
                }
                WindowEvent::RedrawRequested => self.paint(el, &key),
                WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
                event => {
                    self.wins.enter(i);
                    let r = self.handle(i, &key, event);
                    self.wins.leave(i);
                    match r {
                        Ok(draw) => self.after(el, &key, Ok(()), draw),
                        Err(e) => self.stop(e, el),
                    }
                }
            }
        }
    }

    thread_local! {
        // winit allows one event loop per program, so every window shares it.
        static LOOP: std::cell::RefCell<Option<EventLoop<()>>> = const { std::cell::RefCell::new(None) };
    }

    fn new_loop() -> Result<EventLoop<()>, String> {
        let mut b = EventLoop::builder();
        // Windows: sharp on every screen. Each window is drawn at its own screen's scale and told
        // when it moves to one with another (per-monitor DPI awareness, version 2). winit asks for
        // it unless told not to; this says so here, where it can be seen.
        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::EventLoopBuilderExtWindows;
            b.with_dpi_aware(true);
        }
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

    pub fn run<'a, 'b>(s: Settings, shared: &'a Shared<'a, 'b>, step: Box<super::Step>) -> Result<(), String> {
        use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
        let mut el = match LOOP.with(|l| l.borrow_mut().take()) {
            Some(el) => el,
            None => new_loop()?,
        };
        tray::wake_with(el.create_proxy());
        kit::wake_with(el.create_proxy());
        let wins = Wins::new(s, shared, true, step)?;
        let mut app = App { wins, os: std::collections::HashMap::new(), mods: ModifiersState::default(), error: None, tray: tray::Tray::default() };
        let ran = el.run_app_on_demand(&mut app);
        app.tray = tray::Tray::default();
        app.os.clear();
        LOOP.with(|l| *l.borrow_mut() = Some(el));
        ran.map_err(|e| format!("the window stopped: {e}."))?;
        match app.error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Screen readers, through AccessKit: each window's tree goes to UI Automation on Windows and
    /// AT-SPI on Linux, and what a screen reader asks for comes back here. AccessKit may ask from
    /// another thread, so its requests wait in a queue and wake the event loop.
    mod kit {
        use super::super::*;
        use super::Os;
        use winit::event_loop::ActiveEventLoop;
        use winit::window::Window;
        use accesskit::{Action, ActionData, ActionRequest, NodeId, TreeId, TreeInfo, TreeUpdate};
        use std::collections::HashSet;
        use std::sync::Mutex;
        use winit::window::WindowId;

        static ACTIVE: Mutex<Option<HashSet<WindowId>>> = Mutex::new(None);
        static WANTED: Mutex<bool> = Mutex::new(false);
        static ASKED: Mutex<Vec<(WindowId, ActionRequest)>> = Mutex::new(Vec::new());
        static PROXY: Mutex<Option<winit::event_loop::EventLoopProxy<()>>> = Mutex::new(None);

        pub fn wake_with(p: winit::event_loop::EventLoopProxy<()>) {
            if let Ok(mut q) = PROXY.lock() {
                *q = Some(p);
            }
        }

        fn wake() {
            if let Some(p) = PROXY.lock().ok().and_then(|p| p.clone()) {
                let _ = p.send_event(());
            }
        }

        fn active(id: WindowId, on: bool) {
            if let Ok(mut a) = ACTIVE.lock() {
                let set = a.get_or_insert_with(HashSet::new);
                if on {
                    set.insert(id);
                } else {
                    set.remove(&id);
                }
            }
        }

        fn is_active(id: WindowId) -> bool {
            ACTIVE.lock().ok().is_some_and(|a| a.as_ref().is_some_and(|s| s.contains(&id)))
        }

        struct Activate(WindowId);
        impl accesskit::ActivationHandler for Activate {
            // The tree is sent from the event loop, once it has drawn the window.
            fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
                active(self.0, true);
                if let Ok(mut w) = WANTED.lock() {
                    *w = true;
                }
                wake();
                None
            }
        }

        struct Asked(WindowId);
        impl accesskit::ActionHandler for Asked {
            fn do_action(&mut self, request: ActionRequest) {
                if let Ok(mut q) = ASKED.lock() {
                    q.push((self.0, request));
                }
                wake();
            }
        }

        struct Deactivate(WindowId);
        impl accesskit::DeactivationHandler for Deactivate {
            fn deactivate_accessibility(&mut self) {
                active(self.0, false);
            }
        }

        /// Links a window to screen readers. It must be hidden still.
        pub fn adapter(el: &ActiveEventLoop, window: &Window) -> accesskit_winit::Adapter {
            let id = window.id();
            accesskit_winit::Adapter::with_direct_handlers(el, window, Activate(id), Asked(id), Deactivate(id))
        }

        pub fn take_wanted() -> bool {
            WANTED.lock().map(|mut w| std::mem::take(&mut *w)).unwrap_or(false)
        }

        pub fn take_asked() -> Vec<(WindowId, ActionRequest)> {
            ASKED.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default()
        }

        /// What was asked, as Lust does it.
        pub fn act_of(r: &ActionRequest) -> Option<Act> {
            Some(match r.action {
                Action::Click => Act::Click,
                Action::Focus => Act::Focus,
                Action::ScrollIntoView => Act::Show,
                Action::Expand => Act::Open(true),
                Action::Collapse => Act::Open(false),
                Action::Increment => Act::Step(true),
                Action::Decrement => Act::Step(false),
                Action::SetValue => match &r.data {
                    Some(ActionData::Value(v)) => Act::Set(v.to_string()),
                    Some(ActionData::NumericValue(x)) => Act::Set(num_text(*x)),
                    _ => return None,
                },
                Action::ReplaceSelectedText => match &r.data {
                    Some(ActionData::Value(v)) => Act::Set(v.to_string()),
                    _ => return None,
                },
                _ => return None,
            })
        }

        /// The piece a request is for: a text input's line is its input.
        pub fn target(tree: &[ANode], id: u64) -> Option<ANode> {
            if let Some(a) = tree.iter().find(|a| a.id == id) {
                return Some(a.clone());
            }
            tree.iter().find(|a| (0..a.lines.len()).any(|i| line_id(a.id, i) == id)).cloned()
        }

        fn line_id(input: u64, i: usize) -> u64 {
            child_id(input, (1u64 << 50) + i as u64)
        }

        fn role_of(r: Role) -> accesskit::Role {
            use accesskit::Role as K;
            match r {
                Role::Window => K::Window,
                Role::Label => K::Label,
                Role::Button => K::Button,
                Role::CheckBox => K::CheckBox,
                Role::Switch => K::Switch,
                Role::RadioGroup => K::RadioGroup,
                Role::RadioButton => K::RadioButton,
                Role::Slider => K::Slider,
                Role::SpinButton => K::SpinButton,
                Role::TextInput => K::TextInput,
                Role::TextBox => K::MultilineTextInput,
                Role::ComboBox => K::ComboBox,
                Role::MenuButton => K::Button,
                Role::Menu => K::Menu,
                Role::MenuItem => K::MenuItem,
                Role::ListBox => K::ListBox,
                Role::Choice => K::ListBoxOption,
                Role::Dialog => K::Dialog,
                Role::TabList => K::TabList,
                Role::Tab => K::Tab,
                Role::Table => K::Table,
                Role::ColumnHeader => K::ColumnHeader,
                Role::Row => K::Row,
                Role::Cell => K::Cell,
                Role::List => K::List,
                Role::ListItem => K::ListItem,
                Role::DateInput => K::DateInput,
                Role::ColourWell => K::ColorWell,
                Role::Calendar | Role::ColourChooser => K::Group,
                Role::ProgressBar => K::ProgressIndicator,
                Role::Status => K::Status,
                Role::Image => K::Image,
                Role::Tooltip => K::Tooltip,
                Role::Canvas => K::Canvas,
            }
        }

        /// The tree as AccessKit takes it: every node, with the window's scale on the window, so
        /// the pieces' places stay in the view's units.
        fn update(tree: &[ANode], focus: u64, scale: f32) -> TreeUpdate {
            use accesskit::{Affine, HasPopup, Invalid, Live, Node, Rect, SortDirection, TextPosition, TextSelection, Toggled};
            let mut nodes = Vec::with_capacity(tree.len() + 8);
            for a in tree {
                let mut k = Node::new(role_of(a.role));
                let [x, y, w, h] = a.bounds;
                k.set_bounds(Rect::new(x as f64, y as f64, (x + w) as f64, (y + h) as f64));
                if a.id == ROOT_ID {
                    k.set_transform(Affine::scale(scale as f64));
                }
                if !a.name.is_empty() {
                    k.set_label(a.name.clone());
                }
                let text = matches!(a.role, Role::TextInput | Role::TextBox | Role::SpinButton | Role::DateInput);
                if let Some(v) = &a.value {
                    k.set_value(v.clone());
                }
                if !a.placeholder.is_empty() {
                    k.set_placeholder(a.placeholder.clone());
                }
                if let Some([v, lo, hi, step]) = a.num {
                    k.set_numeric_value(v);
                    if lo.is_finite() && lo.abs() < 1e300 {
                        k.set_min_numeric_value(lo);
                    }
                    if hi.is_finite() && hi.abs() < 1e300 {
                        k.set_max_numeric_value(hi);
                    }
                    if step > 0.0 {
                        k.set_numeric_value_step(step);
                    }
                }
                for s in &a.states {
                    match *s {
                        "disabled" => k.set_disabled(),
                        "checked" => k.set_toggled(Toggled::True),
                        "not checked" => k.set_toggled(Toggled::False),
                        "selected" => k.set_selected(true),
                        "expanded" => k.set_expanded(true),
                        "collapsed" => k.set_expanded(false),
                        "modal" => k.set_modal(),
                        "sorted up" => k.set_sort_direction(SortDirection::Ascending),
                        "sorted down" => k.set_sort_direction(SortDirection::Descending),
                        "invalid" => k.set_invalid(Invalid::True),
                        _ => {}
                    }
                }
                if a.role == Role::MenuButton {
                    k.set_has_popup(HasPopup::Menu);
                }
                if a.role == Role::ComboBox {
                    k.set_has_popup(HasPopup::Listbox);
                }
                if a.live {
                    k.set_live(Live::Polite);
                }
                if let Some((count, _, _)) = a.rows {
                    k.set_row_count(count);
                }
                if let Some(c) = a.columns {
                    k.set_column_count(c);
                }
                if let Some(r) = a.row_index {
                    k.set_row_index(r - 1);
                    if a.role == Role::ListItem {
                        k.set_position_in_set(r);
                    }
                }
                let acts = [
                    (DO_CLICK, Action::Click),
                    (DO_FOCUS, Action::Focus),
                    (DO_SET, Action::SetValue),
                    (DO_STEP, Action::Increment),
                    (DO_STEP, Action::Decrement),
                    (DO_SHOW, Action::ScrollIntoView),
                    (DO_OPEN, Action::Expand),
                    (DO_OPEN, Action::Collapse),
                ];
                for (bit, act) in acts {
                    if a.actions & bit != 0 {
                        k.add_action(act);
                    }
                }
                let mut children: Vec<NodeId> = a.children.iter().map(|&c| NodeId(tree[c].id)).collect();
                // A text input's text is in lines (text runs), each letter's length in bytes, so
                // the caret and selection can be told by letter.
                if text {
                    let v = a.value.clone().unwrap_or_default();
                    let mut runs = Vec::new();
                    for (i, &(s, e)) in a.lines.iter().enumerate() {
                        let (s, e) = (s.min(v.len()), e.min(v.len()).max(s.min(v.len())));
                        let line = &v[s..e];
                        let mut r = Node::new(accesskit::Role::TextRun);
                        r.set_value(line.to_string());
                        r.set_character_lengths(line.chars().map(|c| c.len_utf8() as u8).collect::<Vec<u8>>());
                        r.set_bounds(Rect::new(x as f64, y as f64, (x + w) as f64, (y + h) as f64));
                        let id = line_id(a.id, i);
                        runs.push((id, s, e, line.chars().count()));
                        children.push(NodeId(id));
                        nodes.push((NodeId(id), r));
                    }
                    // A place in the text, as a line and the letter in it.
                    let at = |p: usize| {
                        let i = runs.iter().rposition(|&(_, s, _, _)| s <= p).unwrap_or(0);
                        let (id, s, e, n) = runs.get(i).copied().unwrap_or((line_id(a.id, 0), 0, 0, 0));
                        let p = p.clamp(s, e);
                        let idx = if p >= e { n } else { v[s..p].chars().count() };
                        TextPosition { node: NodeId(id), character_index: idx }
                    };
                    if let (Some((c, anchor)), false) = (a.caret, runs.is_empty()) {
                        k.set_text_selection(TextSelection { anchor: at(anchor), focus: at(c) });
                    }
                }
                k.set_children(children);
                nodes.push((NodeId(a.id), k));
            }
            let mut info = TreeInfo::new(NodeId(ROOT_ID));
            info.toolkit_name = Some("Lust".into());
            TreeUpdate { nodes, tree: Some(info), tree_id: TreeId::ROOT, focus: NodeId(focus) }
        }

        /// Sends a window's tree to screen readers, when one is listening and it changed: only
        /// the nodes that are new or changed (a node whose pieces came or went changes too), or
        /// the whole tree the first time.
        pub fn send(o: &mut Os, ui: &Ui, scale: f32) {
            if !is_active(o.window.id()) {
                o.sent = None;
                o.kit_sent.clear();
                return;
            }
            let now = ui.access_tree();
            if o.sent.as_ref() == Some(&now) {
                return;
            }
            let full = update(&now.0, now.1, scale);
            let was = &mut o.kit_sent;
            o.access.update_if_active(|| {
                let fresh = was.is_empty();
                let mut changed = Vec::new();
                let mut kept = HashMap::with_capacity(full.nodes.len());
                for (id, node) in full.nodes {
                    if fresh || was.get(&id.0) != Some(&node) {
                        changed.push((id, node.clone()));
                    }
                    kept.insert(id.0, node);
                }
                *was = kept;
                TreeUpdate { nodes: changed, tree: full.tree, tree_id: full.tree_id, focus: full.focus }
            });
            o.sent = Some(now);
        }
    }

    /// The tray icon: the computer's own, through tray-icon, on Windows. Right-clicking it shows
    /// the items as Windows' own menu; a left click runs `click { }`.
    #[cfg(windows)]
    mod tray {
        use super::super::*;
        use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
        use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

        pub enum Hit {
            Click,
            /// The item at this place among the tray's items and dividers.
            Item(usize),
        }

        static HITS: std::sync::Mutex<Vec<Hit>> = std::sync::Mutex::new(Vec::new());
        static PROXY: std::sync::Mutex<Option<winit::event_loop::EventLoopProxy<()>>> = std::sync::Mutex::new(None);

        fn hit(h: Hit) {
            if let Ok(mut q) = HITS.lock() {
                q.push(h);
            }
            if let Some(p) = PROXY.lock().ok().and_then(|p| p.clone()) {
                let _ = p.send_event(());
            }
        }

        /// The tray's clicks wake this event loop. Its handlers can be set once in a program.
        pub fn wake_with(p: winit::event_loop::EventLoopProxy<()>) {
            if let Ok(mut q) = PROXY.lock() {
                *q = Some(p);
            }
            static SET: std::sync::Once = std::sync::Once::new();
            SET.call_once(|| {
                TrayIconEvent::set_event_handler(Some(|e: TrayIconEvent| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                        hit(Hit::Click);
                    }
                }));
                MenuEvent::set_event_handler(Some(|e: MenuEvent| {
                    if let Some(n) = e.id.0.strip_prefix("lust-").and_then(|n| n.parse::<usize>().ok()) {
                        hit(Hit::Item(n));
                    }
                }));
            });
        }

        pub fn take() -> Vec<Hit> {
            HITS.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default()
        }

        /// The tray icon now, and what it was made from, so it's only made again when that changes.
        #[derive(Default)]
        pub struct Tray {
            icon: Option<TrayIcon>,
            made: String,
        }

        /// A Lust icon, white on a blue rounded square, as RGBA: drawn on black and on white, so
        /// how see-through each pixel is can be worked out.
        fn icon_rgba(name: &str) -> (Vec<u8>, u32, u32) {
            const S: usize = 32;
            let draw = |back: u32| {
                let mut c = Canvas::new(S, S, 1.0);
                c.px.iter_mut().for_each(|p| *p = back);
                c.rect(0.0, 0.0, S as f32, S as f32, 7.0, 0x1c7ed6, 1.0);
                paint_icon(&mut c, name, 5.0, 5.0, 22.0, 0xffffff);
                c.px
            };
            let (black, white) = (draw(0x000000), draw(0xffffff));
            let mut out = Vec::with_capacity(S * S * 4);
            for (b, w) in black.iter().zip(&white) {
                let part = |p: u32, s: u32| ((p >> s) & 0xff) as f32;
                let a = (255.0 - (part(*w, 16) - part(*b, 16))).clamp(0.0, 255.0);
                for s in [16, 8, 0] {
                    let v = if a > 0.0 { part(*b, s) * 255.0 / a } else { 0.0 };
                    out.push(v.round().clamp(0.0, 255.0) as u8);
                }
                out.push(a.round() as u8);
            }
            (out, S as u32, S as u32)
        }

        impl Tray {
            pub fn sync(&mut self, root: &Node) {
                let Some(t) = find_tray(root) else {
                    self.icon = None;
                    self.made.clear();
                    return;
                };
                let mut items = Vec::new();
                tray_items(t, &mut items);
                let style = |name: &str| {
                    t.style.iter().find_map(|(k, v)| match (k.as_str(), v) {
                        (k, SV::Text(s)) if k == name => Some(s.clone()),
                        _ => None,
                    })
                };
                let made = format!(
                    "{:?}|{:?}|{:?}|{:?}",
                    style("icon"),
                    style("picture"),
                    tray_tip(t),
                    items.iter().map(|i| (i.kind.clone(), i.text.clone(), item_off(i))).collect::<Vec<_>>()
                );
                if made == self.made && self.icon.is_some() {
                    return;
                }
                self.made = made;
                let menu = Menu::new();
                for (n, i) in items.iter().enumerate() {
                    if i.kind == "divider" {
                        let _ = menu.append(&PredefinedMenuItem::separator());
                    } else {
                        let _ = menu.append(&MenuItem::with_id(format!("lust-{n}"), &i.text, !item_off(i), None));
                    }
                }
                let picture = style("picture").and_then(|p| super::super::picture(&p));
                let icon = match picture {
                    Some(p) => Icon::from_rgba(p.px.iter().flatten().copied().collect(), p.w as u32, p.h as u32),
                    None => {
                        let (rgba, w, h) = icon_rgba(&style("icon").unwrap_or_else(|| "bell".into()));
                        Icon::from_rgba(rgba, w, h)
                    }
                };
                let mut b = TrayIconBuilder::new().with_menu(Box::new(menu)).with_menu_on_left_click(false);
                if let Ok(i) = icon {
                    b = b.with_icon(i);
                }
                let tip = tray_tip(t);
                if !tip.is_empty() {
                    b = b.with_tooltip(tip);
                }
                // The old icon goes first, so there's only ever one.
                self.icon = None;
                self.icon = b.build().ok();
            }
        }
    }

    /// Elsewhere, a tray does nothing: tray-icon's Linux side needs GTK.
    #[cfg(not(windows))]
    mod tray {
        use super::super::*;

        pub enum Hit {
            Click,
            Item(usize),
        }

        pub fn wake_with(_: winit::event_loop::EventLoopProxy<()>) {}

        pub fn take() -> Vec<Hit> {
            vec![]
        }

        #[derive(Default)]
        pub struct Tray;

        impl Tray {
            pub fn sync(&mut self, _: &Node) {}
        }
    }
}

// ----- the computer's own notifications -----

/// On Windows, a Windows notification (a toast in the corner, kept in the list of
/// notifications), without waiting for it. Programs that aren't installed have no name of their
/// own there, so Windows shows it as PowerShell's, as other small programs' notifications do.
#[cfg(windows)]
fn os_notify(title: &str, body: &str) {
    let (title, body) = (title.to_string(), body.to_string());
    let _ = std::thread::Builder::new().name("notify".into()).spawn(move || {
        use tauri_winrt_notification::Toast;
        let _ = Toast::new(Toast::POWERSHELL_APP_ID).title(&title).text1(&body).show();
    });
}

/// On Linux, through the desktop's notification service on D-Bus, without waiting for it.
#[cfg(target_os = "linux")]
fn os_notify(title: &str, body: &str) {
    let (title, body) = (title.to_string(), body.to_string());
    let _ = std::thread::Builder::new().name("notify".into()).spawn(move || {
        let Ok(c) = zbus::blocking::Connection::session() else { return };
        let hints: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
        let _ = c.call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "Notify",
            &("Lust", 0u32, "", title.as_str(), body.as_str(), Vec::<&str>::new(), hints, -1i32),
        );
    });
}

/// Elsewhere (macOS), App:notify does nothing yet.
#[cfg(not(any(windows, target_os = "linux")))]
fn os_notify(_: &str, _: &str) {}

#[cfg(test)]
mod windows_tests {
    use super::*;

    fn window(site: &str, open: bool, title: &str) -> Node {
        let mut w = Node::new("window");
        w.text = site.into();
        w.value = Some(SV::Truth(open));
        w.style = vec![("title".into(), SV::Text(title.into()))];
        w.action = Some(7);
        let mut t = Node::new("text");
        t.text = format!("in {title}");
        w.children.push(t);
        w
    }

    #[test]
    fn windows_are_known_by_where_they_are_written_and_their_turn() {
        let mut root = Node::new("view");
        root.style = vec![("dark".into(), SV::Flag)];
        let mut col = Node::new("column");
        // Two from the same place (a loop), one closed, and one inside another.
        col.children.push(window("5", true, "A"));
        col.children.push(window("5", false, "B"));
        let mut outer = window("9", true, "C");
        outer.children.push(window("12", true, "D"));
        col.children.push(outer);
        root.children.push(col);
        let open: Vec<String> = open_windows(&root).into_iter().map(|(k, _)| k).collect();
        assert_eq!(open, vec!["5#1", "9#1", "12#1"]);
        // A window's own view: its pieces, its settings, dark mode from the view, and its X.
        let d = window_root(root.clone(), "12#1");
        assert_eq!(d.kind, "view");
        assert_eq!(d.children.len(), 1);
        assert_eq!(d.children[0].text, "in D");
        assert!(d.style.iter().any(|(k, _)| k == "dark"));
        assert_eq!(d.mouse, vec![("shut".to_string(), 7)]);
        // One that's gone is an empty view.
        assert!(window_root(root, "5#9").children.is_empty());
    }

    #[test]
    fn a_window_with_dark_of_its_own_keeps_it() {
        let mut root = Node::new("view");
        root.style = vec![("dark".into(), SV::Flag)];
        let mut w = window("3", true, "Light");
        w.style.push(("dark".into(), SV::Truth(false)));
        root.children.push(w);
        let r = window_root(root, "3#1");
        let darks: Vec<&SV> = r.style.iter().filter(|(k, _)| k == "dark").map(|(_, v)| v).collect();
        assert!(matches!(darks.as_slice(), [SV::Truth(false)]));
        assert!(!settings(&r.style).dark);
    }

    #[test]
    fn toasts_fade_in_and_out() {
        let t = Toast { text: "Saved".into(), style: vec![], start: 10.0, secs: 4.0 };
        assert_eq!(t.solid(10.0, true), 0.0);
        assert_eq!(t.solid(10.0, false), 1.0);
        assert_eq!(t.solid(12.0, true), 1.0);
        assert!((t.solid(13.85, true) - 0.5).abs() < 1e-3);
        assert_eq!(t.solid(14.0, true), 0.0);
    }

    #[test]
    fn the_tray_counts_its_items_and_skips_disabled_ones() {
        let mut tray = Node::new("tray");
        tray.style = vec![("tip".into(), SV::Text("Notes".into()))];
        let mut a = Node::new("item");
        a.text = "Show".into();
        let mut b = Node::new("item");
        b.text = "Count".into();
        b.style = vec![("disabled".into(), SV::Flag)];
        let mut c = Node::new("item");
        c.text = "Later".into();
        c.style = vec![("disabled".into(), SV::Truth(false))];
        // Items from a question or loop sit in a column of their own.
        let mut group = Node::new("column");
        group.children.push(c);
        tray.children = vec![a, Node::new("divider"), b, group];
        let mut root = Node::new("view");
        root.children.push(tray);
        let t = find_tray(&root).expect("a tray");
        let mut items = Vec::new();
        tray_items(t, &mut items);
        let names: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(names, vec!["Show", "", "Count", "Later"]);
        assert_eq!(items.iter().map(|i| item_off(i)).collect::<Vec<_>>(), vec![false, false, true, false]);
        assert_eq!(tray_tip(t), "Notes");
    }

    #[test]
    fn settings_read_full_screen_hiding_and_escape() {
        let s = settings(&[("fullscreen".into(), SV::Flag), ("visible".into(), SV::Truth(false)), ("escape".into(), SV::Flag)]);
        assert!(s.fullscreen && !s.visible && s.escape);
        let s = settings(&[("fullscreen".into(), SV::Truth(false))]);
        assert!(!s.fullscreen && s.visible && !s.escape);
    }
}
