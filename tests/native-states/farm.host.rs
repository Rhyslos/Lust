// A stand-in for an engine whose native methods take and give back states' options.
use std::cell::RefCell;

thread_local! {
    static NOW: RefCell<String> = RefCell::new("Idle".into());
}

struct Host;

impl farm::Host for Host {
    fn sprite_play(&self, _this: &farm::Sprite, clip: &'static str) {
        println!("play {clip}");
        NOW.with(|n| *n.borrow_mut() = clip.to_string());
    }
    fn sprite_playing(&self, _this: &farm::Sprite) -> String {
        NOW.with(|n| n.borrow().clone())
    }
    fn sprite_clips(&self, _this: &farm::Sprite) -> Vec<String> {
        vec!["Idle".into(), "Farmer.Hoe".into()]
    }
    fn first(&self) -> String {
        "Farmer".into()
    }
}

/// An engine that gives back an option the state doesn't have.
struct Wrong;

impl farm::Host for Wrong {
    fn sprite_play(&self, _this: &farm::Sprite, _clip: &'static str) {}
    fn sprite_playing(&self, _this: &farm::Sprite) -> String {
        "Farmer.Run".into()
    }
    fn sprite_clips(&self, _this: &farm::Sprite) -> Vec<String> {
        Vec::new()
    }
    fn first(&self) -> String {
        "Idle".into()
    }
}

fn main() {
    farm::set_host(Host);
    farm::start().unwrap();
    let sprite = farm::Sprite::new().unwrap();
    farm::show(Some(sprite.clone())).unwrap();
    farm::set_host(Wrong);
    match farm::show(Some(sprite)) {
        Ok(()) => println!("never"),
        Err(e) => println!("{e}"),
    }
}
