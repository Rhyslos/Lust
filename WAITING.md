# Waiting: the design

> **Designed, not built.** This records what the project lead chose on 2026-11-19 (the direction) and 2026-11-26 (the details). [LUST.md](LUST.md) describes `wait` as it works today: `wait 2;` pauses the whole program while tweens move on. Once a part is built, it moves into LUST.md and is removed from this file.

Waiting is Lust's answer to `async` and `await`. Any method can wait, and only the piece of work that's waiting pauses, never the window, the game or the rest of the program. No method is marked as one that waits, so there are no labels to spread through the code, which is how Lua's coroutines and game engines work.

## What the project lead chose

| Question | Choice |
| --- | --- |
| Labels such as `async` | **None.** Any method can wait |
| A method calls one that waits | **It waits for it too**, as with `await` |
| Running something alongside | **`start`:** `start openGate();` runs it alongside, and the next line runs straight away |
| Stopping it by hand | **`cancel`:** `start gate = openGate();` gives the work a name, and `cancel gate;` stops it wherever it's waiting |
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

- **A piece of work** (a *run*) is the program's own code from the top, one click's or key's code, a timer's, one event's `on` block, an engine tick's update, or a method started with `start`.
- **`wait` pauses its run,** and everything else carries on: the window draws and answers clicks, other runs go on, and tweens and timers move.
- **A call waits for what it calls.** If `openGate()` waits, whoever called it waits with it, just as it would wait for a long loop. Nothing is marked: a method that waits is called like any other.
- **Slow built-ins pause only their run.** `Web:get(…)`, and later file and database work that takes time, pauses its own run while the window carries on. It's written as an ordinary call: `string page = Web:get(url);`.

### Words

- **`wait seconds;`:** lets that many seconds go by.
- **`wait { … }`:** waits for things to change. It holds questions and `check`s:
  - **`is condition?`** on its own line: it has to be true. With several, all of them have to be true.
  - **`check thing.state { when … { … } }`:** it has to be in one of the `when`s' options. When it is, that `when`'s block runs. With a `check`, the questions beside it still have to be true too.
  - Lust looks again once a frame in a window, once a tick in an engine, and each time the clock moves in a plain program. If it's all true straight away, nothing waits.
  - Anything else in the block (a `print`, a call) is a mistake that says the block only holds questions and `check`s.
- **`… or after seconds { … }`:** a time limit on either kind of wait. If time runs out, the block runs. Either way, the code carries on after it.
- **`start call;`:** runs a method call alongside. It gives nothing back. **`start name = call;`** also gives the work a name in that place, like a variable, for `cancel`.
- **`cancel name;`:** stops that work wherever it is: a `wait` it's in never finishes, and the lines after it don't run. Work it `start`ed carries on, unless it was given a name and is cancelled too. Cancelling work that's already done does nothing. Code can ask `is gate.running?`. What it does is seen through states, fields or events: a thing that loads sets its own state to `Load.Done`, and `wait { is map.state == Load.Done? }` waits for it.

### Safety

- **The window never freezes.** A button's, key's or timer's code that waits runs on its own, so the window keeps answering.
- **Waiting in a circle** (A waits until B is done, and B waits until A is done) is noticed. When everything waiting could only be woken by other waiters, and no timer, input, answer or event could wake any of them, Lust stops with a message naming the circle: `the guard waits for the gate, and the gate waits for the guard, so neither can ever move.`
- **States clean up.** A run started during a state (by `start`, or a wait inside something made in it) is cancelled when that state ends, like anything else made in it. `keep` keeps it.
- **Events:** an `on` block that waits is a run of its own. The rest of the event queue doesn't wait for it.

### In each place

- **A plain program:** its top-level code is one run. The program ends when that run and every run it started are done (or cancelled).
- **A window:** each click, key, timer and done block is a run, and a `wait { }` looks again each frame.
- **An engine:** each component update and event handler is a run, and the engine moves waits along once a tick. How runs cross into the engine's own code is settled while building, and written into ENGINES.md.
- **A view's tests:** `wait 2.5` in a test moves every run's clock along, as it does for tweens and timers today.

## Build order

1. **Runs, `start`, and `wait seconds;` pausing only its run,** in plain programs and windows, in both outputs. Lua uses coroutines; built programs get the same order of events.
2. **`wait { }` with questions,** and **`or after`**.
3. **Slow built-ins** (`Web:get` and the rest) pause only their run.
4. **`check` inside `wait { }`.**
5. **`cancel`**, **states cancel runs**, and **the circle check**.
6. **Engines:** runs inside components' updates and handlers.

## Still open

- Built programs: how a run pauses in Rust (its own small stack, a state machine Lust writes, or threads). Fast, smooth and robust is the bar, and Lua's coroutines set the behaviour both outputs must match.
- A time limit on a slow built-in call itself: `string page = Web:get(url) or after 10 { … };`.
