// A stand-in for Solis delivering events: one queue a tick, entities' components in name order,
// and events sent while delivering join the end of the same tick's queue.
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
    for e in game::events() {
        println!("event {} {:?}", e.name, e.values);
    }
    println!("Armor handles {:?}, Health {:?}, Gun {:?}", game::Armor::HANDLES, game::Health::HANDLES, game::Gun::HANDLES);

    // Entity 1 has a gun aimed at entity 2, which has armor and health.
    let gun = game::Gun::new().unwrap();
    gun.set_me(Some(1)).unwrap();
    gun.set_target(Some(2)).unwrap();
    let armor = game::Armor::new().unwrap();
    armor.set_me(Some(2)).unwrap();
    let health = game::Health::new().unwrap();
    health.set_me(Some(2)).unwrap();

    for tick in 0..3 {
        println!("tick {tick}");
        gun.update(0.5).unwrap();
        // Deliver: each event to the entity's components in name order (Armor, then Health).
        while let Some((to, name, values)) = QUEUE.with(|q| q.borrow_mut().pop_front()) {
            if to == Some(2) {
                let a = armor.handle(&name, &values).unwrap();
                let h = health.handle(&name, &values).unwrap();
                if !a && !h {
                    println!("nobody on 2 handles {name}");
                }
            }
        }
    }

    // The engine hands values in; the wrong count or kind is an error, not a crash.
    let push = vec![
        game::Value::List(vec![game::Value::Float(1.0), game::Value::Int(2), game::Value::Float(0.5)]),
        game::Value::List(vec![game::Value::Float(0.0), game::Value::Float(0.0), game::Value::Float(0.70710677), game::Value::Float(0.70710677)]),
    ];
    println!("{:?}", gun.handle("Pushed", &push));
    println!("{:?}", gun.handle("Pushed", &push[..1]));
    println!("{:?}", gun.handle("Pushed", &[game::Value::Int(1), push[1].clone()]));
    println!("{:?}", gun.handle("Hit", &[]));
    println!("{:?}", health.handle("Hit", &[game::Value::Entity(None), game::Value::Int(1), game::Value::Text("Cold".into())]));
    println!("{:?}", health.to_data());

    // A failure in Lust code says where it happened: file, line and column, and why.
    println!("{:?}", gun.share(4));
    match gun.share(0) {
        Ok(_) => println!("no failure?"),
        Err(e) => println!("{e}\n{:?} {:?} {:?}", e.file, e.line, e.column),
    }
    println!("Lust {}", game::LUST_VERSION.split('-').next().unwrap());
}
