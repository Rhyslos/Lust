// A stand-in for an engine ticking a game whose code waits: the same ticks as engine-test's
// section 10, so the Rust library and Luau print the same.
struct Host;

impl runs::Host for Host {
    fn log(&self, words: String) {
        println!("  {words}");
    }
}

fn main() {
    runs::set_host(Host);
    runs::start().unwrap();
    let gate = runs::Gate::new().unwrap();
    gate.set_me(Some(1)).unwrap();
    let guard = runs::Guard::new().unwrap();
    guard.set_me(Some(2)).unwrap();
    // The engine's time: a paused game (dt 0) and a slowed one move waits less, or not at all.
    let dts = [0.125, 0.125, 0.125, 0.125, 0.0, 0.0, 0.125, 0.0625, 0.0625, 0.125, 0.125, 0.5, 0.5, 0.5, 0.5];
    let mut guarding = true;
    for (k, dt) in dts.iter().enumerate() {
        println!("tick {k} (dt {dt})");
        runs::tick_objects(*dt).unwrap();
        runs::tick_tweens(*dt).unwrap();
        gate.update(*dt).unwrap();
        if guarding {
            guard.update(*dt).unwrap();
        }
        if k == 1 {
            gate.handle("Knock", &[runs::Value::Int(3)]).unwrap();
            guard.handle("Knock", &[runs::Value::Int(3)]).unwrap();
        }
        if k == 3 {
            // The guard's entity is destroyed: its patrol and its waiting handler stop.
            guard.removed().unwrap();
            guarding = false;
        }
        if k == 6 {
            runs::pull_lever().unwrap();
        }
    }
    println!("gate opened {:?}", gate.to_data()["opened"]);
    let sleepy = runs::Sleepy::new().unwrap();
    match sleepy.update(0.125) {
        Ok(()) => println!("Sleepy waited?"),
        Err(e) => println!("Lust said: {e}"),
    }
}
