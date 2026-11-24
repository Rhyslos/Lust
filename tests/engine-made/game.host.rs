// A stand-in for an engine that makes things when game code says new.
struct Host;

impl game::Host for Host {
    fn fighter_made(&self, this: &game::Fighter) -> Option<u64> {
        let at = this.position().unwrap();
        println!("a fighter at {}, {} going {}", at.x, at.y, this.speed().unwrap());
        Some(42)
    }
    fn horn_made(&self, this: &game::Horn) {
        println!("a horn at {}", this.at().unwrap().x);
    }
}

fn main() {
    game::set_host(Host);
    game::start().unwrap();
    game::show().unwrap();
    let gate = game::Gate::new().unwrap();
    gate.set_me(Some(4)).unwrap();
    gate.update(0.1).unwrap();
    println!("the gate's guard: {:?}", gate.guard().unwrap());
}
