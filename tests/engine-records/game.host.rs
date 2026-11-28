// A stand-in for an engine that keeps records: their kinds in a component's schema and an
// event's, their values both ways as Value::Record, and an object's record in a save.
use std::cell::RefCell;
use std::collections::VecDeque;

thread_local! {
    static QUEUE: RefCell<VecDeque<(Option<u64>, String, Vec<game::Value>)>> = RefCell::new(VecDeque::new());
}

struct Host;

impl game::Host for Host {
    fn log(&self, words: String) {
        println!("{words}");
    }
    fn send_event(&self, to: Option<u64>, event: &str, values: Vec<game::Value>) {
        println!("queued {event} to {to:?}: {values:?}");
        QUEUE.with(|q| q.borrow_mut().push_back((to, event.to_string(), values)));
    }
}

fn main() {
    game::set_host(Host);
    game::start().unwrap();
    for s in game::components() {
        for f in &s.fields {
            println!("{}'s {} {:?}", s.name, f.name, f.kind);
            println!("  starts {:?}", f.default);
        }
    }
    for e in game::events() {
        println!("event {} {:?}", e.name, e.values);
    }
    let fighter = game::Fighter::new().unwrap();
    fighter.set_me(Some(1)).unwrap();
    fighter.update(0.5).unwrap();
    while let Some((_, name, values)) = QUEUE.with(|q| q.borrow_mut().pop_front()) {
        fighter.handle(&name, &values).unwrap();
        fighter.handle(&name, &values).unwrap();
    }
    println!("{:?}", fighter.to_data());

    // A record the engine made: every value must be there and of the right kind.
    let mut kit = game::Data::new();
    kit.insert("gun".into(), game::Value::Text("Glock".into()));
    kit.insert("ammo".into(), game::Value::Int(3));
    kit.insert("aim".into(), game::Value::List(vec![game::Value::Float(1.0), game::Value::Float(0.0)]));
    kit.insert("tint".into(), game::Value::List(vec![game::Value::Float(2.0), game::Value::Float(0.5), game::Value::Float(0.0), game::Value::Float(1.0)]));
    let mut loadout = game::Data::new();
    loadout.insert("name".into(), game::Value::Text("medic".into()));
    loadout.insert("main".into(), game::Value::Record(kit.clone()));
    loadout.insert("buddy".into(), game::Value::Entity(Some(9)));
    fighter.handle("Equip", &[game::Value::Record(loadout.clone()), game::Value::Int(2)]).unwrap();
    let mut short = loadout.clone();
    short.remove("buddy");
    println!("{:?}", fighter.handle("Equip", &[game::Value::Record(short), game::Value::Int(2)]).map_err(|e| e.message));

    // set_data: a whole record, or it's left as it is.
    let mut data = game::Data::new();
    data.insert("loadout".into(), game::Value::Record(loadout));
    fighter.set_data(&data);
    println!("{:?}", fighter.to_data());
    let mut bad = game::Data::new();
    bad.insert("loadout".into(), game::Value::Int(3));
    fighter.set_data(&bad);
    println!("{:?}", fighter.to_data()["loadout"]);

    // An object's record, saved and loaded back.
    let armory = game::Armory::get();
    println!("{:?}", armory.to_data());
    let mut save = game::Data::new();
    kit.insert("ammo".into(), game::Value::Int(11));
    save.insert("spare".into(), game::Value::Record(kit));
    armory.set_data(&save);
    println!("{:?}", armory.to_data());
}
