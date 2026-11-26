// Runs without stacks of their own: for a program that never waits while another run could go
// on, a library for an engine (where runs come later), and lust itself. A run just runs, on the
// stack of the code that started it, and is done when it returns.

pub const OWN_STACKS: bool = false;
pub const DEPTH_LIMIT: u32 = 100_000;

pub struct Run;

pub fn in_run() -> bool {
    false
}

pub fn spawn(f: Box<dyn FnOnce()>) -> Option<(Run, f64)> {
    f();
    None
}

pub fn resume(_: Run) -> Option<(Run, f64)> {
    None
}

pub fn pause(_: f64) {}
