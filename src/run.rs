//! Runs the Lua a program turns into, with Lua 5.1 built into lust itself, and
//! turns Lua's errors back into Lust lines and plain words.

use std::process::ExitCode;

use std::cell::RefCell;
use std::rc::Rc;

use mlua::Lua;

use crate::lua::Output;

/// What Lua can't do on its own, from lust: folders, and a precise clock.
fn host(lua: &Lua) -> mlua::Result<()> {
    use crate::native_rt as rt;
    let h = lua.create_table()?;
    let started = std::time::Instant::now();
    h.set("exists", lua.create_function(|_, p: String| Ok(std::path::Path::new(&p).exists()))?)?;
    // A failure comes back as its message and its Problem option.
    h.set(
        "remove",
        lua.create_function(|_, p: String| Ok(rt::fs_remove(&p).err().map_or((None, None), |(k, m)| (Some(m), Some(k)))))?,
    )?;
    h.set(
        "list",
        lua.create_function(|_, p: String| {
            Ok(match rt::fs_list(&p) {
                Ok(names) => (Some(names), None, None),
                Err((k, m)) => (None, Some(m), Some(k)),
            })
        })?,
    )?;
    h.set(
        "mkdir",
        lua.create_function(|_, p: String| Ok(rt::fs_mkdir(&p).err().map_or((None, None), |(k, m)| (Some(m), Some(k)))))?,
    )?;
    // Slow built-ins: the work goes on a thread of its own, and runtime.lua's run looks for its
    // answer by number (native_rt's slow built-ins, the same code as built programs use).
    let slows: Rc<RefCell<(u64, std::collections::HashMap<u64, rt::Slow>)>> = Rc::default();
    let keep = |slows: &Rc<RefCell<(u64, std::collections::HashMap<u64, rt::Slow>)>>, s: rt::Slow| {
        let mut x = slows.borrow_mut();
        x.0 += 1;
        let id = x.0;
        x.1.insert(id, s);
        id
    };
    type Got = (Option<String>, Option<String>, Option<&'static str>);
    fn got(r: rt::SlowResult) -> Got {
        match r {
            Ok(t) => (Some(t), None, None),
            Err((k, m)) => (None, Some(m), Some(k)),
        }
    }
    let x = slows.clone();
    h.set(
        "http_start",
        lua.create_function(move |_, (url, body): (String, Option<String>)| {
            Ok(keep(&x, rt::Slow::start(move || crate::native_net::fetch(&url, body.as_deref()))))
        })?,
    )?;
    let x = slows.clone();
    h.set(
        "choose_open_start",
        lua.create_function(move |_, (title, types): (String, Vec<String>)| {
            Ok(keep(&x, rt::slow_box(move || crate::native_dlg::choose_open(title, types))))
        })?,
    )?;
    let x = slows.clone();
    h.set(
        "choose_save_start",
        lua.create_function(move |_, (title, name): (String, String)| {
            Ok(keep(&x, rt::slow_box(move || crate::native_dlg::choose_save(title, name))))
        })?,
    )?;
    let x = slows.clone();
    h.set(
        "choose_folder_start",
        lua.create_function(move |_, title: String| Ok(keep(&x, rt::slow_box(move || crate::native_dlg::choose_folder(title)))))?,
    )?;
    // Whether the answer's come, and if so the text, or why there's none and its Problem.
    let x = slows.clone();
    h.set(
        "slow_look",
        lua.create_function(move |_, id: u64| {
            let r = x.borrow().1.get(&id).and_then(|s| s.look());
            Ok(match r {
                Some(r) => {
                    x.borrow_mut().1.remove(&id);
                    let (t, m, k) = got(r);
                    (true, t, m, k)
                }
                None => (false, None, None, None),
            })
        })?,
    )?;
    // Waits for the answer, holding up everything: where a run can't pause.
    let x = slows;
    h.set(
        "slow_wait",
        lua.create_function(move |_, id: u64| {
            let s = x.borrow_mut().1.remove(&id);
            Ok(got(s.map_or(Err(("Other", "the work was already done.".to_string())), |s| s.wait())))
        })?,
    )?;
    h.set(
        "choose_open",
        lua.create_function(|_, (title, types): (String, Vec<String>)| Ok(crate::native_dlg::choose_open(title, types)))?,
    )?;
    h.set(
        "choose_save",
        lua.create_function(|_, (title, name): (String, String)| Ok(crate::native_dlg::choose_save(title, name)))?,
    )?;
    h.set(
        "choose_folder",
        lua.create_function(|_, title: String| Ok(crate::native_dlg::choose_folder(title)))?,
    )?;
    h.set("now", lua.create_function(|_, ()| Ok(rt::time_now()))?)?;
    // Random rolls the same way as in built programs (the checks for lo > hi happen in Lua).
    h.set("random", lua.create_function(|_, (lo, hi): (i64, i64)| Ok(rt::random(lo, hi)))?)?;
    h.set("frandom", lua.create_function(|_, (lo, hi): (f64, f64)| Ok(rt::frandom(lo, hi)))?)?;
    h.set("seed", lua.create_function(|_, s: f64| Ok(rt::random_seed(s as i64)))?)?;
    h.set(
        "ui_open",
        lua.create_function(
            |_,
             (settings, view, tweens, settle, spawn, runs): (
                mlua::Table,
                mlua::Function,
                Option<mlua::Function>,
                Option<mlua::Function>,
                Option<mlua::Function>,
                Option<mlua::Function>,
            )| {
                let s = ui_styles(&settings)?;
                // After each callback, the events it sent are delivered (the runtime's L.settle).
                if let Some(f) = settle {
                    crate::native_ui::on_settle(Rc::new(move || f.call::<()>(()).map_err(|e| raw_error(&e))));
                }
                // Each callback is a run of its own (the runtime's L.spawn), and the window's clock
                // moves the runs that wait (L.runs_view).
                SPAWN.with(|s| *s.borrow_mut() = spawn);
                if let Some(f) = runs {
                    crate::native_ui::on_runs(Rc::new(move |dt: f64, wake: bool| {
                        f.call::<Option<f64>>((dt, wake)).map_err(|e| raw_error(&e))
                    }));
                }
                let mut build = || -> Result<(crate::native_ui::Node, Vec<crate::native_ui::Action>), String> {
                    let t: mlua::Table = view.call(()).map_err(|e| raw_error(&e))?;
                    let mut actions = Vec::new();
                    let n = ui_node(&t, &mut actions).map_err(|e| raw_error(&e))?;
                    Ok((n, actions))
                };
                // The window's clock moves the program's tweens (the runtime's L.tween_step).
                let step = Box::new(move |dt: f64| -> Result<bool, String> {
                    match &tweens {
                        Some(f) => f.call::<bool>(dt).map_err(|e| raw_error(&e)),
                        None => Ok(false),
                    }
                });
                Ok(crate::native_ui::show(&s, &mut build, step).err())
            },
        )?,
    )?;
    // Toasts, App:quit() and App:notify(…): the same code as built programs use.
    h.set(
        "ui_toast",
        lua.create_function(|_, (text, style): (String, mlua::Table)| {
            crate::native_ui::toast(text, ui_styles(&style)?);
            Ok(())
        })?,
    )?;
    h.set("app_quit", lua.create_function(|_, ()| Ok(crate::native_ui::quit()))?)?;
    h.set(
        "app_notify",
        lua.create_function(|_, (title, body): (String, String)| Ok(crate::native_ui::notify(title, body)))?,
    )?;
    // Tables and lists: which rows to make, and the order a sort puts them in.
    {
        use crate::native_ui as ui;
        h.set(
            "ui_list_begin",
            lua.create_function(|_, (site, n, rowh): (f64, f64, f64)| Ok(ui::list_begin(site as usize, n.max(0.0) as usize, rowh)))?,
        )?;
        h.set("ui_list_sort", lua.create_function(|_, id: usize| Ok(ui::list_sort(id)))?)?;
        h.set("ui_list_range", lua.create_function(|_, id: usize| Ok(ui::list_range(id)))?)?;
        h.set("ui_list_hint", lua.create_function(|_, id: usize| Ok(ui::list_hint(id)))?)?;
        h.set(
            "ui_list_order",
            lua.create_function(|lua, (id, keys, falling): (usize, mlua::Table, bool)| {
                let mut list = Vec::with_capacity(keys.raw_len());
                for k in keys.sequence_values::<mlua::Value>() {
                    list.push(match k? {
                        mlua::Value::String(s) => ui::SortKey::Text(s.to_str()?.to_string()),
                        mlua::Value::Integer(i) => ui::SortKey::Num(i as f64),
                        mlua::Value::Number(x) => ui::SortKey::Num(x),
                        _ => ui::SortKey::Num(0.0),
                    });
                }
                // Lua counts from 1.
                lua.create_sequence_from(ui::list_order(id, list, falling).into_iter().map(|i| i + 1))
            })?,
        )?;
    }
    h.set(
        "http",
        lua.create_function(|_, (url, body): (String, Option<String>)| {
            Ok(match crate::native_net::fetch(&url, body.as_deref()) {
                Ok(t) => (Some(t), None, None),
                Err((k, m)) => (None, Some(m), Some(k)),
            })
        })?,
    )?;
    {
        use crate::native_text as p;
        fn both<T>(r: Result<T, String>) -> (Option<T>, Option<String>) {
            match r {
                Ok(v) => (Some(v), None),
                Err(e) => (None, Some(e)),
            }
        }
        h.set(
            "pat_matches",
            lua.create_function(|_, (s, pt): (String, String)| Ok(both(p::try_matches(&s, &pt))))?,
        )?;
        h.set(
            "pat_find_all",
            lua.create_function(|_, (s, pt): (String, String)| Ok(both(p::try_find_all(&s, &pt))))?,
        )?;
        h.set(
            "pat_capture",
            lua.create_function(|_, (s, pt): (String, String)| Ok(both(p::try_capture(&s, &pt))))?,
        )?;
        h.set(
            "pat_replace_all",
            lua.create_function(|_, (s, pt, w): (String, String, String)| Ok(both(p::try_replace_all(&s, &pt, &w))))?,
        )?;
    }
    h.set(
        "serve",
        lua.create_function(|_, (port, answer): (i64, mlua::Function)| {
            let r = crate::native_web::serve_with(port, &mut |m, p, b| match answer.call::<(u16, String)>((m, p, b)) {
                Ok(x) => x,
                Err(e) => (500, format!("{{\"error\":{:?}}}", e.to_string())),
            });
            Ok(r.err())
        })?,
    )?;
    // Databases: opened ones are kept here, and Lua holds their number.
    let dbs: Rc<RefCell<Vec<crate::native_db::Db>>> = Rc::new(RefCell::new(Vec::new()));
    let d = dbs.clone();
    h.set(
        "db_open",
        lua.create_function(move |_, (path, tables): (String, mlua::Table)| {
            let mut list: Vec<(String, Vec<(String, String)>)> = Vec::new();
            for t in tables.sequence_values::<mlua::Table>() {
                let t = t?;
                let name: String = t.get(1)?;
                let cols: mlua::Table = t.get(2)?;
                let mut cs = Vec::new();
                for c in cols.sequence_values::<mlua::Table>() {
                    let c = c?;
                    cs.push((c.get::<String>(1)?, c.get::<String>(2)?));
                }
                list.push((name, cs));
            }
            let owned: Vec<(&str, Vec<(&str, &str)>)> = list
                .iter()
                .map(|(n, cs)| (n.as_str(), cs.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect()))
                .collect();
            let refs: Vec<(&str, &[(&str, &str)])> = owned.iter().map(|(n, cs)| (*n, cs.as_slice())).collect();
            match crate::native_db::open_db(&path, &refs) {
                Ok(db) => {
                    d.borrow_mut().push(db);
                    Ok((Some(d.borrow().len()), None))
                }
                Err(e) => Ok((None, Some(e))),
            }
        })?,
    )?;
    fn params(t: mlua::Table) -> mlua::Result<Vec<crate::native_db::SqlVal>> {
        use crate::native_db::SqlVal;
        let mut out = Vec::new();
        for v in t.sequence_values::<mlua::Value>() {
            out.push(match v? {
                mlua::Value::Number(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => SqlVal::Int(n as i64),
                mlua::Value::Number(n) => SqlVal::Real(n),
                mlua::Value::Integer(i) => SqlVal::Int(i),
                mlua::Value::String(s) => SqlVal::Text(s.to_str()?.to_string()),
                mlua::Value::Boolean(b) => SqlVal::Int(b as i64),
                _ => SqlVal::Null,
            });
        }
        Ok(out)
    }
    let d = dbs.clone();
    h.set(
        "db_exec",
        lua.create_function(move |_, (id, sql, ps): (usize, String, mlua::Table)| {
            let dbs = d.borrow();
            match crate::native_db::exec(&dbs[id - 1], &sql, &params(ps)?) {
                Ok(n) => Ok((Some(n), None)),
                Err(e) => Ok((None, Some(e))),
            }
        })?,
    )?;
    let d = dbs.clone();
    h.set(
        "db_rows",
        lua.create_function(move |lua, (id, sql, ps): (usize, String, mlua::Table)| {
            use crate::native_db::SqlVal;
            let dbs = d.borrow();
            match crate::native_db::rows(&dbs[id - 1], &sql, &params(ps)?) {
                Ok(rows) => {
                    let out = lua.create_table()?;
                    for (i, r) in rows.into_iter().enumerate() {
                        let row = lua.create_table()?;
                        for (j, v) in r.into_iter().enumerate() {
                            match v {
                                SqlVal::Int(x) => row.set(j + 1, x as f64)?,
                                SqlVal::Real(x) => row.set(j + 1, x)?,
                                SqlVal::Text(s) => row.set(j + 1, s)?,
                                SqlVal::Null => {}
                            }
                        }
                        out.set(i + 1, row)?;
                    }
                    Ok((Some(out), None))
                }
                Err(e) => Ok((None, Some(e))),
            }
        })?,
    )?;
    h.set("clock", lua.create_function(move |_, ()| Ok(started.elapsed().as_secs_f64()))?)?;
    // Waits take real time, but not in lust test, where the clock is Lust's own.
    let quick = std::env::var_os("LUST_VIEW_TEST").is_some();
    h.set(
        "sleep",
        lua.create_function(move |_, secs: f64| {
            if !quick && secs > 0.0 {
                std::thread::sleep(std::time::Duration::from_secs_f64(secs.min(24.0 * 24.0 * 3600.0)));
            }
            Ok(())
        })?,
    )?;
    lua.globals().set("LUST_HOST", h)
}

thread_local! {
    /// The runtime's L.spawn: runs a window's callback as a run of its own.
    static SPAWN: RefCell<Option<mlua::Function>> = const { RefCell::new(None) };
}

/// Runs a window's callback, given these values, as a run of its own.
fn act(f: &mlua::Function, args: impl mlua::IntoLuaMulti) -> mlua::Result<()> {
    match SPAWN.with(|s| s.borrow().clone()) {
        Some(spawn) => spawn.call::<()>((f.clone(), args)),
        None => f.call::<()>(args),
    }
}

/// A Lua error's own message, like "lust:12: something went wrong", without Lua's extras.
fn raw_error(e: &mlua::Error) -> String {
    match e {
        mlua::Error::RuntimeError(s) => s.lines().next().unwrap_or("").to_string(),
        mlua::Error::CallbackError { cause, .. } => raw_error(cause),
        other => other.to_string().lines().next().unwrap_or("").to_string(),
    }
}

/// A style list from Lua: { {"gap", 12}, {"center", true}, {"hover", { … }} }.
fn ui_styles(t: &mlua::Table) -> mlua::Result<Vec<(String, crate::native_ui::SV)>> {
    use crate::native_ui::SV;
    let mut out = Vec::new();
    for pair in t.sequence_values::<mlua::Table>() {
        let pair = pair?;
        let name: String = pair.get(1)?;
        let v = match pair.get::<mlua::Value>(2)? {
            // swatches [red5, blue5]: a colour array, each colour's four parts.
            mlua::Value::Table(list) if name == "swatches" => {
                let mut all = Vec::new();
                for c in list.sequence_values::<mlua::Table>() {
                    all.push(lua_colour(&c?)?);
                }
                SV::Colours(all)
            }
            mlua::Value::Number(n) => SV::Num(n),
            mlua::Value::Integer(i) => SV::Num(i as f64),
            mlua::Value::String(s) => SV::Text(s.to_str()?.to_string()),
            mlua::Value::Table(n) => SV::Nested(ui_styles(&n)?),
            // A style that's on only when something is true, and that's false now.
            mlua::Value::Boolean(false) => SV::Truth(false),
            _ => SV::Flag,
        };
        out.push((name, v));
    }
    Ok(out)
}

/// A colour from Lua, { r, g, b, a }: each part is already an f32, so it's kept exactly.
fn lua_colour(c: &mlua::Table) -> mlua::Result<[f32; 4]> {
    Ok([
        c.get::<f64>("r")? as f32,
        c.get::<f64>("g")? as f32,
        c.get::<f64>("b")? as f32,
        c.get::<f64>("a")? as f32,
    ])
}

/// A view's tree of tables, turned into nodes. Buttons' functions become actions.
fn ui_node(t: &mlua::Table, actions: &mut Vec<crate::native_ui::Action>) -> mlua::Result<crate::native_ui::Node> {
    let mut n = crate::native_ui::Node::new(&t.get::<String>("k")?);
    n.text = t.get::<Option<String>>("t")?.unwrap_or_default();
    if let Some(s) = t.get::<Option<mlua::Table>>("s")? {
        n.style = ui_styles(&s)?;
    }
    n.value = match t.get::<mlua::Value>("v")? {
        mlua::Value::String(s) => Some(crate::native_ui::SV::Text(s.to_str()?.to_string())),
        mlua::Value::Boolean(b) => Some(crate::native_ui::SV::Truth(b)),
        mlua::Value::Integer(i) => Some(crate::native_ui::SV::Num(i as f64)),
        mlua::Value::Number(x) => Some(crate::native_ui::SV::Num(x)),
        // A colour picker's colour.
        mlua::Value::Table(c) => Some(crate::native_ui::SV::Rgba(lua_colour(&c)?)),
        _ => None,
    };
    if let Some(f) = t.get::<Option<mlua::Function>>("f")? {
        // An input's action is given its new value.
        actions.push(Box::new(move || {
            let r = match crate::native_ui::incoming() {
                crate::native_ui::SV::Text(s) => act(&f, s),
                crate::native_ui::SV::Truth(b) => act(&f, b),
                crate::native_ui::SV::Num(x) => act(&f, x),
                crate::native_ui::SV::Rgba(c) => act(&f, (c[0] as f64, c[1] as f64, c[2] as f64, c[3] as f64)),
                _ => act(&f, ()),
            };
            r.map_err(|e| raw_error(&e))
        }));
        n.action = Some(actions.len() - 1);
    }
    // Blocks for the mouse: a canvas's press, drag, release and hover are given where the mouse
    // is, a drop's block the files' paths, and a list's reorder where a row went from and to.
    for (key, name) in [
        ("P", "press"),
        ("D", "drag"),
        ("U", "release"),
        ("H", "hover"),
        ("X", "drop"),
        ("R", "reorder"),
        ("C", "click"),
        ("Q", "close"),
        ("F", "fullscreen"),
    ] {
        if let Some(f) = t.get::<Option<mlua::Function>>(key)? {
            actions.push(Box::new(move || {
                let r = match crate::native_ui::incoming() {
                    crate::native_ui::SV::Point(x, y) => act(&f, (x, y)),
                    crate::native_ui::SV::List(paths) => act(&f, mlua::Variadic::from_iter(paths)),
                    crate::native_ui::SV::Truth(b) => act(&f, b),
                    _ => act(&f, ()),
                };
                r.map_err(|e| raw_error(&e))
            }));
            n.mouse.push((name.to_string(), actions.len() - 1));
        }
    }
    if let Some(m) = t.get::<Option<mlua::Table>>("m")? {
        for item in m.sequence_values::<mlua::Table>() {
            n.menu.push(ui_node(&item?, actions)?);
        }
    }
    n.whole = t.get::<Option<bool>>("i")?.unwrap_or(false);
    // A table or list: { id, rows, first made, picked (-1 for none) }.
    if let Some(l) = t.get::<Option<mlua::Table>>("L")? {
        let pick: i64 = l.get(4)?;
        n.list = Some(crate::native_ui::ListInfo {
            id: l.get(1)?,
            count: l.get(2)?,
            first: l.get(3)?,
            pick: (pick >= 0).then_some(pick as usize),
        });
    }
    if let Some(o) = t.get::<Option<mlua::Table>>("o")? {
        for s in o.sequence_values::<String>() {
            n.options.push(s?);
        }
    }
    if let Some(f) = t.get::<Option<mlua::Function>>("e")? {
        // What Enter runs in an input.
        actions.push(Box::new(move || act(&f, ()).map_err(|e| raw_error(&e))));
        n.enter = Some(actions.len() - 1);
    }
    if let Some(c) = t.get::<Option<mlua::Table>>("c")? {
        for child in c.sequence_values::<mlua::Table>() {
            n.children.push(ui_node(&child?, actions)?);
        }
    }
    Ok(n)
}

/// Runs the program, writing its things' states to the file `states` names, if any (states.rs):
/// what Ledger's States panel shows, asked for with LUST_STATES.
pub fn run_watched(out: &Output, files: &[String], states: Option<&std::path::Path>) -> ExitCode {
    let lua = Lua::new();
    if let Err(e) = host(&lua) {
        eprintln!("couldn't start Lua: {e}");
        return ExitCode::from(1);
    }
    let feed = states.and_then(|p| match crate::states::Feed::open(p) {
        Ok(f) => Some(std::sync::Arc::new(f)),
        Err(e) => {
            eprintln!(
                "lust couldn't write the states to {} ({e}), so the program runs without sending them.",
                p.display()
            );
            None
        }
    });
    if let Some(f) = &feed {
        if let Err(e) = crate::states::install(&lua, f, &out.names) {
            eprintln!("couldn't start Lua: {e}");
            return ExitCode::from(1);
        }
    }
    let result = lua.load(&out.code).set_name("=lust").exec();
    if let Some(f) = &feed {
        f.finish();
    }
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let raw = match &e {
                mlua::Error::RuntimeError(s) => s.clone(),
                mlua::Error::CallbackError { cause, .. } => cause.to_string(),
                other => other.to_string(),
            };
            let (line, msg) = locate(&raw, &out.lines);
            let msg = crate::ast::unmark(&crate::lua::friendly(&msg));
            match line {
                Some(l) => {
                    let (f, row) = crate::ast::unpack(l);
                    eprintln!("{}:{row}  {msg}", files.get(f).map(|s| s.as_str()).unwrap_or(""));
                }
                None => eprintln!("{}  {msg}", files[0]),
            }
            ExitCode::from(1)
        }
    }
}

/// "lust:123: message" → the Lust line for Lua line 123, and the message.
fn locate(raw: &str, lines: &[u32]) -> (Option<u32>, String) {
    let first = raw.lines().next().unwrap_or("");
    if let Some(rest) = first.strip_prefix("lust:") {
        if let Some((num, msg)) = rest.split_once(": ") {
            if let Ok(n) = num.parse::<usize>() {
                let l = lines.get(n.saturating_sub(1)).copied().unwrap_or(0);
                return ((l > 0).then_some(l), msg.to_string());
            }
        }
    }
    (None, first.to_string())
}

#[cfg(test)]
pub(crate) mod tests {
    /// A table and a list of `rows` rows, opened in test mode, scrolled to the middle and the end,
    /// sorted and picked from. Gives back how long each step took.
    fn timed(rows: usize, steps: &[&str]) -> Vec<(String, std::time::Duration)> {
        let src = format!(
            "class Item {{\n    has {{\n        int n = 0;\n        string name = \"\";\n    }}\n}}\n\
             object Store {{\n    has {{\n        Item array items = [];\n    }}\n}}\n\
             loop (int i = 0; i < {rows}; i += 1) {{\n    Store.items:inject(new Item {{ n = i, name = \"Item \" + i.text() }});\n}}\n\
             assembly App {{\n    has {{\n        Item chosen = nothing;\n        Item other = nothing;\n    }}\n\
             view [title \"Big\", width 800, height 600] {{\n        split {{\n\
             table (Item i in Store.items) [select chosen] {{\n                column \"N\" i.n [width 80];\n                column \"Name\" i.name;\n            }}\n\
             list (Item i in Store.items) [select other, rowHeight 28] {{\n                text i.name;\n            }}\n        }}\n    }}\n}}\n\
             App:open();\nend\n"
        );
        let dir = std::env::temp_dir().join(format!("lust-rows-{}-{rows}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("main.lst");
        std::fs::write(&path, src).unwrap();
        let c = crate::cli::compile(&path, false).expect("the timing test's program checks");
        std::env::set_var("LUST_VIEW_TEST", "1");
        crate::native_ui::test_commands(steps.iter().map(|s| s.to_string()).collect());
        let code = super::run_watched(&c.out, &c.files, None);
        assert_eq!(code, std::process::ExitCode::SUCCESS);
        let _ = std::fs::remove_dir_all(&dir);
        crate::native_ui::test_times()
    }

    /// 100,000 rows: each frame (a scroll and drawing again, or a pick) takes under 16 ms, sorted
    /// or not, and opening the view takes about as long as with 1,000 rows. Timed in release builds only,
    /// as debug builds are far slower: `cargo test --release rows_stay_quick -- --nocapture`.
    #[test]
    fn a_hundred_thousand_rows_stay_quick() {
        let steps = [
            "paint 1",
            "scroll table 1 1500000",
            "paint 1",
            "scroll list 1 1400000",
            "paint 1",
            "scroll table 1 99999999",
            "scroll list 1 99999999",
            "paint 1",
            "click Item 99999",
            "paint 1",
            "key up",
            "paint 1",
            // Sorting reads every row once. The frame after it can be slower too, as Lua tidies
            // away what the sort used; the frames after that are timed.
            "click N",
            "click N",
            "scroll table 1 -900000",
            "scroll table 1 -900000",
            "paint 1",
            "scroll table 1 2000",
            "paint 1",
        ];
        // A frame: what a step does, then drawing the window. What went over 16 ms, if anything.
        let slow = |times: &[(String, std::time::Duration)]| -> Option<String> {
            if times[0].1.as_secs_f64() >= 0.016 {
                return Some(format!("opening took {:?}", times[0].1));
            }
            times
                .windows(2)
                .find(|w| w[1].0 == "paint 1" && (w[0].1 + w[1].1).as_secs_f64() >= 0.016)
                .map(|w| format!("{} then drawing took {:?}", w[0].0, w[0].1 + w[1].1))
        };
        let small = timed(1_000, &steps);
        let mut big = timed(100_000, &steps);
        // A busy computer can make one run slow, so it gets three tries; something really slow
        // is slow every time.
        for _ in 0..2 {
            if slow(&big).is_none() {
                break;
            }
            big = timed(100_000, &steps);
        }
        for ((what, s), (_, b)) in small.iter().zip(&big) {
            eprintln!(
                "{what:<26} 1,000 rows: {:>6.2} ms   100,000 rows: {:>6.2} ms",
                s.as_secs_f64() * 1e3,
                b.as_secs_f64() * 1e3
            );
        }
        assert_eq!(big.len(), steps.len() + 1);
        if !cfg!(debug_assertions) {
            if let Some(why) = slow(&big) {
                panic!("{why}");
            }
        }
    }

    /// A local website for the tests: /slow answers after 150 ms, /missing is 404, /secret is
    /// 401, and anything else answers with its own path. Gives back its address.
    pub(crate) fn site() -> String {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let at = format!("http://{}", server.server_addr().to_ip().unwrap());
        std::thread::spawn(move || {
            for req in server.incoming_requests() {
                std::thread::spawn(move || {
                    let path = req.url().to_string();
                    let status = match path.as_str() {
                        "/missing" => 404,
                        "/secret" => 401,
                        _ => 200,
                    };
                    if path == "/slow" {
                        std::thread::sleep(std::time::Duration::from_millis(150));
                    }
                    let _ = req.respond(tiny_http::Response::from_string(format!("page {path}")).with_status_code(status));
                });
            }
        });
        at
    }

    /// Web:get in lust run pauses only the run that asks: the program's own code goes on while
    /// two runs wait for their answers, which come back as they were sent, and a 404 is NotFound.
    #[test]
    fn web_requests_pause_only_their_run() {
        let at = site();
        let dir = std::env::temp_dir().join(format!("lust-web-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("log.txt");
        let logged = log.to_string_lossy().replace('\\', "/");
        let src = format!(
            "method grab(string name, string address) {{\n    string page = Web:get(address);\n    Files:append(\"{logged}\", name + \" \" + page + \"\\n\");\n}}\n\
             method missing() {{\n    try {{\n        string page = Web:get(\"{at}/missing\");\n    }} error (Problem p, string m) {{\n        Files:append(\"{logged}\", p.text() + \"\\n\");\n    }}\n}}\n\
             Files:write(\"{logged}\", \"\");\n\
             start grab(\"a\", \"{at}/slow\");\nstart grab(\"b\", \"{at}/quick\");\nstart missing();\n\
             Files:append(\"{logged}\", \"top\\n\");\n\
             string mine = Web:post(\"{at}/mine\", \"{{}}\");\nFiles:append(\"{logged}\", \"top \" + mine + \"\\n\");\nend\n"
        );
        let path = dir.join("main.lst");
        std::fs::write(&path, src).unwrap();
        let c = crate::cli::compile(&path, false).expect("the web test's program checks");
        assert_eq!(super::run_watched(&c.out, &c.files, None), std::process::ExitCode::SUCCESS);
        let text = std::fs::read_to_string(&log).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let _ = std::fs::remove_dir_all(&dir);
        // The top went on before any answer came, and the program waited for every run.
        assert_eq!(lines.first(), Some(&"top"), "{text}");
        for want in ["a page /slow", "b page /quick", "NotFound", "top page /mine"] {
            assert!(lines.contains(&want), "{want} is missing from:\n{text}");
        }
        assert_eq!(lines.len(), 5, "{text}");
    }
    /// With LUST_STATES, lust run writes each thing with a state of its own: made (named after its
    /// variable, field or object), every change, and the end of what's made in a state; then stop.    /// Without it, nothing is written.
    #[test]
    fn the_states_feed_lists_things_and_their_changes() {
        let dir = std::env::temp_dir().join(format!("lust-states-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("main.lst");
        std::fs::write(&path, FEED_PROGRAM).unwrap();
        let c = crate::cli::compile(&path, false).expect("the states test's program checks");
        let feed = dir.join("states.jsonl");
        assert_eq!(super::run_watched(&c.out, &c.files, Some(&feed)), std::process::ExitCode::SUCCESS);
        let text = std::fs::read_to_string(&feed).unwrap();
        // The times differ from run to run.
        let lines: Vec<String> = text
            .lines()
            .map(|l| match l.find(",\"ms\":") {
                Some(i) => {
                    let rest = &l[i + 6..];
                    format!(
                        "{}{}",
                        &l[..i],
                        &rest[rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len())..]
                    )
                }
                None => l.to_string(),
            })
            .collect();
        let want = [
            format!("{{\"ev\":\"start\",\"v\":1,\"lust\":\"{}\"}}", env!("CARGO_PKG_VERSION")),
            r#"{"ev":"made","id":1,"class":"Game","name":"Game","opt":"Calm"}"#.into(),
            r#"{"ev":"made","id":2,"class":"Hero","name":"scout","opt":"Calm"}"#.into(),
            r#"{"ev":"made","id":3,"class":"Hero","name":"ada","opt":"Battle.Melee"}"#.into(),
            r#"{"ev":"set","id":2,"from":"Calm","opt":"Battle.Melee"}"#.into(),
            r#"{"ev":"made","id":4,"class":"Torch","name":"torch","opt":"Off"}"#.into(),
            r#"{"ev":"set","id":4,"from":"Off","opt":"On"}"#.into(),
            r#"{"ev":"made","id":5,"class":"Torch","name":"spare","opt":"Off"}"#.into(),
            r#"{"ev":"set","id":5,"from":"Off","opt":"On"}"#.into(),
            r#"{"ev":"set","id":2,"from":"Battle.Melee","opt":"Battle.Ranged"}"#.into(),
            r#"{"ev":"end","id":5,"how":"state"}"#.into(),
            r#"{"ev":"set","id":2,"from":"Battle.Ranged","opt":"Calm"}"#.into(),
            r#"{"ev":"made","id":6,"class":"Hero","name":"in squad","opt":"Dead"}"#.into(),
            r#"{"ev":"stop"}"#.into(),
        ];
        assert_eq!(lines, want, "{text}");
        // Off: no file.
        let _ = std::fs::remove_file(&feed);
        assert_eq!(super::run_watched(&c.out, &c.files, None), std::process::ExitCode::SUCCESS);
        assert!(!feed.exists());
        // A feed that can't be written: the program still runs.
        let nowhere = dir.join("no-such-folder").join("states.jsonl");
        assert_eq!(
            super::run_watched(&c.out, &c.files, Some(&nowhere)),
            std::process::ExitCode::SUCCESS
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    const FEED_PROGRAM: &str = r#"state Phase = [
    Calm -> Battle,
    Battle [
        Melee -> Battle.Ranged,
        Ranged -> [Battle.Melee, Calm],
    ] -> Dead,
    Dead,
];

state Glow = [Off, On];

class Torch {
    has {
        state = Glow.Off;
    }
    method light() {
        state = Glow.On;
    }
}

class Hero {
    has {
        state = Phase.Calm;
        string name = "";
        Torch torch = nothing;
    }
    method fight() {
        state = Phase.Battle.Melee;
        keep new Torch in torch;
        torch:light();
        Torch spare = new Torch;
        spare:light();
    }
    method rest() {
        state = Phase.Battle.Ranged;
        state = Phase.Calm;
    }
}

object Game {
    has {
        state = Phase.Calm;
    }
}

Hero scout = new Hero { name = "Scout" };
Hero ada = new Hero in Phase.Battle.Melee;
Hero array squad = [];
scout:fight();
scout:rest();
keep (new Hero in Phase.Dead) in squad;
print("done");
end
"#;
}
