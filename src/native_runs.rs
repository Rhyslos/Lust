// Runs on stacks of their own, for built programs (native_rt.rs keeps the queue and the clock).
// Each run but the program's own code is a coroutine (the corosensei library): it has its own
// stack, so a wait deep inside its calls pauses only it, on the same thread, as Lua's coroutines
// do for lust run. A run's stack is only reserved: the computer gives it memory as it's used, so
// thousands of runs waiting cost little, and finished runs' stacks are used again.
//
// This is the one place built programs use unsafe code: a wait has to find the run it's in, and
// the library only hands that to the run's first function.

use corosensei::stack::DefaultStack;
use corosensei::{Coroutine, CoroutineResult, Yielder};
use std::cell::{Cell, RefCell};

/// Runs have stacks of their own.
pub const OWN_STACKS: bool = true;
/// How deep calls may go inside a run, on its 128 MB stack (the program's own code has 512 MB).
pub const DEPTH_LIMIT: u32 = 20_000;
const STACK: usize = 128 * 1024 * 1024;
/// Finished runs' stacks kept for new runs.
const SPARE: usize = 32;

type Co = Coroutine<(), f64, (), DefaultStack>;

/// A run that's waiting.
pub struct Run(Co);

thread_local! {
    /// The Yielder of the run going now, as an address; 0 for the program's own code.
    static NOW: Cell<usize> = const { Cell::new(0) };
    static STACKS: RefCell<Vec<DefaultStack>> = const { RefCell::new(Vec::new()) };
}

/// Whether a run (not the program's own code) is going now.
pub fn in_run() -> bool {
    NOW.with(|n| n.get()) != 0
}

/// Starts f as a run, straight away: Some with when it's due if it waits, None once it's done.
/// A failure in it goes on to the caller.
pub fn spawn(f: Box<dyn FnOnce()>) -> Option<(Run, f64)> {
    let stack = match STACKS.with(|s| s.borrow_mut().pop()) {
        Some(s) => s,
        None => match DefaultStack::new(STACK) {
            Ok(s) => s,
            Err(_) => super::fail("too many runs are waiting at once for this computer to keep.".into()),
        },
    };
    let co = Coroutine::with_stack(stack, move |y: &Yielder<(), f64>, ()| {
        NOW.with(|n| n.set(y as *const Yielder<(), f64> as usize));
        f();
    });
    go(co)
}

/// A waiting run goes on, until it waits again or ends.
pub fn resume(r: Run) -> Option<(Run, f64)> {
    go(r.0)
}

fn go(mut co: Co) -> Option<(Run, f64)> {
    // Whatever was going before goes on afterwards, even if the run failed.
    struct Back(usize);
    impl Drop for Back {
        fn drop(&mut self) {
            NOW.with(|n| n.set(self.0));
        }
    }
    let _back = Back(NOW.with(|n| n.get()));
    match co.resume(()) {
        CoroutineResult::Yield(due) => Some((Run(co), due)),
        CoroutineResult::Return(()) => {
            let stack = co.into_stack();
            STACKS.with(|s| {
                let mut s = s.borrow_mut();
                if s.len() < SPARE {
                    s.push(stack);
                }
            });
            None
        }
    }
}

/// Pauses the run going now until `due`: the code that started or woke it goes on.
pub fn pause(due: f64) {
    let at = NOW.with(|n| n.get());
    // SAFETY: `at` is the address of the Yielder of the run going now. The library hands it to
    // the run's first function, which keeps it for as long as the run lasts, and the run's own
    // code is the only code going while NOW holds it (go puts the caller's back when the run
    // waits or ends, and pause puts it back when the run goes on).
    #[allow(unsafe_code)]
    let y = unsafe { &*(at as *const Yielder<(), f64>) };
    y.suspend(due);
    NOW.with(|n| n.set(at));
}
