//! Runs Lust the way an engine does: hands over files, loads the Lua into Luau, makes components,
//! ticks them, delivers events, swaps in edited code, and points at failures.
//! `cargo run` compares what it prints with expected.out; `cargo run -- --print` just prints.

use mlua::{Function, Lua, Table, Value};
use std::cell::RefCell;
use std::fmt::Write;
use std::path::Path;
use std::rc::Rc;

/// The engine reads the game's files itself, and hands over each one's path and text.
fn files(dir: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(base, &p, out);
            } else {
                let rel = p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/");
                out.push((rel, std::fs::read_to_string(&p).unwrap()));
            }
        }
    }
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    walk(&base, &base, &mut out);
    out
}

fn load(lua: &Lua, chunk: &lust::LuaChunk) -> Table {
    lua.load(&chunk.code).set_name(lust::LuaChunk::NAME).eval().unwrap()
}

fn main() {
    let mut out = String::new();
    let log = Rc::new(RefCell::new(String::new()));
    writeln!(out, "compiler {}", lust::VERSION.split('-').next().unwrap()).unwrap();

    // 1. Mistakes: every one, with file, line and column.
    match lust::check("main.lst", &files("broken")) {
        Ok(()) => writeln!(out, "no mistakes?").unwrap(),
        Err(problems) => {
            for p in &problems {
                writeln!(out, "{p}").unwrap();
                writeln!(out, "  = {} {} {}", p.file, p.line, p.column).unwrap();
            }
        }
    }
    writeln!(out, "{:?}", lust::check("nope.lst", &files("broken")).unwrap_err()[0].message).unwrap();

    // 2. The game in Luau.
    let lua = Lua::new();
    let natives = lua.create_table().unwrap();
    let l = log.clone();
    natives
        .set(
            "log",
            lua.create_function(move |_, words: String| {
                writeln!(l.borrow_mut(), "{words}").unwrap();
                Ok(())
            })
            .unwrap(),
        )
        .unwrap();
    natives
        .set(
            "nearest",
            lua.create_function(|_, _from: Option<String>| Ok("0000000000002")).unwrap(),
        )
        .unwrap();
    let queue = lua.create_table().unwrap();
    lua.globals().set("QUEUE", queue.clone()).unwrap();
    let send: Function = lua
        .load("return function(to, name, values) QUEUE[#QUEUE + 1] = { to = to, name = name, values = values } end")
        .eval()
        .unwrap();
    natives.set("send_event", send).unwrap();
    lua.globals().set("LUST_NATIVES", natives).unwrap();

    let v1 = lust::to_lua("main.lst", &files("v1")).unwrap();
    let api1 = load(&lua, &v1);
    let new: Function = api1.get("new").unwrap();
    let turret: Table = new.call(("Turret", "0000000000001")).unwrap();
    let shield: Table = new.call(("Shield", "0000000000002")).unwrap();
    let events: Table = api1.get("events").unwrap();
    let hit: Table = events.get("Hit").unwrap();
    for f in hit.sequence_values::<Table>() {
        let f = f.unwrap();
        writeln!(out, "Hit carries {} {}", f.get::<String>(2).unwrap(), f.get::<String>(3).unwrap()).unwrap();
    }

    for tick in 0..4 {
        writeln!(out, "tick {tick}").unwrap();
        let update: Function = turret.get("update").unwrap();
        update.call::<()>((turret.clone(), 1.0 / 60.0)).unwrap();
        // Deliver the queue, in order; events sent while delivering join the end.
        let mut i = 1;
        while let Ok(Value::Table(e)) = queue.get::<Value>(i) {
            let name: String = e.get("name").unwrap();
            let handle: Function = shield.get("handle").unwrap();
            match handle.call::<bool>((shield.clone(), name.clone(), e.get::<Table>("values").unwrap())) {
                Ok(handled) => writeln!(out, "  {name} handled: {handled}").unwrap(),
                Err(err) => {
                    // A failure points at the Lust: file, line and column.
                    let at = v1.locate(&err.to_string());
                    writeln!(out, "  {at}").unwrap();
                    writeln!(out, "  = {} {:?} {:?}", at.file, at.line, at.column).unwrap();
                }
            }
            i += 1;
        }
        for k in 1..i {
            queue.set(k, Value::Nil).unwrap();
        }
        out.push_str(
            &log.borrow_mut()
                .split_off(0)
                .lines()
                .map(|l| format!("  {l}\n"))
                .collect::<String>(),
        );
    }

    // 3. The code changes while the game runs: swap, keeping what can be kept.
    let v2 = lust::to_lua("main.lst", &files("v2")).unwrap();
    let api2 = load(&lua, &v2);
    let changes: Table = api2.get::<Function>("changes").unwrap().call(api1.clone()).unwrap();
    for what in ["changed", "removed", "added"] {
        let names: Vec<String> = changes.get::<Table>(what).unwrap().sequence_values().map(|n| n.unwrap()).collect();
        writeln!(out, "{what}: {names:?}").unwrap();
    }
    let swap: Function = api2.get("swap").unwrap();
    for (name, c) in [("turret", &turret), ("shield", &shield)] {
        let (now, notes): (Value, Table) = swap.call(c.clone()).unwrap();
        writeln!(out, "swap {name}: {}", if now.is_nil() { "gone" } else { "kept" }).unwrap();
        for n in notes.sequence_values::<String>() {
            writeln!(out, "  note: {}", n.unwrap()).unwrap();
        }
    }
    for k in ["me", "ammo", "heat", "mode", "target", "range", "label"] {
        writeln!(out, "  turret.{k} = {:?}", turret.get::<Value>(k).unwrap()).unwrap();
    }
    let update: Function = turret.get("update").unwrap();
    update.call::<()>((turret.clone(), 0.5)).unwrap();
    out.push_str(&log.borrow_mut().split_off(0));

    let expected_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("expected.out");
    if std::env::args().any(|a| a == "--print") {
        print!("{out}");
        return;
    }
    let expected = std::fs::read_to_string(&expected_path).unwrap_or_default().replace("\r\n", "\n");
    if expected != out {
        println!("expected:\n{expected}\nbut printed:\n{out}");
        std::process::exit(1);
    }
    println!("engine test passed");
}
