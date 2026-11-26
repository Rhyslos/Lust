// An engine calling Lust code with a check in a wait { } straight from the engine's call: it can't
// wait there, but goes on when it needn't, and the when for the state it's in runs.
fn main() {
    boss::start().unwrap();
    match boss::chase() {
        Ok(v) => println!("chase: {v}"),
        Err(e) => println!("Lust said: {e} ({})", e.problem),
    }
    boss::scare().unwrap();
    println!("chase: {}", boss::chase().unwrap());
}
