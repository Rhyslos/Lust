// A stand-in for an engine giving every entity methods: where it is, moving it, its name.
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
    fn entity_name(&self, this: Option<u64>) -> String {
        format!("entity {}", this.unwrap_or(0))
    }
    fn player(&self) -> Option<u64> {
        Some(7)
    }
}

fn main() {
    PLACES.with(|p| p.borrow_mut().insert(7, game::Vec3 { x: 1.0, y: 2.0, z: 0.0 }));
    game::set_host(Host);
    game::start().unwrap();
    game::show().unwrap();
    let walker = game::Walker::new().unwrap();
    walker.set_me(Some(7)).unwrap();
    for _ in 0..30 {
        walker.update(0.5).unwrap();
    }
    game::show().unwrap();
}
