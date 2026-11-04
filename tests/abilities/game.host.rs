// A stand-in for an engine whose entities have abilities, ticking one component.
use std::cell::RefCell;
use std::collections::BTreeMap;

thread_local! {
    static PLACES: RefCell<BTreeMap<u64, game::Vec3>> = RefCell::new(BTreeMap::new());
}

struct Host;

impl game::Host for Host {
    fn entity_position(&self, this: Option<u64>) -> game::Vec3 {
        PLACES.with(|p| p.borrow().get(&this.unwrap_or(0)).copied().unwrap_or_default())
    }
    fn entity_move_to(&self, this: Option<u64>, place: game::Vec3) {
        PLACES.with(|p| p.borrow_mut().insert(this.unwrap_or(0), place));
    }
    fn entity_jump(&self, this: Option<u64>) {
        println!("the engine makes {} jump", this.unwrap_or(0));
    }
    fn entity_velocity(&self, _this: Option<u64>) -> f64 {
        2.5
    }
    fn log(&self, words: String) {
        println!("{words}");
    }
}

fn main() {
    game::set_host(Host);
    game::start().unwrap();
    let hopper = game::Hopper::new().unwrap();
    hopper.set_me(Some(9)).unwrap();
    hopper.update(0.5).unwrap();
}
