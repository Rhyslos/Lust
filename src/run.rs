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
    h.set("remove", lua.create_function(|_, p: String| Ok(rt::fs_remove(&p).err()))?)?;
    h.set(
        "list",
        lua.create_function(|_, p: String| {
            Ok(match rt::fs_list(&p) {
                Ok(names) => (Some(names), None),
                Err(e) => (None, Some(e)),
            })
        })?,
    )?;
    h.set("mkdir", lua.create_function(|_, p: String| Ok(rt::fs_mkdir(&p).err()))?)?;
    h.set("now", lua.create_function(|_, ()| Ok(rt::time_now()))?)?;
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
    lua.globals().set("LUST_HOST", h)
}

pub fn run(out: &Output, files: &[String]) -> ExitCode {
    let lua = Lua::new();
    if let Err(e) = host(&lua) {
        eprintln!("couldn't start Lua: {e}");
        return ExitCode::from(1);
    }
    let result = lua.load(&out.code).set_name("=lust").exec();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let raw = match &e {
                mlua::Error::RuntimeError(s) => s.clone(),
                mlua::Error::CallbackError { cause, .. } => cause.to_string(),
                other => other.to_string(),
            };
            let (line, msg) = locate(&raw, &out.lines);
            let msg = crate::ast::unmark(&friendly(&msg));
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

fn friendly(msg: &str) -> String {
    if msg.contains("attempt to index") && msg.contains("nil value") {
        return "this is nothing, so it has no fields or methods. Check first: is it !== nothing?".into();
    }
    if msg.contains("attempt to call") && msg.contains("nil value") {
        return "this hook is nothing, so it can't be called. Check first: is it !== nothing?".into();
    }
    if msg.contains("stack overflow") {
        return "methods called each other too deeply. That usually means one calls itself with no way to stop.".into();
    }
    if msg.contains("attempt to perform arithmetic") || msg.contains("attempt to concatenate") || msg.contains("attempt to compare") {
        return "a value here is nothing, so it can't be used in maths or text.".into();
    }
    msg.to_string()
}
