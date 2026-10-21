//! Runs the Lua a program turns into, with Lua 5.1 built into lust itself, and
//! turns Lua's errors back into Lust lines and plain words.

use std::process::ExitCode;

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
