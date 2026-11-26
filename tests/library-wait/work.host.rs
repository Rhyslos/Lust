// An engine calling Lust code that names started work and cancels it: work that never waits is
// done by the line after its start.
fn main() {
    work::start().unwrap();
    println!("{}", work::try_it().unwrap());
}
