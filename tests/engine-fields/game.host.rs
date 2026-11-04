// A stand-in for an engine that fills in a component's field with something its editor made.
struct Host;

impl game::Host for Host {
    fn farmer_animator_play(&self, this: &game::FarmerAnimator, clip: String) {
        println!("animator {} plays {clip}", this.id().unwrap());
    }
}

fn main() {
    game::set_host(Host);
    game::start().unwrap();
    println!("filled: {:?}", game::Farmer::FILLED);
    let schema = game::Farmer::schema();
    let names: Vec<&str> = schema.fields.iter().map(|f| f.name.as_str()).collect();
    println!("kept and shown: {names:?}");
    let farmer = game::Farmer::new().unwrap();
    farmer.set_me(Some(4)).unwrap();
    // Before the engine fills it in.
    match farmer.update(0.1) {
        Ok(()) => println!("never"),
        Err(e) => println!("{e}"),
    }
    let anim = game::FarmerAnimator::new().unwrap();
    anim.set_id(12).unwrap();
    farmer.set_anim(Some(anim)).unwrap();
    farmer.update(0.1).unwrap();
}
