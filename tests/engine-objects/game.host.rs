// A stand-in for an engine that ticks a game's objects, then its components, and saves them.
fn show() {
    let clock = game::Clock::get().to_data();
    println!("day {:?}, hour {:?}, {:?}, coins {:?}", clock["day"], clock["hour"], clock["season"], game::Money::get().to_data()["coins"]);
}

fn main() {
    game::start().unwrap();
    for s in game::objects() {
        let names: Vec<&str> = s.fields.iter().map(|f| f.name.as_str()).collect();
        println!("{}: {names:?}", s.name);
    }
    let farmer = game::Farmer::new().unwrap();
    farmer.set_me(Some(1)).unwrap();
    for _ in 0..3 {
        game::tick_objects(1.0).unwrap();
        farmer.update(1.0).unwrap();
    }
    show();
    // A save, then play on, then load it.
    let saved = (game::Clock::get().to_data(), game::Money::get().to_data(), game::Camp::get().to_data());
    game::tick_objects(5.0).unwrap();
    show();
    let camp = game::Camp::get().to_data();
    println!("camp changed: {}", camp != saved.2);
    game::Clock::get().set_data(&saved.0);
    game::Money::get().set_data(&saved.1);
    game::Camp::get().set_data(&saved.2);
    show();
    println!("camp loaded exactly: {}", game::Camp::get().to_data() == saved.2);
    for name in ["fire", "cell", "facing", "glow"] {
        println!("{name} {:?}", saved.2[name]);
    }
}
