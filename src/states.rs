//! The states feed: while Ledger watches a program's states, `lust run` writes each thing with a
//! state of its own, and every change to it, to a file Ledger names in `LUST_STATES`.
//!
//! The file is apart from the program's own output, so nothing a program prints can be taken for
//! it, and a file works the same on Windows, macOS and Linux: no ports, no handles passed down.
//! Each line is one JSON object (see LUST.md, section 21, Watching states from Ledger):
//!
//! ```text
//! {"ev":"start","v":1,"lust":"2.0.0-alpha.101"}
//! {"ev":"made","ms":3,"id":1,"class":"Hero","name":"scout","opt":"Calm"}
//! {"ev":"set","ms":1240,"id":1,"from":"Calm","opt":"Battle.Melee"}
//! {"ev":"end","ms":2210,"id":1,"how":"state"}
//! {"ev":"skipped","ms":2250,"n":4180}
//! {"ev":"stop","ms":3000}
//! ```
//!
//! Lines are gathered and written every 50 ms by a thread of their own, so the program never waits
//! for the disk. A program that makes things or changes states faster than anyone can read (more
//! than 200 lines in 50 ms) has the rest gathered up: a `skipped` line says how many changes
//! weren't listed one by one, then each thing made since is written once, in the option it's in
//! by then, and each thing that changed gets one `set` line, from where it was to where it ended
//! up. A thing made and ended within those 50 ms is only counted.

use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

/// The feed's format. Ledger reads lines of versions it knows.
pub const VERSION: u32 = 1;
/// How often lines are written.
const TICK: Duration = Duration::from_millis(50);
/// Changes written one by one in each tick; the rest are gathered up.
const PER_TICK: u32 = 200;

pub struct Feed {
    shared: Arc<Shared>,
    writer: Mutex<Option<std::thread::JoinHandle<()>>>,
}

struct Shared {
    inner: Mutex<Inner>,
    /// Things Lua let go of, by id. Kept apart from `inner`: they arrive while Lua tidies up, which
    /// can happen in the middle of any of the feed's calls.
    gone: Mutex<Vec<u64>>,
    stop: AtomicBool,
}

struct Inner {
    out: Box<dyn Write + Send>,
    start: Instant,
    next: u64,
    /// Each thing still going: its option now.
    live: HashMap<u64, String>,
    /// Lines not written yet.
    pending: String,
    /// Changes written one by one this tick.
    this_tick: u32,
    /// Changes gathered up this tick: where each thing was, and where it is now.
    gathered: BTreeMap<u64, (String, String)>,
    /// Things made this tick past the limit, not written yet: a thing that ends before the tick
    /// does is never written at all, only counted.
    held: BTreeMap<u64, Held>,
    skipped: u64,
    closed: bool,
}

struct Held {
    class: String,
    name: Option<String>,
    opt: String,
    ms: u128,
}

fn made_line(ms: u128, id: u64, class: &str, name: Option<&str>, opt: &str) -> String {
    let name = name.map(|n| format!(",\"name\":{}", q(n))).unwrap_or_default();
    format!(
        "{{\"ev\":\"made\",\"ms\":{ms},\"id\":{id},\"class\":{}{name},\"opt\":{}}}\n",
        q(class),
        q(opt)
    )
}

/// Handed to Lua with each thing: Lua drops it when it lets the thing go.
pub struct Token {
    id: u64,
    shared: Weak<Shared>,
}

impl Drop for Token {
    fn drop(&mut self) {
        if let Some(s) = self.shared.upgrade() {
            s.gone.lock().unwrap_or_else(|e| e.into_inner()).push(self.id);
        }
    }
}

#[cfg(feature = "cli")]
impl mlua::UserData for Token {}

/// Text as JSON writes it, quotes and all.
fn q(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl Inner {
    fn ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    fn end(&mut self, id: u64, how: &str) {
        if self.live.remove(&id).is_none() {
            return;
        }
        if self.held.remove(&id).is_some() {
            self.skipped += 1;
            return;
        }
        self.gathered.remove(&id);
        let ms = self.ms();
        self.pending
            .push_str(&format!("{{\"ev\":\"end\",\"ms\":{ms},\"id\":{id},\"how\":{}}}\n", q(how)));
    }

    /// Writes what's waiting: things Lua let go of, then the changes gathered up.
    fn flush(&mut self, gone: Vec<u64>) {
        for id in gone {
            self.end(id, "gone");
        }
        let ms = self.ms();
        if self.skipped > 0 {
            self.pending
                .push_str(&format!("{{\"ev\":\"skipped\",\"ms\":{ms},\"n\":{}}}\n", self.skipped));
            self.skipped = 0;
        }
        for (id, h) in std::mem::take(&mut self.held) {
            self.pending.push_str(&made_line(h.ms, id, &h.class, h.name.as_deref(), &h.opt));
        }
        for (id, (from, opt)) in std::mem::take(&mut self.gathered) {
            if from != opt {
                self.pending.push_str(&format!(
                    "{{\"ev\":\"set\",\"ms\":{ms},\"id\":{id},\"from\":{},\"opt\":{}}}\n",
                    q(&from),
                    q(&opt)
                ));
            }
        }
        self.this_tick = 0;
        if !self.pending.is_empty() {
            // A feed nobody can write to any more just stops; the program goes on.
            if self.out.write_all(self.pending.as_bytes()).and_then(|_| self.out.flush()).is_err() {
                self.closed = true;
            }
            self.pending.clear();
        }
    }
}

impl Feed {
    /// Starts a feed into a new file at `path` (any file there is replaced).
    pub fn open(path: &Path) -> std::io::Result<Feed> {
        let file = std::fs::File::create(path)?;
        Ok(Feed::to(Box::new(file)))
    }

    /// Starts a feed into `out`.
    pub fn to(mut out: Box<dyn Write + Send>) -> Feed {
        let hello = format!("{{\"ev\":\"start\",\"v\":{VERSION},\"lust\":{}}}\n", q(env!("CARGO_PKG_VERSION")));
        let closed = out.write_all(hello.as_bytes()).and_then(|_| out.flush()).is_err();
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner {
                out,
                start: Instant::now(),
                next: 0,
                live: HashMap::new(),
                pending: String::new(),
                this_tick: 0,
                gathered: BTreeMap::new(),
                held: BTreeMap::new(),
                skipped: 0,
                closed,
            }),
            gone: Mutex::new(Vec::new()),
            stop: AtomicBool::new(false),
        });
        let s = shared.clone();
        let writer = std::thread::spawn(move || {
            while !s.stop.load(Ordering::Relaxed) {
                // finish wakes it early.
                std::thread::park_timeout(TICK);
                s.flush();
            }
        });
        Feed {
            shared,
            writer: Mutex::new(Some(writer)),
        }
    }

    /// A thing with a state of its own was made, in option `opt`. Gives back its token.
    pub fn made(&self, class: &str, name: Option<&str>, opt: &str) -> Token {
        let mut g = self.shared.lock();
        g.next += 1;
        let id = g.next;
        if !g.closed {
            let ms = g.ms();
            g.live.insert(id, opt.to_string());
            if g.this_tick >= PER_TICK {
                let held = Held {
                    class: class.to_string(),
                    name: name.map(String::from),
                    opt: opt.to_string(),
                    ms,
                };
                g.held.insert(id, held);
                // Counted as skipped only if it ends this tick; otherwise its made line is written.
            } else {
                g.this_tick += 1;
                let line = made_line(ms, id, class, name, opt);
                g.pending.push_str(&line);
            }
        }
        Token {
            id,
            shared: Arc::downgrade(&self.shared),
        }
    }

    /// The thing `token` stands for is in option `opt` now.
    pub fn set(&self, token: &Token, opt: &str) {
        let mut g = self.shared.lock();
        let id = token.id;
        let Some(from) = g.live.get(&id).cloned() else { return };
        if from == opt {
            return;
        }
        g.live.insert(id, opt.to_string());
        if let Some(h) = g.held.get_mut(&id) {
            h.opt = opt.to_string();
            g.skipped += 1;
            return;
        }
        if g.this_tick >= PER_TICK {
            g.skipped += 1;
            g.gathered.entry(id).or_insert_with(|| (from, String::new())).1 = opt.to_string();
            return;
        }
        g.this_tick += 1;
        let ms = g.ms();
        let line = format!(
            "{{\"ev\":\"set\",\"ms\":{ms},\"id\":{id},\"from\":{},\"opt\":{}}}\n",
            q(&from),
            q(opt)
        );
        g.pending.push_str(&line);
    }

    /// The thing `token` stands for ended with the option of its owner's it was made in.
    pub fn ended(&self, token: &Token) {
        self.shared.lock().end(token.id, "state");
    }

    /// The program is over: everything waiting is written, then a stop line, and nothing after
    /// it. So the things still going when it ended stay listed, as they were.
    pub fn finish(&self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(w) = self.writer.lock().unwrap_or_else(|e| e.into_inner()).take() {
            w.thread().unpark();
            let _ = w.join();
        }
        self.shared.flush();
        let mut g = self.shared.lock();
        let ms = g.ms();
        g.pending.push_str(&format!("{{\"ev\":\"stop\",\"ms\":{ms}}}\n"));
        g.flush(Vec::new());
        g.closed = true;
    }
}

impl Shared {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn flush(&self) {
        let gone = std::mem::take(&mut *self.gone.lock().unwrap_or_else(|e| e.into_inner()));
        let mut g = self.lock();
        if !g.closed {
            g.flush(gone);
        }
    }
}

/// Gives the runtime its `LUST_HOST.states`: made, set and ended (runtime.lua, the states feed).
/// `names` is each Lua line that makes a thing straight into a variable or field, and that name
/// (lua.rs, Output::names); `made` looks up the line that called `new`.
#[cfg(feature = "cli")]
pub fn install(lua: &mlua::Lua, feed: &Arc<Feed>, names: &[(u32, String)]) -> mlua::Result<()> {
    let names: HashMap<u32, String> = names.iter().cloned().collect();
    let t = lua.create_table()?;
    let f = feed.clone();
    t.set(
        "made",
        lua.create_function(move |lua, (class, opt): (String, String)| {
            // 0 is this function, 1 the runtime's new, 2 the line that called it.
            let line = lua.inspect_stack(2).map(|d| d.curr_line()).unwrap_or(-1);
            let name = u32::try_from(line).ok().and_then(|l| names.get(&l));
            let token = f.made(&class, name.map(|s| s.as_str()), &opt);
            lua.create_userdata(token)
        })?,
    )?;
    let f = feed.clone();
    t.set(
        "set",
        lua.create_function(move |_, (token, opt): (mlua::AnyUserData, String)| {
            if let Ok(t) = token.borrow::<Token>() {
                f.set(&t, &opt);
            }
            Ok(())
        })?,
    )?;
    let f = feed.clone();
    t.set(
        "ended",
        lua.create_function(move |_, token: mlua::AnyUserData| {
            if let Ok(t) = token.borrow::<Token>() {
                f.ended(&t);
            }
            Ok(())
        })?,
    )?;
    let host: mlua::Table = lua.globals().get("LUST_HOST")?;
    host.set("states", t)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lines written so far, shared with the test.
    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl Sink {
        fn lines(&self) -> Vec<String> {
            String::from_utf8(self.0.lock().unwrap().clone())
                .unwrap()
                .lines()
                .map(|l| l.to_string())
                .collect()
        }
    }

    /// Takes the times out, which differ from run to run.
    fn untimed(l: &str) -> String {
        match l.find(",\"ms\":") {
            Some(i) => {
                let rest = &l[i + 6..];
                let j = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
                format!("{}{}", &l[..i], &rest[j..])
            }
            None => l.to_string(),
        }
    }

    #[test]
    fn writes_made_set_end_and_stop() {
        let sink = Sink::default();
        let feed = Feed::to(Box::new(sink.clone()));
        let hero = feed.made("Hero", Some("scout"), "Calm");
        let door = feed.made("Door", None, "Locked");
        feed.set(&hero, "Battle.Melee");
        feed.set(&hero, "Battle.Melee"); // staying where it is changes nothing
        feed.ended(&door);
        feed.ended(&door); // only once
        feed.finish();
        let lines: Vec<String> = sink.lines().iter().map(|l| untimed(l)).collect();
        assert_eq!(
            lines,
            [
                format!("{{\"ev\":\"start\",\"v\":1,\"lust\":\"{}\"}}", env!("CARGO_PKG_VERSION")),
                r#"{"ev":"made","id":1,"class":"Hero","name":"scout","opt":"Calm"}"#.into(),
                r#"{"ev":"made","id":2,"class":"Door","opt":"Locked"}"#.into(),
                r#"{"ev":"set","id":1,"from":"Calm","opt":"Battle.Melee"}"#.into(),
                r#"{"ev":"end","id":2,"how":"state"}"#.into(),
                r#"{"ev":"stop"}"#.into(),
            ]
        );
    }

    #[test]
    fn a_thing_let_go_ends_and_changes_after_that_are_ignored() {
        let sink = Sink::default();
        let feed = Feed::to(Box::new(sink.clone()));
        let a = feed.made("Hero", None, "Calm");
        let id = a.id;
        drop(a);
        // A token for the same id, as if Lua still held a copy: its changes no longer count.
        let again = Token {
            id,
            shared: Arc::downgrade(&feed.shared),
        };
        feed.shared.flush();
        feed.set(&again, "Battle");
        std::mem::forget(again);
        feed.finish();
        let lines: Vec<String> = sink.lines().iter().map(|l| untimed(l)).collect();
        assert_eq!(lines[2], r#"{"ev":"end","id":1,"how":"gone"}"#);
        assert_eq!(lines.len(), 4, "{lines:?}");
    }

    #[test]
    fn a_flood_of_changes_is_gathered_up() {
        let sink = Sink::default();
        let feed = Feed::to(Box::new(sink.clone()));
        let a = feed.made("Light", Some("lamp"), "Off");
        let b = feed.made("Light", Some("torch"), "Off");
        // Far more than one tick takes one by one, all at once.
        for i in 0..10_000 {
            feed.set(&a, if i % 2 == 0 { "On" } else { "Off" });
        }
        feed.set(&b, "On");
        feed.finish();
        let lines = sink.lines();
        let sets = lines.iter().filter(|l| l.contains("\"ev\":\"set\"")).count();
        assert!(sets <= PER_TICK as usize * 3 + 2, "{sets} set lines");
        // The rest were counted on skipped lines.
        let n: u64 = lines
            .iter()
            .filter(|l| l.contains("skipped"))
            .map(|l| l.rsplit("\"n\":").next().unwrap().trim_end_matches('}').parse::<u64>().unwrap())
            .sum();
        assert!(n >= 10_001 - sets as u64, "{n} skipped, {sets} written");
        // The last word on each thing is where it ended up.
        let last_a = lines.iter().rev().find(|l| l.contains("\"id\":1,")).unwrap();
        assert!(last_a.ends_with(r#""opt":"Off"}"#), "{last_a}");
        let last_b = lines.iter().rev().find(|l| l.contains("\"id\":2,")).unwrap();
        assert!(last_b.ends_with(r#""opt":"On"}"#), "{last_b}");
    }

    #[test]
    fn a_flood_of_short_lived_things_is_only_counted() {
        let sink = Sink::default();
        let feed = Feed::to(Box::new(sink.clone()));
        for _ in 0..5_000 {
            let t = feed.made("Spark", Some("spark"), "Lit");
            feed.ended(&t);
        }
        let kept = feed.made("Spark", Some("last"), "Lit");
        feed.set(&kept, "Out");
        feed.finish();
        let lines = sink.lines();
        let made = lines.iter().filter(|l| l.contains("\"ev\":\"made\"")).count();
        let ends = lines.iter().filter(|l| l.contains("\"ev\":\"end\"")).count();
        assert!(made <= PER_TICK as usize * 3 + 1, "{made} made lines");
        assert_eq!(made, ends + 1, "every thing written is ended, but the last");
        let n: u64 = lines
            .iter()
            .filter(|l| l.contains("skipped"))
            .map(|l| l.rsplit("\"n\":").next().unwrap().trim_end_matches('}').parse::<u64>().unwrap())
            .sum();
        assert!(n >= 5_000 - ends as u64, "{n} counted, {ends} written");
        // The one still going is written, in the option it ended up in.
        let last = lines
            .iter()
            .filter(|l| l.contains("\"name\":\"last\"") || l.contains("\"id\":5001,"))
            .last()
            .unwrap();
        assert!(last.ends_with(r#""opt":"Out"}"#), "{last}");
    }

    #[test]
    fn text_is_escaped_as_json() {
        assert_eq!(q("plain"), "\"plain\"");
        assert_eq!(q("a\"b\\c\nd\u{1}"), "\"a\\\"b\\\\c\\nd\\u0001\"");
    }

    /// Every line the feed writes is one JSON object, whatever names and options it's given.
    #[test]
    fn any_text_makes_lines_of_json() {
        let sink = Sink::default();
        let feed = Feed::to(Box::new(sink.clone()));
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut text = || {
            let mut s = String::new();
            for _ in 0..(seed % 12) {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                s.push(char::from_u32((seed % 0x3000) as u32).unwrap_or('?'));
            }
            s
        };
        let mut tokens = Vec::new();
        for _ in 0..300 {
            let (c, n, o) = (text(), text(), text());
            tokens.push(feed.made(&c, Some(&n), &o));
            let o2 = text();
            feed.set(tokens.last().unwrap(), &o2);
        }
        feed.finish();
        for l in sink.lines() {
            assert!(l.starts_with('{') && l.ends_with('}'), "{l}");
            assert!(!l.chars().any(|c| (c as u32) < 0x20), "{l:?}");
            // Quotes inside text are always escaped, so they pair up.
            let mut quotes = 0;
            let mut esc = false;
            for c in l.chars() {
                match (esc, c) {
                    (true, _) => esc = false,
                    (false, '\\') => esc = true,
                    (false, '"') => quotes += 1,
                    _ => {}
                }
            }
            assert_eq!(quotes % 2, 0, "{l}");
        }
    }
}
