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
    // A vec3 goes both ways as a Luau vector.
    natives
        .set(
            "positionOf",
            lua.create_function(|_, _e: Option<String>| Ok(mlua::Vector::new(4.0, 0.1, -2.0)))
                .unwrap(),
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
    writeln!(out, "vectors: {}", api1.get::<String>("vectors").unwrap()).unwrap();
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
            writeln!(
                out,
                "  {name} carries {:?}",
                e.get::<Table>("values").unwrap().get::<Value>(3).unwrap()
            )
            .unwrap();
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
    for k in ["me", "ammo", "heat", "mode", "target", "range", "label", "aim"] {
        writeln!(out, "  turret.{k} = {:?}", turret.get::<Value>(k).unwrap()).unwrap();
    }
    let update: Function = turret.get("update").unwrap();
    update.call::<()>((turret.clone(), 0.5)).unwrap();
    out.push_str(&log.borrow_mut().split_off(0));

    // 4. The engine's own folder gives entity abilities, called bare inside components.
    let engine = lust::Engine {
        folders: vec!["solis".into()],
        records: false,
    };
    let hop = lust::to_lua_with(&engine, "main.lst", &files("abilities")).unwrap();
    for n in &hop.notes {
        writeln!(out, "note: {n}").unwrap();
    }
    let places = lua.create_table().unwrap();
    lua.globals().set("PLACES", places).unwrap();
    let natives: Table = lua.globals().get("LUST_NATIVES").unwrap();
    for (name, body) in [
        (
            "entity_position",
            "return function(e) return PLACES[e] or vector.create(0, 0, 0) end",
        ),
        ("entity_moveTo", "return function(e, to) PLACES[e] = to end"),
        (
            "entity_jump",
            "return function(e) LUST_NATIVES.log('the engine makes ' .. e .. ' jump') end",
        ),
        ("entity_velocity", "return function(e) return 2.5 end"),
        (
            "FarmerAnimator_play",
            "return function(a, clip) LUST_NATIVES.log('animator ' .. a.id .. ' plays ' .. clip) end",
        ),
    ] {
        let f: Function = lua.load(body).eval().unwrap();
        natives.set(name, f).unwrap();
    }
    let api3 = load(&lua, &hop);
    let hopper: Table = api3.get::<Function>("new").unwrap().call(("Hopper", "0000000000009")).unwrap();
    let update: Function = hopper.get("update").unwrap();
    update.call::<()>((hopper.clone(), 0.5)).unwrap();
    let bits: Table = api3.get::<Function>("new").unwrap().call(("Bits", "0000000000010")).unwrap();
    let update: Function = bits.get("update").unwrap();
    update.call::<()>((bits.clone(), 0.5)).unwrap();
    out.push_str(&log.borrow_mut().split_off(0));

    // 5. A field the engine fills in: its kind is "object", with its class; the engine makes one
    // of its own classes with make, and fills the field before the first update.
    let comps: Table = api3.get("components").unwrap();
    let farmer_class: Table = comps.get("Farmer").unwrap();
    for f in farmer_class.get::<Table>("__schema").unwrap().sequence_values::<Table>() {
        let f = f.unwrap();
        writeln!(
            out,
            "Farmer's {} is {} {}",
            f.get::<String>(2).unwrap(),
            f.get::<String>(3).unwrap(),
            f.get::<String>(4).unwrap()
        )
        .unwrap();
    }
    let farmer: Table = api3.get::<Function>("new").unwrap().call(("Farmer", "0000000000004")).unwrap();
    // Before it's filled, reading it stops, naming the field, on its line.
    let early: Function = farmer.get("update").unwrap();
    if let Err(err) = early.call::<()>((farmer.clone(), 0.5)) {
        writeln!(out, "{}", hop.locate(&err.to_string())).unwrap();
    }
    let values = lua.create_table().unwrap();
    values.set("id", 12).unwrap();
    let anim: Table = api3.get::<Function>("make").unwrap().call(("FarmerAnimator", values)).unwrap();
    farmer.set("anim", anim).unwrap();
    let update: Function = farmer.get("update").unwrap();
    update.call::<()>((farmer.clone(), 0.5)).unwrap();
    out.push_str(&log.borrow_mut().split_off(0));

    // 6. Objects: the engine ticks them (name order, before components) and saves their fields.
    let names: Vec<String> = api3
        .get::<Table>("objectNames")
        .unwrap()
        .sequence_values()
        .map(|n| n.unwrap())
        .collect();
    writeln!(out, "objects: {names:?}").unwrap();
    let tick: Function = api3.get("tick").unwrap();
    let save: Function = api3.get("save").unwrap();
    let load_back: Function = api3.get("load").unwrap();
    tick.call::<()>(1.0).unwrap();
    let saved: Table = save.call("Clock").unwrap();
    writeln!(
        out,
        "saved: day {} hour {}",
        saved.get::<i64>("day").unwrap(),
        saved.get::<f64>("hour").unwrap()
    )
    .unwrap();
    tick.call::<()>(10.0).unwrap();
    load_back.call::<()>(("Clock", saved)).unwrap();
    let clock: Table = api3.get::<Table>("objects").unwrap().get("Clock").unwrap();
    writeln!(
        out,
        "loaded: day {} hour {}",
        clock.get::<i64>("day").unwrap(),
        clock.get::<f64>("hour").unwrap()
    )
    .unwrap();
    // A save of the wrong kinds (text for a number, a number with a fraction for an int) leaves
    // those fields as they are, as Rust's set_data does.
    let wrong = lua.create_table().unwrap();
    wrong.set("day", "three").unwrap();
    wrong.set("hour", 7.0).unwrap();
    load_back.call::<()>(("Clock", wrong)).unwrap();
    let odd = lua.create_table().unwrap();
    odd.set("day", 2.5).unwrap();
    load_back.call::<()>(("Clock", odd)).unwrap();
    writeln!(
        out,
        "wrong kinds kept: day {} hour {}",
        clock.get::<i64>("day").unwrap(),
        clock.get::<f64>("hour").unwrap()
    )
    .unwrap();
    // An object the engine doesn't keep is a clear stop.
    let err = save.call::<Table>("Weather").unwrap_err().to_string();
    writeln!(
        out,
        "Weather: {}",
        err.lines().next().unwrap_or("").trim_start_matches("runtime error: ")
    )
    .unwrap();

    // 7. vec2s and colours: a vec2 is a table { x, y } and a colour { r, g, b, a } in Luau too. A
    // native gives them back as tables (a colour's a may be left out), and a part above 1 is kept at 1.
    for (name, body) in [
        ("spotOf", "return function(e) return { x = 1.5, y = 2 } end"),
        ("tintOf", "return function(e) return { r = 2, g = 0.25, b = 0.1 } end"),
    ] {
        let f: Function = lua.load(body).eval().unwrap();
        natives.set(name, f).unwrap();
    }
    let flat = lust::to_lua("main.lst", &files("flat")).unwrap();
    let api4 = load(&lua, &flat);
    let lamp_class: Table = api4.get::<Table>("components").unwrap().get("Lamp").unwrap();
    for f in lamp_class.get::<Table>("__schema").unwrap().sequence_values::<Table>() {
        let f = f.unwrap();
        writeln!(out, "Lamp's {} is {}", f.get::<String>(2).unwrap(), f.get::<String>(3).unwrap()).unwrap();
    }
    let glow: Table = api4.get::<Table>("events").unwrap().get("Glow").unwrap();
    for f in glow.sequence_values::<Table>() {
        let f = f.unwrap();
        writeln!(out, "Glow carries {} {}", f.get::<String>(2).unwrap(), f.get::<String>(3).unwrap()).unwrap();
    }
    let lamp: Table = api4.get::<Function>("new").unwrap().call(("Lamp", "0000000000005")).unwrap();
    let light: Table = lamp.get("light").unwrap();
    writeln!(
        out,
        "light r {} g {} b {} a {}",
        light.get::<f64>("r").unwrap(),
        light.get::<f64>("g").unwrap(),
        light.get::<f64>("b").unwrap(),
        light.get::<f64>("a").unwrap()
    )
    .unwrap();
    let update: Function = lamp.get("update").unwrap();
    update.call::<()>((lamp.clone(), 0.5)).unwrap();
    out.push_str(&log.borrow_mut().split_off(0));
    let mut i = 1;
    while let Ok(Value::Table(e)) = queue.get::<Value>(i) {
        let values: Table = e.get("values").unwrap();
        let tint: Table = values.get(1).unwrap();
        let at: Table = values.get(2).unwrap();
        writeln!(
            out,
            "{} carries tint a {} and at x {} y {}",
            e.get::<String>("name").unwrap(),
            tint.get::<f64>("a").unwrap(),
            at.get::<f64>("x").unwrap(),
            at.get::<f64>("y").unwrap()
        )
        .unwrap();
        queue.set(i, Value::Nil).unwrap();
        i += 1;
    }

    // 8. An object's vec3, vec2, rotation and colour, saved by the engine and loaded back exactly;
    // in Luau they go through JSON too. Values of the wrong shape are left as they are.
    let camp_names: Vec<String> = api4
        .get::<Table>("objectNames")
        .unwrap()
        .sequence_values()
        .map(|n| n.unwrap())
        .collect();
    writeln!(out, "objects: {camp_names:?}").unwrap();
    let tick: Function = api4.get("tick").unwrap();
    let save: Function = api4.get("save").unwrap();
    let load_back: Function = api4.get("load").unwrap();
    tick.call::<()>(1.0).unwrap();
    out.push_str(&log.borrow_mut().split_off(0));
    let camp: Table = api4.get::<Table>("objects").unwrap().get("Camp").unwrap();
    let parts = |v: &Value, keys: &[&str]| -> Vec<u32> {
        keys.iter()
            .map(|k| match v {
                Value::Vector(x) => match *k {
                    "x" => x.x(),
                    "y" => x.y(),
                    _ => x.z(),
                }
                .to_bits(),
                Value::Table(t) => (t.get::<f64>(*k).unwrap() as f32).to_bits(),
                _ => 0,
            })
            .collect()
    };
    let fields: [(&str, &[&str]); 4] = [
        ("fire", &["x", "y", "z"]),
        ("cell", &["x", "y"]),
        ("facing", &["x", "y", "z", "w"]),
        ("glow", &["r", "g", "b", "a"]),
    ];
    let now = |camp: &Table| -> Vec<Vec<u32>> { fields.iter().map(|(f, k)| parts(&camp.get::<Value>(*f).unwrap(), k)).collect() };
    let saved: Table = save.call("Camp").unwrap();
    let before = now(&camp);
    writeln!(
        out,
        "camp saved {}",
        fields
            .iter()
            .map(|(f, _)| *f)
            .filter(|f| saved.contains_key(*f).unwrap())
            .collect::<Vec<_>>()
            .join(", ")
    )
    .unwrap();
    tick.call::<()>(1.0).unwrap();
    log.borrow_mut().clear();
    writeln!(out, "camp changed: {}", now(&camp) != before).unwrap();
    load_back.call::<()>(("Camp", saved)).unwrap();
    writeln!(out, "camp loaded exactly: {}", now(&camp) == before).unwrap();
    // A save the engine kept as plain tables of numbers, the way it writes them to disk.
    let plain = lua.create_table().unwrap();
    let fire = lua.create_table().unwrap();
    fire.set("x", 0.1).unwrap();
    fire.set("y", 0.2).unwrap();
    fire.set("z", 0.3).unwrap();
    plain.set("fire", fire).unwrap();
    let glow = lua.create_table().unwrap();
    glow.set("r", 2.0).unwrap();
    glow.set("g", 0.5).unwrap();
    glow.set("b", 0.25).unwrap();
    plain.set("glow", glow).unwrap();
    load_back.call::<()>(("Camp", plain)).unwrap();
    let f = parts(&camp.get::<Value>("fire").unwrap(), &["x", "y", "z"]);
    writeln!(out, "camp fire is f32s: {}", f == [0.1f32, 0.2, 0.3].map(f32::to_bits)).unwrap();
    let g: Table = camp.get("glow").unwrap();
    writeln!(out, "camp glow r {} a {}", g.get::<f64>("r").unwrap(), g.get::<f64>("a").unwrap()).unwrap();
    // Wrong shapes: too few numbers, text, a table without w, a colour without b.
    let wrong = lua.create_table().unwrap();
    let short = lua.create_table().unwrap();
    short.set("x", 1.0).unwrap();
    short.set("y", 2.0).unwrap();
    wrong.set("fire", short.clone()).unwrap();
    wrong.set("cell", "4, 5").unwrap();
    let nw = lua.create_table().unwrap();
    nw.set("x", 0.0).unwrap();
    nw.set("y", 0.0).unwrap();
    nw.set("z", 0.0).unwrap();
    wrong.set("facing", nw).unwrap();
    let rg = lua.create_table().unwrap();
    rg.set("r", 0.0).unwrap();
    rg.set("g", 0.0).unwrap();
    wrong.set("glow", rg).unwrap();
    let kept = now(&camp);
    load_back.call::<()>(("Camp", wrong)).unwrap();
    writeln!(out, "camp wrong shapes kept: {}", now(&camp) == kept).unwrap();

    // 9. Tweens: the engine moves them on with tickTweens once a tick, after the objects' tick
    // and before the components' updates. A tween on a component's field belongs to it.
    let tw = lust::to_lua("main.lst", &files("tweens")).unwrap();
    let api5 = load(&lua, &tw);
    let new: Function = api5.get("new").unwrap();
    let doors: Vec<Table> = ["0000000000011", "0000000000012"]
        .iter()
        .map(|e| new.call(("Door", *e)).unwrap())
        .collect();
    let tick: Function = api5.get("tick").unwrap();
    let tick_tweens: Function = api5.get("tickTweens").unwrap();
    let sky: Table = api5.get::<Table>("objects").unwrap().get("Sky").unwrap();
    for n in 0..8 {
        tick.call::<()>(0.1).unwrap();
        tick_tweens.call::<()>(0.1).unwrap();
        for d in &doors {
            d.get::<Function>("update").unwrap().call::<()>((d.clone(), 0.1)).unwrap();
        }
        if n == 2 {
            let shut: Function = doors[0].get("handle").unwrap();
            shut.call::<bool>((doors[0].clone(), "Shut", lua.create_table().unwrap())).unwrap();
        }
        out.push_str(&log.borrow_mut().split_off(0));
        let tint: Table = sky.get("tint").unwrap();
        writeln!(
            out,
            "tick {n}: open {} and {}, opened {} and {}, tint r {} g {}",
            doors[0].get::<f64>("open").unwrap(),
            doors[1].get::<f64>("open").unwrap(),
            doors[0].get::<i64>("opened").unwrap(),
            doors[1].get::<i64>("opened").unwrap(),
            tint.get::<f64>("r").unwrap(),
            tint.get::<f64>("g").unwrap()
        )
        .unwrap();
    }

    // 10. Runs: started work, event handlers and tweens' done blocks wait across ticks, on the
    // engine's own time (tickTweens' dt), while the engine keeps calling every update. The same
    // game and ticks as tests/library-wait/runs, which builds it as a Rust library.
    let rn = lust::to_lua("main.lst", &files("runs")).unwrap();
    let api6 = load(&lua, &rn);
    let new: Function = api6.get("new").unwrap();
    let gate: Table = new.call(("Gate", "0000000000001")).unwrap();
    let guard: Table = new.call(("Guard", "0000000000002")).unwrap();
    let tick: Function = api6.get("tick").unwrap();
    let tick_tweens: Function = api6.get("tickTweens").unwrap();
    let removed: Function = api6.get("removed").unwrap();
    let dts = [
        0.125, 0.125, 0.125, 0.125, 0.0, 0.0, 0.125, 0.0625, 0.0625, 0.125, 0.125, 0.5, 0.5, 0.5, 0.5,
    ];
    let mut guarding = true;
    let call_update = |c: &Table, dt: f64| c.get::<Function>("update").unwrap().call::<()>((c.clone(), dt)).unwrap();
    for (k, dt) in dts.iter().enumerate() {
        writeln!(out, "tick {k} (dt {dt})").unwrap();
        tick.call::<()>(*dt).unwrap();
        tick_tweens.call::<()>(*dt).unwrap();
        call_update(&gate, *dt);
        if guarding {
            call_update(&guard, *dt);
        }
        if k == 1 {
            for c in [&gate, &guard] {
                let values = lua.create_table().unwrap();
                values.set(1, 3).unwrap();
                values.set("n", 1).unwrap();
                c.get::<Function>("handle")
                    .unwrap()
                    .call::<bool>((c.clone(), "Knock", values))
                    .unwrap();
            }
        }
        if k == 3 {
            // The guard's entity is destroyed: its patrol and its waiting handler stop.
            removed.call::<()>(guard.clone()).unwrap();
            guarding = false;
        }
        if k == 6 {
            let lever: Table = api6.get::<Table>("objects").unwrap().get("Lever").unwrap();
            lever.set("pulled", true).unwrap();
        }
        out.push_str(
            &log.borrow_mut()
                .split_off(0)
                .lines()
                .map(|l| format!("  {l}\n"))
                .collect::<String>(),
        );
    }
    writeln!(out, "gate opened Int({})", gate.get::<i64>("opened").unwrap()).unwrap();
    let sleepy: Table = new.call(("Sleepy", "0000000000003")).unwrap();
    match sleepy.get::<Function>("update").unwrap().call::<()>((sleepy.clone(), 0.125)) {
        Ok(()) => writeln!(out, "Sleepy waited?").unwrap(),
        Err(err) => writeln!(out, "Lust said: {}", rn.locate(&err.to_string())).unwrap(),
    }

    // 11. Screens, as an engine shows them: a Lua of their own, whose LUST_HOST.ui_open keeps
    // each view and gives back straight away, and the engine draws it after each tick, reading
    // the tree. Pieces inside pieces (ClassPick uses ClassCard, which uses Badge) keep their
    // fields between drawings, as they do in lust run (tests/view-nested).
    let screens = Lua::new();
    let kept: Rc<RefCell<Vec<Function>>> = Rc::default();
    let k = kept.clone();
    let host = screens.create_table().unwrap();
    host.set(
        "ui_open",
        screens
            .create_function(move |_, (_settings, view): (Value, Function)| {
                k.borrow_mut().push(view);
                Ok(())
            })
            .unwrap(),
    )
    .unwrap();
    screens.globals().set("LUST_HOST", host).unwrap();
    let sc = lust::to_lua("main.lst", &files("screens")).unwrap();
    load(&screens, &sc);
    let view = kept.borrow()[0].clone();
    fn shown(t: &Table, depth: usize, out: &mut String) {
        let kind: String = t.get("k").unwrap();
        let text: Option<String> = t.get("t").unwrap();
        writeln!(out, "{}{kind} {}", "  ".repeat(depth), text.unwrap_or_default()).unwrap();
        if let Some(c) = t.get::<Option<Table>>("c").unwrap() {
            for child in c.sequence_values::<Table>() {
                shown(&child.unwrap(), depth + 1, out);
            }
        }
    }
    fn press(t: &Table, label: &str) -> bool {
        if t.get::<Option<String>>("t").unwrap().as_deref() == Some(label) {
            if let Some(f) = t.get::<Option<Function>>("f").unwrap() {
                f.call::<()>(()).unwrap();
                return true;
            }
        }
        t.get::<Option<Table>>("c")
            .unwrap()
            .is_some_and(|c| c.sequence_values::<Table>().any(|x| press(&x.unwrap(), label)))
    }
    for (n, click) in ["Pick Archer", "badge of Knight 0", ""].iter().enumerate() {
        match view.call::<Table>(()) {
            Ok(tree) => {
                writeln!(out, "screen drawing {n}").unwrap();
                shown(&tree, 1, &mut out);
                if !click.is_empty() {
                    writeln!(out, "  pressed {click}: {}", press(&tree, click)).unwrap();
                }
            }
            Err(err) => writeln!(out, "screen failed: {}", sc.locate(&err.to_string())).unwrap(),
        }
    }

    // 11. Records, in an engine that keeps them: a component's record field in its schema, saved
    // and loaded back (a copy each way), carried in events both ways, and kept by a swap.
    let without = lust::to_lua("main.lst", &files("records")).unwrap_err();
    writeln!(out, "without records: {}", without[0].message).unwrap();
    let keeps = lust::Engine {
        folders: vec![],
        records: true,
    };
    let rec = lust::to_lua_with(&keeps, "main.lst", &files("records")).unwrap();
    let api5 = load(&lua, &rec);
    let fighter_class: Table = api5.get::<Table>("components").unwrap().get("Fighter").unwrap();
    fn entry(f: &Table, depth: usize, out: &mut String) {
        let kind: String = f.get(3).unwrap();
        writeln!(out, "{}{} is {kind}", "  ".repeat(depth), f.get::<String>(2).unwrap()).unwrap();
        if kind == "record" {
            writeln!(out, "{}  (a {})", "  ".repeat(depth), f.get::<String>(5).unwrap()).unwrap();
            for x in f.get::<Table>(4).unwrap().sequence_values::<Table>() {
                entry(&x.unwrap(), depth + 1, out);
            }
        }
    }
    for f in fighter_class.get::<Table>("__schema").unwrap().sequence_values::<Table>() {
        entry(&f.unwrap(), 0, &mut out);
    }
    let equip: Table = api5.get::<Table>("events").unwrap().get("Equip").unwrap();
    for f in equip.sequence_values::<Table>() {
        entry(&f.unwrap(), 0, &mut out);
    }
    let fighter: Table = api5.get::<Function>("new").unwrap().call(("Fighter", "0000000000007")).unwrap();
    let update: Function = fighter.get("update").unwrap();
    update.call::<()>((fighter.clone(), 0.5)).unwrap();
    out.push_str(&log.borrow_mut().split_off(0));
    // The engine delivers what was sent: the same table, which Lust copies as it arrives.
    let handle: Function = fighter.get("handle").unwrap();
    let mut i = 1;
    while let Ok(Value::Table(e)) = queue.get::<Value>(i) {
        let values: Table = e.get("values").unwrap();
        let sent: Table = values.get(1).unwrap();
        let main: Table = sent.get("main").unwrap();
        writeln!(
            out,
            "{} carries {} with ammo {}",
            e.get::<String>("name").unwrap(),
            sent.get::<String>("name").unwrap(),
            main.get::<i64>("ammo").unwrap()
        )
        .unwrap();
        handle
            .call::<bool>((fighter.clone(), e.get::<String>("name").unwrap(), values.clone()))
            .unwrap();
        handle
            .call::<bool>((fighter.clone(), e.get::<String>("name").unwrap(), values))
            .unwrap();
        writeln!(
            out,
            "after handling, the engine's copy is still {}",
            sent.get::<String>("name").unwrap()
        )
        .unwrap();
        queue.set(i, Value::Nil).unwrap();
        i += 1;
    }
    out.push_str(&log.borrow_mut().split_off(0));
    // An event from the engine: a table it made, made Lust's own as it arrives.
    let loadout: Table = fighter.get("loadout").unwrap();
    let given: Table = lua
        .load("return { name = \"medic\", main = { gun = \"Glock\", ammo = 3, aim = vector.create(1, 0, 0) }, buddy = \"0000000000009\" }")
        .eval()
        .unwrap();
    let event_values = lua.create_table().unwrap();
    event_values.set(1, given.clone()).unwrap();
    event_values.set(2, 0).unwrap();
    event_values.set("n", 2).unwrap();
    handle.call::<bool>((fighter.clone(), "Equip", event_values)).unwrap();
    out.push_str(&log.borrow_mut().split_off(0));
    writeln!(
        out,
        "loadout kept: {}, the engine's still {}",
        loadout.get::<String>("name").unwrap(),
        given.get::<String>("name").unwrap()
    )
    .unwrap();
    // One with a value missing is refused, as Rust's handle refuses it.
    given.set("main", Value::Nil).unwrap();
    let short = lua.create_table().unwrap();
    short.set(1, given).unwrap();
    short.set(2, 0).unwrap();
    short.set("n", 2).unwrap();
    match handle.call::<bool>((fighter.clone(), "Equip", short)) {
        Ok(_) => writeln!(out, "short record taken?").unwrap(),
        Err(e) => writeln!(out, "short record: {}", rec.locate(&e.to_string()).message).unwrap(),
    }
    // An object's record, saved by the engine: a copy, so changing the save changes nothing.
    // Loading gives a copy back, and one of the wrong shape is left as it is.
    let save5: Function = api5.get("save").unwrap();
    let load5: Function = api5.get("load").unwrap();
    let armory: Table = api5.get::<Table>("objects").unwrap().get("Armory").unwrap();
    let saved: Table = save5.call("Armory").unwrap();
    let spare: Table = saved.get("spare").unwrap();
    spare.set("ammo", 99).unwrap();
    let own: Table = armory.get("spare").unwrap();
    writeln!(
        out,
        "saved spare: {} {}, armory still {}",
        spare.get::<String>("gun").unwrap(),
        spare.get::<i64>("ammo").unwrap(),
        own.get::<i64>("ammo").unwrap()
    )
    .unwrap();
    load5.call::<()>(("Armory", saved.clone())).unwrap();
    spare.set("ammo", 12).unwrap();
    let own: Table = armory.get("spare").unwrap();
    writeln!(out, "loaded spare: ammo {}", own.get::<i64>("ammo").unwrap()).unwrap();
    let wrong: Table = lua
        .load("return { spare = { gun = \"Bazooka\", ammo = 1, aim = vector.create(0, 0, 0) } }")
        .eval()
        .unwrap();
    load5.call::<()>(("Armory", wrong)).unwrap();
    let own: Table = armory.get("spare").unwrap();
    writeln!(
        out,
        "wrong spare left: {} {}",
        own.get::<String>("gun").unwrap(),
        own.get::<i64>("ammo").unwrap()
    )
    .unwrap();
    // Swapped to the same code, the record is kept.
    let rec2 = lust::to_lua_with(&keeps, "main.lst", &files("records")).unwrap();
    let api6 = load(&lua, &rec2);
    let (now, notes): (Value, Table) = api6.get::<Function>("swap").unwrap().call(fighter.clone()).unwrap();
    writeln!(
        out,
        "swap: {}, {} notes",
        if now.is_nil() { "gone" } else { "kept" },
        notes.raw_len()
    )
    .unwrap();
    let kept: Table = fighter.get("loadout").unwrap();
    let main: Table = kept.get("main").unwrap();
    writeln!(out, "after swap: ammo {}", main.get::<i64>("ammo").unwrap()).unwrap();

    // 12. Events in options: handle runs the entry for the component's state when it's called,
    // and an option no entry covers ignores the event.
    let st = lust::to_lua("main.lst", &files("states")).unwrap();
    let api7 = load(&lua, &st);
    let shield: Table = api7.get::<Function>("new").unwrap().call(("Shield", "0000000000004")).unwrap();
    for (k, damage) in [1.0, 3.0, 2.0, 5.0].iter().enumerate() {
        if k == 2 {
            shield.set("state", "Down.Cracked").unwrap();
        }
        let values = lua.create_table().unwrap();
        values.set(1, *damage).unwrap();
        values.set("n", 1).unwrap();
        let heard: bool = shield
            .get::<Function>("handle")
            .unwrap()
            .call((shield.clone(), "Hit", values))
            .unwrap();
        let said = log.borrow_mut().split_off(0).trim_end().replace('\n', "; ");
        writeln!(
            out,
            "hit {damage}: {}; handled {heard}, now {} with hp {}",
            if said.is_empty() { "nothing ran" } else { &said },
            shield.get::<String>("state").unwrap(),
            shield.get::<f64>("hp").unwrap()
        )
        .unwrap();
    }

    // 13. Methods in options: update runs the body for the component's state each tick, and
    // mend, only for Broken, stops with WrongState in any other option.
    let update: Function = shield.get("update").unwrap();
    let mend: Function = shield.get("mend").unwrap();
    for _ in 0..2 {
        update.call::<()>((shield.clone(), 1.0 / 60.0)).unwrap();
        let ran = mend.call::<()>(shield.clone()).map_err(|e| e.to_string());
        let said = log.borrow_mut().split_off(0).trim_end().replace('\n', "; ");
        let err = ran.err().unwrap_or_default();
        let err = err.lines().next().unwrap_or("").trim_start_matches("runtime error: ").to_string();
        writeln!(
            out,
            "update and mend: {}{}; now {}",
            if said.is_empty() { "nothing ran" } else { &said },
            if err.is_empty() { String::new() } else { format!("; mend: {err}") },
            shield.get::<String>("state").unwrap()
        )
        .unwrap();
    }
    shield.set("state", "Down.Cracked").unwrap();
    update.call::<()>((shield.clone(), 1.0 / 60.0)).unwrap();
    let said = log.borrow_mut().split_off(0).trim_end().replace('\n', "; ");
    writeln!(out, "update when cracked: {said}").unwrap();

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
