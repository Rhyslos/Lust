// A stand-in for an engine that moves a game's tweens on once a tick, after the objects and
// before the components.
struct Host;

impl game::Host for Host {
    fn log(&self, words: String) {
        println!("  {words}");
    }
}

fn main() {
    game::set_host(Host);
    game::start().unwrap();
    // Two doors: the first is shut while it's opening, the second opens all the way.
    let door = game::Door::new().unwrap();
    door.set_me(Some(1)).unwrap();
    let other = game::Door::new().unwrap();
    other.set_me(Some(2)).unwrap();
    for tick in 0..8 {
        game::tick_objects(0.1).unwrap();
        game::tick_tweens(0.1).unwrap();
        door.update(0.1).unwrap();
        other.update(0.1).unwrap();
        if tick == 2 {
            door.handle("Shut", &[]).unwrap();
        }
        let (d, o) = (door.to_data(), other.to_data());
        let s = game::Sky::get().to_data();
        println!("tick {tick}: open {:?} and {:?}, opened {:?} and {:?}", d["open"], o["open"], d["opened"], o["opened"]);
        println!("  tint {:?}", s["tint"]);
    }
    // A tween that never started does nothing, and a tick of 0 moves nothing.
    game::tick_tweens(0.0).unwrap();
}
