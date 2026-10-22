// A stand-in for Solis: an engine in Rust that runs game logic written in Lust.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// The engine's own data: where each entity is, and the clock.
#[derive(Default)]
struct Engine {
    positions: RefCell<HashMap<i64, f64>>,
    clock: Cell<f64>,
}

struct Host(Rc<Engine>);

// The bodies of the Lust code's native methods.
impl game::Host for Host {
    fn entity_x(&self, this: &game::Entity) -> f64 {
        let id = this.id().unwrap();
        *self.0.positions.borrow().get(&id).unwrap_or(&0.0)
    }
    fn entity_move_by(&self, this: &game::Entity, dx: f64) {
        let id = this.id().unwrap();
        *self.0.positions.borrow_mut().entry(id).or_insert(0.0) += dx;
    }
    fn entity_say(&self, this: &game::Entity, words: String) {
        println!("entity {} says: {words}", this.id().unwrap());
    }
    fn now(&self) -> f64 {
        self.0.clock.get()
    }
}

fn main() {
    let engine = Rc::new(Engine::default());
    game::set_host(Host(engine.clone()));
    game::start().unwrap();

    let player = game::Entity::new().unwrap();
    player.set_id(7).unwrap();
    let gun = game::Weapon::new().unwrap();
    gun.set_name("Blaster".into()).unwrap();

    // A few frames: the clock moves on, and the player tries to fire every 0.2 seconds.
    for frame in 0..6 {
        engine.clock.set(frame as f64 * 0.2);
        let fired = gun.fire(Some(player.clone())).unwrap();
        println!("t={:.1} fired={fired} ammo={}", engine.clock.get(), gun.ammo().unwrap());
    }
    println!("player x = {}", player.x().unwrap());
    println!("damage = {}, critical = {}", game::damage(10, false).unwrap(), game::damage(10, true).unwrap());

    let score = game::Score::get();
    score.set_shots(3).unwrap();
    println!("shots = {}", score.shots().unwrap());

    // A Lust mistake comes back as text, with the Lust file and line.
    gun.set_ammo(1).unwrap();
    let broken = game::Weapon::new().unwrap();
    match broken.fire(None) {
        Ok(_) => println!("no error?"),
        Err(e) => println!("Lust said: {e}"),
    }
}
