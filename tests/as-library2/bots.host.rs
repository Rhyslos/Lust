// A stand-in for Solis that hands Lust vectors, rotations and lists, and takes them back.
use std::cell::RefCell;
use std::collections::BTreeMap;

struct Host {
    places: RefCell<BTreeMap<i64, bots::Vec3>>,
}

impl bots::Host for Host {
    fn thing_position(&self, this: &bots::Thing) -> bots::Vec3 {
        self.places.borrow()[&this.id().unwrap()]
    }
    fn thing_move_to(&self, this: &bots::Thing, to: bots::Vec3) {
        self.places.borrow_mut().insert(this.id().unwrap(), to);
    }
    fn thing_facing(&self, _this: &bots::Thing) -> bots::Rotation {
        // A quarter turn around z, as Solis would hand it over.
        bots::Rotation { x: 0.0, y: 0.0, z: 0.70710677, w: 0.70710677 }
    }
    fn thing_near(&self, this: &bots::Thing, radius: f64) -> Vec<Option<bots::Thing>> {
        let me = self.places.borrow()[&this.id().unwrap()];
        let mut out = Vec::new();
        for (id, p) in self.places.borrow().iter() {
            let d = ((p.x - me.x).powi(2) + (p.y - me.y).powi(2) + (p.z - me.z).powi(2)).sqrt() as f64;
            if *id != this.id().unwrap() && d <= radius {
                let t = bots::Thing::new().unwrap();
                t.set_id(*id).unwrap();
                out.push(Some(t));
            }
        }
        out
    }
    fn ids(&self) -> Vec<i64> {
        self.places.borrow().keys().copied().collect()
    }
    fn gravity(&self) -> bots::Vec3 {
        bots::Vec3 { x: 0.0, y: 0.0, z: -9.81 }
    }
    fn aim(&self, path: Vec<bots::Vec3>, turn: bots::Rotation) {
        println!("aim along {} points, from {:?} to {:?}, turned {:?}", path.len(), path[0], path[1], turn);
    }
}

fn main() {
    let mut places = BTreeMap::new();
    places.insert(1, bots::Vec3 { x: 0.0, y: 0.0, z: 10.0 });
    places.insert(2, bots::Vec3 { x: 3.0, y: 0.0, z: 10.0 });
    places.insert(3, bots::Vec3 { x: 30.0, y: 0.0, z: 0.0 });
    bots::set_host(Host { places: RefCell::new(places) });
    bots::start().unwrap();

    let me = bots::Thing::new().unwrap();
    me.set_id(1).unwrap();
    let bot = bots::Bot::new().unwrap();
    for _ in 0..3 {
        bot.step(Some(me.clone()), 1.0 / 60.0).unwrap();
    }
    println!("velocity {:?}", bot.velocity().unwrap());
    println!("ahead {:?}", bot.ahead(Some(me.clone())).unwrap());
    println!("crowd {}", bot.crowd(Some(me.clone())).unwrap());
    println!("all {}", bots::count_all().unwrap());
    bot.set_velocity(bots::Vec3 { x: 1.0, y: 2.0, z: 3.0 }).unwrap();
    println!("velocity {:?}", bot.velocity().unwrap());
    bot.plan(Some(me)).unwrap();
}
