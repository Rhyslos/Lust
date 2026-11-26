# Waiting: the design

> **Designed, and being built: step 1 is built (alpha 84).** This records what the project lead chose on 2026-11-19 (the direction) and 2026-11-26 (the details). Runs, `wait seconds;` pausing only its run, and `start call;` are built, and their rules are in [LUST.md](LUST.md) (section 11, Waiting). Once a part is built, it moves into LUST.md and is removed from this file.

Waiting is Lust's answer to `async` and `await`. Any method can wait, and only the piece of work that's waiting pauses, never the window, the game or the rest of the program. No method is marked as one that waits, so there are no labels to spread through the code, which is how Lua's coroutines and game engines work.

## What the project lead chose

| Question | Choice |
| --- | --- |
| Labels such as `async` | **None.** Any method can wait |
| A method calls one that waits | **It waits for it too**, as with `await` |
| Running something alongside | **`start`:** `start openGate();` runs it alongside, and the next line runs straight away |
| Stopping it by hand | **`cancel`:** `start gate = openGate();` gives the work a name, and `cancel gate;` stops it wherever it's waiting |
| Knowing how it went | **Named work has a state,** `Run`: `Waiting`, `Going`, `Done` or `Cancelled`, waited for and checked like any other |
| Waiting for something to be true | **A `wait { }` block of questions:** `wait { is door.state == Door.Open? }`. Lust waits for states to change, not for code to finish |
| Waiting for whichever comes first | **`check` inside `wait { }`:** it waits until the state is one of its `when`s, then runs that one |
| A time limit | **`or after`:** `wait { … } or after 5 { … }`, where the block runs if time ran out |
| Several things at once | **More questions in the block:** all of them have to be true. Work running alongside shows it's done through its state (so no `wait all` or `wait first`) |
| The window and the game | **Never freeze.** Clicks, keys, timers, events and engine ticks each run on their own |
| Waiting in a circle | **Noticed.** Lust stops with a message naming the circle |
| States | **Clean up.** A wait started during a state is cancelled when that state ends (`keep` aside) |

## How it reads

```
method openGate() {
    play(creak);
    wait 2;                      -- only this piece of work pauses
    spawnEnemies();
}

start openGate();                -- runs alongside: the next line runs at once
print("Gate opening");

openGate();                      -- waits here until the gate is open
print("Gate open");

start gate = openGate();         -- a name, so it can be cancelled
…
cancel gate;                     -- stops openGate() wherever it's waiting

wait {                           -- named work has a state of its own
    check gate.state {
        when Done { print("Open!"); }
        when Cancelled { print("Gave up."); }
    }
}

wait {                           -- until the door is open
    is door.state == Door.Open?
} or after 5 {
    print("The door never opened.");
}

start map:load();                -- each sets its own state to Load.Done when it's finished
start sounds:load();
wait {                           -- until both are: every question has to be true
    is map.state == Load.Done?
    is sounds.state == Load.Done?
}

wait {                           -- until the boss is in one of these, then that one runs
    check boss.state {
        when Dead { print("Won!"); }
        when Fleeing { chase(boss); }
    }
} or after 60 {
    print("Too slow.");
}
```

## The rules

### What pauses

Runs, `wait` pausing only its run, and a call waiting for what it calls are built: see LUST.md, section 11. Still to build:

- **An engine tick's update** is a run too (step 6).
- **Slow built-ins pause only their run.** `Web:get(…)`, and later file and database work that takes time, pauses its own run while the window carries on. It's written as an ordinary call: `string page = Web:get(url);`.

### Words

- **`wait { … }`:** waits for things to change. It holds questions and `check`s:
  - **`is condition?`** on its own line: it has to be true. With several, all of them have to be true.
  - **`check thing.state { when … { … } }`:** it has to be in one of the `when`s' options. When it is, that `when`'s block runs. With a `check`, the questions beside it still have to be true too.
  - Lust looks again once a frame in a window, once a tick in an engine, and each time the clock moves in a plain program. If it's all true straight away, nothing waits.
  - Anything else in the block (a `print`, a call) is a mistake that says the block only holds questions and `check`s.
- **`… or after seconds { … }`:** a time limit on either kind of wait. If time runs out, the block runs. Either way, the code carries on after it.
- **`start name = call;`** (plain `start call;` is built) also gives the work a name in that place, like a variable, for `cancel`.
- **`cancel name;`:** stops that work wherever it is: a `wait` it's in never finishes, and the lines after it don't run. Work it `start`ed carries on, unless it was given a name and is cancelled too. Cancelling work that's already done does nothing.
- **Named work has a state,** the built-in state `Run`: `Waiting` (paused in a `wait`), `Going` (running), `Done` or `Cancelled`. So it's waited for like any state: `wait { is gate.state == Run.Done? }`, or `check gate.state { when Done { … } when Cancelled { … } }`. It changes only by itself or through `cancel`, never by `gate.state = …;`. What it does is seen through states, fields or events: a thing that loads sets its own state to `Load.Done`, and `wait { is map.state == Load.Done? }` waits for it.

### Safety

- **Waiting in a circle** (A waits until B is done, and B waits until A is done) is noticed. When everything waiting could only be woken by other waiters, and no timer, input, answer or event could wake any of them, Lust stops with a message naming the circle: `the guard waits for the gate, and the gate waits for the guard, so neither can ever move.`
- **States clean up.** A run started during a state (by `start`, or a wait inside something made in it) is cancelled when that state ends, like anything else made in it. `keep` keeps it.

### In each place

- **A plain program:** (built) the program ends when its own code and every run it started are done, or cancelled once `cancel` is built.
- **A window:** (built for `wait seconds;`) a `wait { }` looks again each frame.
- **An engine:** each component update and event handler is a run, and the engine moves waits along once a tick. How runs cross into the engine's own code is settled while building, and written into ENGINES.md.

## Build order

1. ✅ **Runs, `start`, and `wait seconds;` pausing only its run,** in plain programs and windows, in both outputs. Lua uses coroutines; built programs get the same order of events. Built in alpha 84.
2. **`wait { }` with questions,** and **`or after`**.
3. **Slow built-ins** (`Web:get` and the rest) pause only their run.
4. **`check` inside `wait { }`.**
5. **`cancel`**, **states cancel runs**, and **the circle check**.
6. **Engines:** runs inside components' updates and handlers.

## Decided while building

- **Built programs give each run a stack of its own** (decided 2026-11-26, step 1). Three ways were compared, with fast, smooth and robust as the bar and Lua's coroutines setting the behaviour:
  - **Stacks of their own (stackful coroutines), through the corosensei library** (MIT or Apache): chosen. A wait anywhere, however deep in calls, hooks, loops or `try`, pauses just its run, so the Rust Lust writes doesn't change shape, and runs stay on one thread, where the runtime's shared values (`Rc`) and per-thread state already live. Switching runs takes a few nanoseconds. Each stack is reserved, not used (128 MB of room, 20,000 calls deep; the computer hands over memory as it's touched), on Windows and Linux alike, and finished stacks are used again. corosensei is the library Wasmer uses for the same job, written in inline assembly with no C compiler needed, and works on Windows x86-64, Linux and macOS. The cost: one short `unsafe` block (a wait finding the run it's in), so built programs that use runs say `deny(unsafe_code)` with that one module allowed, instead of `forbid`. The `generator` library offers the same without that block, but it commits each stack's memory in full on Windows (thousands of runs would take gigabytes) and puts in its own crash handler, so it lost.
  - **Threads handing over a baton,** one going at a time: lost. The runtime's values (`Rc`, `RefCell`) and per-thread state (tweens, events, the database) can't move between threads safely, so it would need unsafe code everywhere instead of in one place, and each run would cost a whole thread.
  - **Lust writing waiting methods as state machines** (or Rust's `async`): lost. Every method that might wait, and every hook (which could be anything), would have to be rewritten as a state machine, with `try`, loops and values held across waits taken apart; the Rust would change shape everywhere, and get slower, for a small gain.
- **Programs that never need it don't carry it.** A program that waits and has runs besides its own code (`start`, done blocks, events or a window) gets the stacks; any other program runs its runs straight through, as before, and stays `forbid(unsafe_code)`. Engines' libraries have no runs until step 6, so they're unchanged.

## Still open
- A time limit on a slow built-in call itself: `string page = Web:get(url) or after 10 { … };`.
