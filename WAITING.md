# Waiting: the design

> **Designed, not built.** This records what the project lead chose on 2026-11-19 (the direction) and 2026-11-26 (the details). [LUST.md](LUST.md) describes `wait` as it works today: `wait 2;` pauses the whole program while tweens move on. Once a part is built, it moves into LUST.md and is removed from this file.

Waiting is Lust's answer to `async` and `await`. Any method can wait, and only the piece of work that's waiting pauses, never the window, the game or the rest of the program. No method is marked as one that waits, so there are no labels to spread through the code, which is how Lua's coroutines and game engines work.

## What the project lead chose

| Question | Choice |
| --- | --- |
| Labels such as `async` | **None.** Any method can wait |
| A method calls one that waits | **It waits for it too**, as with `await` |
| Running something alongside | **`start`:** `start openGate();` runs it alongside, and the next line runs straight away |
| Waiting for something to be true | **`wait until …`:** `wait until door.state == Door.Open;` |
| A time limit | **`or after`:** `wait until … or after 5 { … }`, where the block runs if time ran out |
| Several things at once | **Blocks:** `wait all { … }` and `wait first { … }` |
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

wait until door.state == Door.Open or after 5 {
    print("The door never opened.");
}

wait all {                       -- each line runs alongside; done when all are
    loadMap();
    loadSounds();
    loadPlayer();
}

wait first {                     -- done when the first one is
    wait until boss.hp <= 0;
    wait 60;
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
- **`wait until condition;`:** waits until the condition is true. It's checked once a frame in a window, once a tick in an engine, and once each time the clock moves in a plain program.
- **`… or after seconds { … }`:** a time limit on either kind of wait. If time runs out, the block runs. Either way, the code carries on after it.
- **`start call;`:** runs a method call alongside. It gives nothing back. What it does is seen through fields, events, or a `wait until` on what it changes.
- **`wait all { … }`:** each statement inside runs alongside the others, and the block is done when all of them are. **`wait first { … }`:** done when the first one is, and the others are stopped there.

### Safety

- **The window never freezes.** A button's, key's or timer's code that waits runs on its own, so the window keeps answering.
- **Waiting in a circle** (A waits until B is done, and B waits until A is done) is noticed. When everything waiting could only be woken by other waiters, and no timer, input, answer or event could wake any of them, Lust stops with a message naming the circle: `the guard waits for the gate, and the gate waits for the guard, so neither can ever move.`
- **States clean up.** A run started during a state (by `start`, or a wait inside something made in it) is cancelled when that state ends, like anything else made in it. `keep` keeps it.
- **Events:** an `on` block that waits is a run of its own. The rest of the event queue doesn't wait for it.

### In each place

- **A plain program:** its top-level code is one run. The program ends when that run and every run it started are done (or cancelled).
- **A window:** each click, key, timer and done block is a run, and `wait until` is checked each frame.
- **An engine:** each component update and event handler is a run, and the engine moves waits along once a tick. How runs cross into the engine's own code is settled while building, and written into ENGINES.md.
- **A view's tests:** `wait 2.5` in a test moves every run's clock along, as it does for tweens and timers today.

## Build order

1. **Runs, `start`, and `wait seconds;` pausing only its run,** in plain programs and windows, in both outputs. Lua uses coroutines; built programs get the same order of events.
2. **`wait until`** and **`or after`**.
3. **Slow built-ins** (`Web:get` and the rest) pause only their run.
4. **`wait all` and `wait first`.**
5. **States cancel runs**, and **the circle check**.
6. **Engines:** runs inside components' updates and handlers.

## Still open

- Built programs: how a run pauses in Rust (its own small stack, a state machine Lust writes, or threads). Fast, smooth and robust is the bar, and Lua's coroutines set the behaviour both outputs must match.
- Whether `start` can be stopped by hand (`stop`), or only by its state ending.
- A time limit on a slow built-in call itself: `string page = Web:get(url) or after 10 { … };`.
