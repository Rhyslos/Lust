# Waiting: the design

> **Designed, and being built: steps 1 to 5 are built (alpha 88).** This records what the project lead chose on 2026-11-19 (the direction) and 2026-11-26 (the details). Runs, `wait seconds;` pausing only its run, `start call;` and `start name = call;`, `cancel`, `Run`, `wait { }` with questions and a `check`, `or after`, slow built-ins pausing only their run, states cancelling their work, and the circle check are built, and their rules are in [LUST.md](LUST.md) (section 11, Waiting). Once a part is built, it moves into LUST.md and is removed from this file.

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

Runs, `wait` pausing only its run, a call waiting for what it calls, and slow built-ins (`Web:get`, the file boxes) pausing only their run are built: see LUST.md, section 11. Still to build:

- **An engine tick's update** is a run too (step 6).
- **`Web:serve` pausing only its run:** a server started alongside, answering while timers and other runs go on, and stopped by `cancel`. Left out of step 5: see Decided while building.

### Words

- **`wait { … }`** with questions and a `check`, and **`or after`**, are built: see LUST.md, section 11. Still to build:
  - In an engine, a `wait { }` looks again once a tick (step 6; until then it goes on only if it's already true).
- **`start name = call;`, `cancel name;` and `Run`** are built: see LUST.md, section 11.

### Safety

- **Waiting in a circle** is noticed, and **states clean up** their work: both are built, see LUST.md, section 11.

### In each place

- **A plain program:** (built) the program ends when its own code and every run it started are done or cancelled.
- **A window:** (built) a `wait { }` looks again each frame.
- **An engine:** each component update and event handler is a run, and the engine moves waits along once a tick. How runs cross into the engine's own code is settled while building, and written into ENGINES.md.

## Build order

1. ✅ **Runs, `start`, and `wait seconds;` pausing only its run,** in plain programs and windows, in both outputs. Lua uses coroutines; built programs get the same order of events. Built in alpha 84.
2. ✅ **`wait { }` with questions,** and **`or after`**. Built in alpha 85.
3. ✅ **Slow built-ins** (`Web:get`, `Web:post` and the file boxes) pause only their run. Built in alpha 86.
4. ✅ **`check` inside `wait { }`.** Built in alpha 87.
5. ✅ **`cancel`**, **states cancel runs**, and **the circle check**. Built in alpha 88.
6. **Engines:** runs inside components' updates and handlers.

## Decided while building

- **Built programs give each run a stack of its own** (decided 2026-11-26, step 1). Three ways were compared, with fast, smooth and robust as the bar and Lua's coroutines setting the behaviour:
  - **Stacks of their own (stackful coroutines), through the corosensei library** (MIT or Apache): chosen. A wait anywhere, however deep in calls, hooks, loops or `try`, pauses just its run, so the Rust Lust writes doesn't change shape, and runs stay on one thread, where the runtime's shared values (`Rc`) and per-thread state already live. Switching runs takes a few nanoseconds. Each stack is reserved, not used (128 MB of room, 20,000 calls deep; the computer hands over memory as it's touched), on Windows and Linux alike, and finished stacks are used again. corosensei is the library Wasmer uses for the same job, written in inline assembly with no C compiler needed, and works on Windows x86-64, Linux and macOS. The cost: one short `unsafe` block (a wait finding the run it's in), so built programs that use runs say `deny(unsafe_code)` with that one module allowed, instead of `forbid`. The `generator` library offers the same without that block, but it commits each stack's memory in full on Windows (thousands of runs would take gigabytes) and puts in its own crash handler, so it lost.
  - **Threads handing over a baton,** one going at a time: lost. The runtime's values (`Rc`, `RefCell`) and per-thread state (tweens, events, the database) can't move between threads safely, so it would need unsafe code everywhere instead of in one place, and each run would cost a whole thread.
  - **Lust writing waiting methods as state machines** (or Rust's `async`): lost. Every method that might wait, and every hook (which could be anything), would have to be rewritten as a state machine, with `try`, loops and values held across waits taken apart; the Rust would change shape everywhere, and get slower, for a small gain.
- **`wait { }` looks again on the clock's 1/60 s steps** (decided 2026-11-26, step 2). A run in a `wait { }` waits like `wait seconds;` until the clock's next 1/60 s step (or its time limit, if that's sooner), looks, and waits again if it has to. Looking again whenever any other run goes on was the other way: it can see a change a little sooner, but it needs a second list of waiting runs beside the clock's, and a run that's woken by another waiter's change in the same moment could be missed. On the clock's steps, a window's frames, a plain program and later an engine's ticks all work alike, and runs look in the order they started waiting, the same in both outputs.
- **A time limit that can't be reached is a mistake only when both are written as numbers** (step 2): `wait 2 or after 3 { … }` has a block that can never run, which is worth saying; `wait secs or after limit { … }` is fine, as which is shorter is only known as it runs.
- **In an engine, `wait { }` doesn't wait yet** (step 2). Nothing else runs there while code waits (step 1 kept the engine's old `wait`), so questions that are false couldn't come true: it goes straight on when they're true, and otherwise stops with a message, rather than freezing the engine. Step 6 makes it wait once a tick.
- **Programs that never need it don't carry it.** A program that waits and has runs besides its own code (`start`, done blocks, events or a window) gets the stacks; any other program runs its runs straight through, as before, and stays `forbid(unsafe_code)`. Engines' libraries have no runs until step 6, so they're unchanged.

- **Slow built-ins work on a thread of their own, and the run looks for the answer on the clock's 1/60 s steps** (decided 2026-11-26, step 3). The run pauses as `wait 0;` does, so the runs due now go first even when the answer is quick, then looks once a step, like `wait { }`, with the answer handed back over a channel. Waking the run the moment the answer came was the other way: a little sooner, but it needs the clock's loop to sleep on the channel and the timers at once, in both outputs and in windows, and the order runs wake in would follow the network. On the steps, both outputs and windows share the loop steps 1 and 2 built.
- **In `lust test`, slow work takes no time on Lust's clock** (step 3): the first look waits for the answer, so the clock never moves while one is out. Tests then print the same every time, however slow the computer or the network; the run still pauses for the runs due now first, as it does for real.
- **Which built-ins** (step 3): asking a website and the file boxes, where the wait is long and the window freezing is the problem. Not files and databases: they're quick, and runs working on the same file or table at once would see each other's half-done work (and SQLite's connection lives on the program's thread). Not `ask(…)`: it types into the console, which other runs would print over. Not `Web:serve` yet: a server that pauses only its run never ends, so it needs `cancel` (step 5) first. On macOS the file boxes open only on the main thread, so there they still hold everything up.
- **Where a run can't pause, slow built-ins hold everything up, as before** (step 3): in an engine, in the code that draws a view, and in a route's method answering a request (Lua can't pause inside the `pcall` that catches its failure, so neither output does, and the answers stay the same).
- **No mistakes are new** (step 3): a slow built-in is an ordinary call, so it's checked as before; a program that asks a website from a run gets runs' stacks as a program that waits does.
- **One `check` per `wait { }`** (decided 2026-11-26, step 4). All of several checks having to match was the other way, but then which `when` runs, one from each check in turn or only the first, isn't clear from reading it, and the same can be written with one check and questions beside it. A second check is a mistake that says so.
- **The `when` runs on the option the last look saw** (step 4): each look takes the state once, asks the questions and the `when`s' options on it, and once they match, the `when`s are gone through on that same value, so a state that can only be read by calling a method is read once a look, and the `when` that runs is always one that matched.
- **A `check` in a `wait { }` is for states only** (step 4): a bool's `when true` is a question (`wait { is door.open? }`), which the mistake shows. A check with no `when` could never end the wait, so it's a mistake too.

- **A name for work lives in a variable only** (decided 2026-11-26, step 5). Fields were the other way, so one method can start work and another cancel it, but that needs a type to write in `has { }` (and in arrays, parameters and what a method gives back), which is a new word for the project lead to choose. Until then, work to stop from elsewhere is started during a state, and the state changing cancels it. The name is declared where the `start` is, and always names that work, so it can't be given anything else.
- **Cancelling stops the work's code for good** (step 5): a cancelled run that's waiting is never woken again (Lua drops its coroutine; a built program wakes it once, to unwind its stack with a marker no `try` catches, so its Rust values are let go, but no Lust code runs). A `try` around the cancelled wait runs neither its `error` block nor anything after it, since a cancel isn't a failure. Cancelling itself stops the run at once, and work that's going but not the run going now (one that started it, or one waiting for an event's handler) stops as soon as it would go on.
- **Which state owns work** (step 5): the one that would own something made with `new` on the `start` line, that is the option of the thing whose method has the `start` (or its node), and also the thing whose method the work runs (`start guard:patrol();`, `start patrol();` in the guard's method, and each event handler), until that thing ends. The second way is what "a wait inside something made in it" covers: no run wakes inside a thing that's ended. Work that ends its own state is cancelled there and then.
- **There's no `Failed` option** (step 5, confirming step 1): a failure no `try` in its own run catches still stops the program, so work only ever ends `Done` or `Cancelled`.
- **The circle check notices only what's certain** (step 5). A run is stuck when it's in a `wait { }` with no time limit whose questions and `check` call nothing (a method, hook, built-in, `Random`, save or database read could change by itself, as could a question about the time). The program stops when every waiting run is stuck, no tween is moving, no window is open, and each has looked again since anything else ran (counted, not timed, so both outputs stop at the same look). Naming the exact circle (who waits for whom) would need to know what each question reads and who writes it; the message names every stuck run and the line of its wait instead, which is the same list when they're all in the circle.
- **`Web:serve` still holds everything up** (step 5). For a server to pause only its run, both outputs' servers (lust's host in run.rs and the built program's tiny_http loop) would look for requests on Lust's 1/60 s steps instead of waiting for them, with a route's method answering inside a run that can't pause; with `cancel` in place, stopping it is the easy half. It's left for a step of its own, so it can be tried with real requests.
- **In an engine** (step 5, until step 6): `start gate = …` calls the method straight through, as `start` does there, so the work is `Done` by the next line and `cancel` then does nothing; work cancelled while going stops, as elsewhere.

## Still open
- Names for work in fields, arrays and parameters, which needs a type word for it.
- A time limit on a slow built-in call itself: `string page = Web:get(url) or after 10 { … };`.
