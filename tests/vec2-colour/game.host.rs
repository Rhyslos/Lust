// A stand-in 2D engine: vec2s and colours going into and out of Lust, as fields, natives and events.
use std::cell::RefCell;
use std::collections::VecDeque;

thread_local! {
    static SPOT: RefCell<game::Vec2> = RefCell::new(game::Vec2 { x: 1.0, y: 1.0 });
    static QUEUE: RefCell<VecDeque<(Option<u64>, String, Vec<game::Value>)>> = RefCell::new(VecDeque::new());
}

struct Host;

impl game::Host for Host {
    fn spot_of(&self, _e: Option<u64>) -> game::Vec2 {
        SPOT.with(|s| *s.borrow())
    }
    fn slide(&self, e: Option<u64>, to: game::Vec2) {
        println!("{e:?} slides to {to:?}");
        SPOT.with(|s| *s.borrow_mut() = to);
    }
    // A part above 1 is kept at 1 as it comes into Lust.
    fn tint_of(&self, _e: Option<u64>) -> game::Colour {
        game::Colour { r: 1.5, g: 0.5, b: 0.0, a: 1.0 }
    }
    fn paint(&self, e: Option<u64>, c: game::Colour) {
        println!("{e:?} painted {:?}", c.to_array());
    }
    fn path(&self) -> Vec<game::Vec2> {
        vec![game::Vec2 { x: 0.0, y: 0.5 }, game::Vec2 { x: -1.0, y: 2.0 }]
    }
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
        println!("{}:", s.name);
        for f in &s.fields {
            println!("  {} {:?} = {:?}", f.name, f.kind, f.default);
        }
    }
    for e in game::events() {
        println!("event {} {:?}", e.name, e.values);
    }
    let sprite = game::Sprite::new().unwrap();
    sprite.set_me(Some(3)).unwrap();
    sprite.update(0.5).unwrap();
    while let Some((_, name, values)) = QUEUE.with(|q| q.borrow_mut().pop_front()) {
        println!("handled: {:?}", sprite.handle(&name, &values));
    }

    // The editor sets fields: a colour part above 1 is kept at 1, and the wrong count is left out.
    let mut data = game::Data::new();
    data.insert("speed".into(), game::Value::List(vec![game::Value::Float(0.25), game::Value::Int(4)]));
    data.insert(
        "tint".into(),
        game::Value::List(vec![game::Value::Float(2.0), game::Value::Float(0.5), game::Value::Float(-1.0), game::Value::Float(0.5)]),
    );
    data.insert("shadow".into(), game::Value::List(vec![game::Value::Float(0.5)]));
    sprite.set_data(&data);
    println!("{:?}", sprite.to_data());

    // An event from the engine with the wrong kind is an error, not a crash.
    println!("{:?}", sprite.handle("Flash", &[game::Value::Int(1), game::Value::List(vec![])]));
    println!("{:?}", game::Colour::from([0.5, 2.0, -3.0, 1.0]));
}
