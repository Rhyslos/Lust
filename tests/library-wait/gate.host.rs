// An engine calling Lust code that waits { }: it can't wait yet, but goes on when it needn't.
fn main() {
    gate::start().unwrap();
    match gate::through() {
        Ok(v) => println!("through: {v}"),
        Err(e) => println!("Lust said: {e} ({})", e.problem),
    }
    gate::open_up().unwrap();
    println!("through: {}", gate::through().unwrap());
}
