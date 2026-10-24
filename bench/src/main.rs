//! Ticks N movers for T ticks, delivering their events, in Rust and in Luau (with and without
//! native code), and says how long a tick takes in each. All of them have to end with exactly the
//! same values, so it doubles as a check that Lua while editing and Rust in a release agree.
//! `--batched` ticks from one Lua loop instead of a call per component; `--lua file.lua` runs a
//! timing script in Luau, with the runtime as L and the chunk's table as API.

use mlua::{Function, Lua, Table, Value};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Instant;

const N: usize = 2000;
const TICKS: usize = 600;
const DT: f64 = 1.0 / 60.0;

thread_local! {
    static QUEUE: RefCell<VecDeque<(Option<u64>, Vec<game::Value>)>> = RefCell::new(VecDeque::new());
}

struct Host;
impl game::Host for Host {
    fn send_event(&self, to: Option<u64>, _event: &str, values: Vec<game::Value>) {
        QUEUE.with(|q| q.borrow_mut().push_back((to, values)));
    }
}

/// A mover's values as numbers, and its mood: both ways must give exactly the same.
type Values = (Vec<f64>, String);

fn values_rust(m: &game::Mover) -> Values {
    let d = m.to_data();
    let mut nums = Vec::new();
    for k in ["position", "velocity", "facing"] {
        if let Some(game::Value::List(items)) = d.get(k) {
            for i in items {
                if let game::Value::Float(x) = i {
                    nums.push(*x);
                }
            }
        }
    }
    if let Some(game::Value::Float(s)) = d.get("speed") {
        nums.push(*s);
    }
    if let Some(game::Value::Int(b)) = d.get("bounces") {
        nums.push(*b as f64);
    }
    let mood = match d.get("mood") {
        Some(game::Value::Text(t)) => t.clone(),
        _ => String::new(),
    };
    (nums, mood)
}

fn rust() -> (f64, Vec<Values>) {
    game::set_host(Host);
    game::start().unwrap();
    let movers: Vec<game::Mover> = (0..N)
        .map(|i| {
            let m = game::Mover::new().unwrap();
            m.set_me(Some(i as u64 + 1)).unwrap();
            m.set_other(Some(((i + 1) % N) as u64 + 1)).unwrap();
            m.set_velocity(game::Vec3 {
                x: (i % 7) as f32 * 0.5,
                y: 0.25,
                z: (i % 5) as f32,
            })
            .unwrap();
            m
        })
        .collect();
    let start = Instant::now();
    for _ in 0..TICKS {
        for m in &movers {
            m.update(DT).unwrap();
        }
        while let Some((to, values)) = QUEUE.with(|q| q.borrow_mut().pop_front()) {
            let i = to.unwrap() as usize - 1;
            movers[i].handle("Bumped", &values).unwrap();
        }
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / TICKS as f64;
    (ms, movers.iter().map(values_rust).collect())
}

fn luau(jit: bool) -> (f64, Vec<Values>) {
    let files = vec![(
        "game.lst".to_string(),
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/game.lst")).unwrap(),
    )];
    let chunk = lust::to_lua("game.lst", &files).unwrap();
    let lua = Lua::new();
    lua.enable_jit(jit);
    let queue = lua.create_table().unwrap();
    lua.globals().set("QUEUE", queue.clone()).unwrap();
    let natives = lua.create_table().unwrap();
    let send: Function = lua
        .load("return function(to, name, values) QUEUE[#QUEUE + 1] = { to, values } end")
        .eval()
        .unwrap();
    natives.set("send_event", send).unwrap();
    lua.globals().set("LUST_NATIVES", natives).unwrap();
    let api: Table = lua.load(&chunk.code).set_name(lust::LuaChunk::NAME).eval().unwrap();
    let new: Function = api.get("new").unwrap();
    let id = |i: usize| {
        // Crockford base 32, 13 characters, like Lust's own.
        const A: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        let mut n = i as u64 + 1;
        let mut s = [b'0'; 13];
        for k in (0..13).rev() {
            s[k] = A[(n & 31) as usize];
            n >>= 5;
        }
        String::from_utf8(s.to_vec()).unwrap()
    };
    let mut movers = Vec::new();
    let mut updates = Vec::new();
    let mut handles = Vec::new();
    for i in 0..N {
        let m: Table = new.call(("Mover", id(i))).unwrap();
        m.set("other", id((i + 1) % N)).unwrap();
        // In Luau, a vec3 is a Luau vector.
        let v = mlua::Vector::new((i % 7) as f32 * 0.5, 0.25, (i % 5) as f32);
        m.set("velocity", v).unwrap();
        updates.push(m.get::<Function>("update").unwrap());
        handles.push(m.get::<Function>("handle").unwrap());
        movers.push(m);
    }
    let index: std::collections::HashMap<String, usize> = (0..N).map(|i| (id(i), i)).collect();
    let list = lua.create_sequence_from(movers.iter().cloned()).unwrap();
    let tick: Function = lua
        .load("return function(list, dt) for i = 1, #list do local c = list[i]; c:update(dt) end end")
        .eval()
        .unwrap();
    let batched = std::env::args().any(|a| a == "--batched");
    let start = Instant::now();
    for _ in 0..TICKS {
        if batched {
            tick.call::<()>((list.clone(), DT)).unwrap();
        } else {
            for (m, u) in movers.iter().zip(&updates) {
                u.call::<()>((m.clone(), DT)).unwrap();
            }
        }
        let mut k = 1;
        while let Ok(Value::Table(e)) = queue.get::<Value>(k) {
            let to: String = e.get(1).unwrap();
            let i = index[&to];
            handles[i]
                .call::<bool>((movers[i].clone(), "Bumped", e.get::<Table>(2).unwrap()))
                .unwrap();
            queue.set(k, Value::Nil).unwrap();
            k += 1;
        }
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / TICKS as f64;
    let values: Function = lua
        .load(
            r#"return function(m)
                local p, v, f = m.position, m.velocity, m.facing
                return { p.x, p.y, p.z, v.x, v.y, v.z, f.x, f.y, f.z, f.w, m.speed, m.bounces }, m.mood
            end"#,
        )
        .eval()
        .unwrap();
    let out = movers
        .iter()
        .map(|m| {
            let (t, mood): (Table, String) = values.call(m.clone()).unwrap();
            (t.sequence_values::<f64>().map(|x| x.unwrap()).collect(), mood)
        })
        .collect();
    (ms, out)
}

fn main() {
    // --load file.lst: compiles it and loads it in Luau, with native code and without.
    if let Some(i) = std::env::args().position(|a| a == "--load") {
        let path = std::env::args().nth(i + 1).unwrap();
        let files = vec![("main.lst".to_string(), std::fs::read_to_string(&path).unwrap())];
        match lust::to_lua("main.lst", &files) {
            Err(p) => println!("mistake: {}", p[0]),
            Ok(chunk) => {
                for jit in [true, false] {
                    let lua = Lua::new();
                    lua.enable_jit(jit);
                    match lua.load(&chunk.code).set_name(lust::LuaChunk::NAME).exec() {
                        Ok(()) => println!("loaded (native code: {jit})"),
                        Err(e) => println!(
                            "Luau refused it (native code: {jit}): {}",
                            e.to_string().lines().next().unwrap_or("")
                        ),
                    }
                }
            }
        }
        return;
    }
    // --lua file: runs a Lua script in Luau after the game's chunk, with the chunk's table as API.
    if let Some(i) = std::env::args().position(|a| a == "--lua") {
        let script = std::fs::read_to_string(std::env::args().nth(i + 1).unwrap()).unwrap();
        let files = vec![(
            "game.lst".to_string(),
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/game.lst")).unwrap(),
        )];
        let chunk = lust::to_lua("game.lst", &files).unwrap();
        let lua = Lua::new();
        let code = chunk.code.replace("return _L.engine(", "return _L, _L.engine(");
        let (rt, api): (Table, Table) = lua.load(&code).set_name(lust::LuaChunk::NAME).eval().unwrap();
        lua.globals().set("API", api).unwrap();
        lua.globals().set("L", rt).unwrap();
        lua.globals().set("CHUNK", chunk.code.clone()).unwrap();
        lua.load(&script).exec().unwrap();
        return;
    }
    let (rms, rsum) = rust();
    let (lms, lsum) = luau(true);
    let (ims, isum) = luau(false);
    assert!(lsum == isum, "native code and the interpreter disagree");
    println!("{N} movers, {TICKS} ticks");
    println!("Rust: {rms:.3} ms a tick");
    println!("Luau, native code: {lms:.3} ms a tick");
    println!("Luau, interpreted: {ims:.3} ms a tick");
    // Exactly the same numbers, to the last bit, in every mover.
    let mut different = 0;
    for (i, (r, l)) in rsum.iter().zip(&lsum).enumerate() {
        let same = r.1 == l.1 && r.0.len() == l.0.len() && r.0.iter().zip(&l.0).all(|(a, b)| a.to_bits() == b.to_bits());
        if !same {
            if different < 3 {
                println!("mover {i} differs:\n  Rust {r:?}\n  Luau {l:?}");
            }
            different += 1;
        }
    }
    if different > 0 || rsum.len() != lsum.len() {
        println!("{different} movers differ between Rust and Luau.");
        std::process::exit(1);
    }
    println!("Rust and Luau end with exactly the same values in all {N} movers.");
}
