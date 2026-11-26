// An engine calling Lust code that names started work and cancels it: runs aren't there yet, so
// start just calls the method.
fn main() {
    work::start().unwrap();
    println!("{}", work::try_it().unwrap());
}
