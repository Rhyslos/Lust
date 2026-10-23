// A stand-in for Solis: entities with components written in Lust, ticked in a fixed order.
use std::cell::RefCell;
use std::collections::BTreeMap;

struct World {
    places: RefCell<BTreeMap<u64, game::Vec3>>,
}

thread_local! {
    static WORLD: World = World { places: RefCell::new(BTreeMap::new()) };
}

struct Host;

impl game::Host for Host {
    fn position_of(&self, e: Option<u64>) -> game::Vec3 {
        WORLD.with(|w| w.places.borrow().get(&e.unwrap_or(0)).copied().unwrap_or_default())
    }
    fn move_to(&self, e: Option<u64>, to: game::Vec3) {
        WORLD.with(|w| w.places.borrow_mut().insert(e.unwrap_or(0), to));
    }
    fn nearest(&self, from: Option<u64>) -> Option<u64> {
        WORLD.with(|w| w.places.borrow().keys().copied().find(|k| Some(*k) != from))
    }
    fn log(&self, words: String) {
        println!("{words}");
    }
}

fn main() {
    game::set_host(Host);
    game::start().unwrap();
    for s in game::components() {
        println!("{}:", s.name);
        for f in &s.fields {
            println!("  {} {:?} = {:?}", f.name, f.kind, f.default);
        }
    }
    WORLD.with(|w| {
        w.places.borrow_mut().insert(1, game::Vec3 { x: 0.0, y: 0.0, z: 0.0 });
        w.places.borrow_mut().insert(2, game::Vec3 { x: 5.0, y: 0.0, z: 0.0 });
    });

    // A gun on entity 1, loaded from an area file's values; one field is the wrong kind.
    let mut data = game::Data::new();
    data.insert("ammo".into(), game::Value::Int(2));
    data.insert("mode".into(), game::Value::Text("Burst".into()));
    data.insert("reload".into(), game::Value::Text("fast".into()));
    let gun = game::Gun::from_data(&data).unwrap();
    gun.set_me(Some(1)).unwrap();
    let spin = game::Spinner::new().unwrap();
    spin.set_me(Some(2)).unwrap();

    // Ticks: entities in ID order, components in name order.
    for tick in 0..40 {
        gun.update(1.0 / 60.0).unwrap();
        spin.update(1.0 / 60.0).unwrap();
        if tick % 20 == 19 {
            println!("tick {tick}: {:?}", gun.to_data());
        }
    }
    println!("{:?}", spin.to_data());
    println!("entity 2 at {:?}", WORLD.with(|w| w.places.borrow()[&2]));

    // An edit in the editor changes one field, and keeps the rest.
    let mut edit = game::Data::new();
    edit.insert("label".into(), game::Value::Text("blaster".into()));
    gun.set_data(&edit);
    println!("{:?}", gun.to_data());
}
