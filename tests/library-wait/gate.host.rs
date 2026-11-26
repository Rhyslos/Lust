// An engine calling Lust code that waits { } straight from the engine's call: it can't wait
// there, but goes on when it needn't.
fn main() {
    gate::start().unwrap();
    match gate::through() {
        Ok(v) => println!("through: {v}"),
        Err(e) => println!("Lust said: {e} ({})", e.problem),
    }
    gate::open_up().unwrap();
    println!("through: {}", gate::through().unwrap());
}
