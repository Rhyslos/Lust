# Notes for engines using Lust

This file is for programs that use Lust as a library: Solis first. When an engine updates its pinned Lust, it reads the section for each version it skips over, newest at the bottom, and does what **The engine does** says. The full rules are in [LUST.md](LUST.md), section 21.

An engine moving from alpha 48 to alpha 67 or later reads everything below.

## alpha 49: states in native methods

- A native method can take and give back any state, including options inside options: `method animate(Clip clip) native;`.
- **Rust:** a state comes in as `&'static str`, the option's full name without the state's (`"Farmer.Walk"`). One goes back the same way. There is no number: match on the name.
- **Lua:** the same full-name text.
- **A name given back that the state doesn't have** stops the program with the calling line. Lust never quietly picks another option.
- **The engine does:** passes and returns states as full names.

## alpha 50: methods on entity

- An engine's own files give `entity` native methods: `method vec3 entity:position() native;`.
- **Which files are the engine's:** they're the files in its **host folders**, named with `lust::Engine { folders: vec!["solis".into()] }` and the `_with` calls (`check_with`, `to_lua_with`, `to_rust_library_with`, `read_values_with`). A game's own file declaring one is a mistake.
- **Rust:** the host trait gets `entity_<name>(&self, this: Option<u64>, …)`.
- **Lua:** `LUST_NATIVES["entity_<name>"](e, …)`.
- `lust.host` (a file listing folders, one per line) is only for `lust check` and tests on the disk. A program using the compiler goes by its own `lust::Engine`, never a `lust.host`.
- **The engine does:** compiles games with the `_with` calls and its folders.

## alpha 51: abilities called bare, and `this`

- Inside a component, the entity's methods are called bare: `moveTo(place)`, `velocity()`. On any other entity they're called with `:`, as in `target:push(force)`.
- A variable may share a name with an ability: `vec3 velocity = velocity();` is fine.
- **Name clashes:** a component's own method with the same name as an ability wins. The ability stays reachable as `this:jump()`, so adding an ability to the engine never breaks a game.
- `this` is the whole entity: `send Hit(this, 6) to target;`.
- `me` still works, with a note. The note shows in `LuaChunk::notes` and from `lust check`; a Rust library has nowhere to put notes.
- **The engine does:** shows `LuaChunk::notes` to the game writer, and moves its samples, kit and manual from `me` to `this`. `me` becomes a mistake once Solis has moved over.

## alpha 52: fields the engine fills in

- A component may list a field of an engine class with no starting value: `FarmerAnimator anim;`. That's a class declared in the host folders; a game's own class still needs a starting value.
- **Lua:** the schema entry is `{ key, name, "object", "FarmerAnimator" }`. `api.make(name, values)` makes one, and `api.classes` lists them.
- **Rust:** `Farmer::FILLED` lists the field and class pairs, and `set_<field>` fills one.
- Using the field before it's filled stops with the line and, from alpha 58, the field's name.
- **The engine does:** fills every such field before the component's first `update`.

## alpha 53 and 55: data files

- A data file is an ordinary `.lst` of `lock` values of fixed values.
- **Reading one:** `lust::read_values(entry, files)` gives back its values without running anything. `read_values_with(engine, entry, files)` also takes the engine's folders. The files are handed over as for `lust::check`: the data file and what it fetches.
- **Checking (alpha 55):** it's checked like any file. A wrong type, an unknown class or field, or a missing option is a mistake with its line, and a dotted name must be a state's option.
- **The signature changed in alpha 55** from `(path, text)` to `(entry, files)`.
- **Speed:** about 0.05 s for 1 MB and about 0.7 s for 19 MB, in a release build.
- **The engine does:** reads maps and other editor data with `read_values_with`. Game code never fetches a big data file, only the names it uses.

## alpha 54 and 56: objects in an engine

- An `object` is one for the whole game, reached by name: `Clock.hour`.
- **Passed objects only (alpha 56):** an object is ticked and saved only when its file passes it (`pass Clock;`). That can be any file of the game.
- **Ticking:** its `update(float dt)` runs once a tick, objects in name order, before the components. An `update` the engine can't call is a mistake from `to_lua` and `to_rust_library`.
- **Lua:** `objects`, `objectNames`, `tick(dt)`, `save(name)` and `load(name, values)`.
  - `load` leaves a value of the wrong kind as it is (alpha 56).
  - A name that isn't a passed object stops with a message.
- **Rust:** each object has `NAME`, `schema()`, `to_data()` and `set_data(&data)`, plus `objects()` and `tick_objects(dt)`.
- **The engine does:** calls `tick` once a tick before the components, keeps every object's fields in its saves, and tells game writers to `pass` their objects.

## alpha 57: tidy-ups from review

- `entity:text()` is a mistake in an engine file: `.text()` is Lust's own.
- A game method called `entity_<name>` is a mistake when the engine gives `entity:<name>`, since both would be `entity_<name>` in Rust and Lua.
- **Messages:**
  - `this` outside a component says only that it's a component's.
  - A repeated entity method says which line it's repeated from.
- **The engine does:** nothing, unless one of its files declares `entity:text()`.

## alpha 58: an unfilled engine field is named

- Reading a component's field the engine fills in (alpha 52) before it's filled stops on its line, saying which field: "anim is filled in by the engine before the component's first update, and hasn't been yet." Before, it said only "this is nothing".
- **The engine does:** nothing; it still fills every such field before the first `update`.

## alpha 59: the engine's components join only when fetched

- Files in the host folders still come with every program, for their `entity` methods. But a component or object in one (a ready-made component in `solis/kit/`, say) is part of the game only once the game fetches its file, directly or through another file.
- Before, every one of them joined every game: they showed among its components, and a game's own component of the same name clashed with them.
- **The engine does:** nothing. A game that used a ready-made component without fetching it now has to fetch it, as the manual already says.

## alpha 62: number helpers, array maths and random tools

- New calls that need nothing from the engine: `Maths:lerp`, `sign`, `log`, `exp`, `trunc`, `fraction`, `moveTowards`, `smoothstep`, `isFinite`, `noise` and `Maths.e`; `sum`, `average`, `smallest`, `largest`, `pick` and `shuffle` on arrays. They use only Lua's standard `math` library, which Luau has.
- `Random:seed(n)`, `Random:chance(p)` and `Random:direction()` (a vec3) use the same rolls as `Random[a, b]`.
- **Lua:** an engine that gives Lust its own rolls through `LUST_HOST.random` and `LUST_HOST.frandom` should also give `LUST_HOST.seed(n)`, so `Random:seed` restarts them. Without it, `Random:seed` seeds Lua's own `math.random`.
- `Maths`, `Files`, `Time`, `Json`, `Web` and `Random` stay names a game can't use.
- **The engine does:** nothing, unless it gives Lust its own random rolls (above).

## alpha 63: rotation and vector helpers

- New calls on values that already cross to engines: `Rotation:toward(dir)`, `Rotation:around(axis, degrees)`, `a:lerp(b, t)` on rotations and `a:angle(b)` on vec3s. Rotations and vec3s are kept and passed exactly as before.
- `Rotation` becomes a built-in object, so a game can't use it as a name (the type `rotation` is unchanged).
- **The engine does:** nothing.

## alpha 65: vec2 and colour

- Two new value types cross to engines, in native methods (taking and giving back, and in arrays), components' fields, objects' kept fields and events.
- **`vec2`:** two f32s, x and y.
  - **Rust:** the library's `Vec2 { x: f32, y: f32 }`. `FieldKind::Vec2`, and its `Value` is `List` of two numbers.
  - **Lua:** a table `{ x = …, y = … }`, in Luau too (Luau's `vector` has three numbers, and a table keeps a vec2 apart from a vec3). Its schema kind is `"vec2"`. A native may give back any table with `x` and `y`, or a Luau vector (its z is dropped).
- **`colour`:** red, green, blue and see-through, four f32s from 0 to 1.
  - **Rust:** the library's `Colour { r: f32, g: f32, b: f32, a: f32 }`, with `to_array()` for `[f32; 4]` and `Colour::from([r, g, b, a])`. `FieldKind::Colour`, and its `Value` is `List` of four numbers. Solis's own `Color` is three sRGB numbers, so the adapter adds or drops the see-through.
  - **Lua:** a table `{ r = …, g = …, b = …, a = … }`. Its schema kind is `"colour"`. A native may leave out `a`, which is then 1.
  - Every way in (a native's answer, `set_data`, `from_data`, an event's values, `Colour::from`) keeps each part between 0 and 1; not a number becomes 0.
- Lust never changes a vec2 or colour table in place, so it shares them; an engine never changes one it got from Lust.
- The library now also exports `Vec2` and `Colour` next to `Vec3` and `Rotation`.
- `Colour` becomes a built-in object, so a game can't use it as a name (the type `colour` is a new word too, but a variable may still be called `colour`).
- They don't go into JSON, Lust's own saves or databases yet.
- **The engine does:** adds `Vec2` and `Colour` to its adapter's kinds, if its inspector should show them.

## alpha 66: vectors, rotations and colours in JSON, saves, databases and data files

- `vec3`, `vec2`, `rotation` and `colour` go into `Json:write` and `Json:read`, Lust's own saves and database columns, exactly. Nothing changes in how they cross to an engine.
  - **JSON:** a vec3 is `[x, y, z]`, a vec2 `[x, y]`, a rotation its quaternion `[x, y, z, w]` (three numbers are read as degrees), and a colour its hex text when that's exact, or `[r, g, b, a]`. Each number is the shortest text that reads back as the same f32. Runs the same in Luau.
- **Data files:** a colour written as a colour word (`orange5`) or `Colour:hex("#3a8f2a")` comes through as a new `Fixed::Colour([r, g, b, a])`, four f32s from 0 to 1. Colours written as numbers, vec3s, vec2s and rotations (as degrees) still come as `Fixed::List`. `as_vec3()`, `as_vec2()` and `as_colour()` read either form. `Rotation:toward(…)` stays a mistake in a data file.
  - **Rust:** a `match` on `Fixed` that lists every variant needs an arm for `Colour`.
- **Engine saves, Lua (`load(name, values)`):** a vec3, vec2, rotation or colour is now taken only when it has numbers for its parts (`x`, `y`, `z`; `x`, `y`; `x`, `y`, `z`, `w`; `r`, `g`, `b`, with `a` optional), as Rust's `set_data` already checks; anything else leaves the field as it is. What's taken is made Lust's own: rounded to f32 (a Luau vector for a vec3), and a colour kept between 0 and 1. Before, any table was taken as it was.
- **Rust (`to_data`, `set_data`):** unchanged; they already kept all four kinds, as `Value::List` of numbers.
- **The engine does:** adds a `Fixed::Colour` arm where it matches on `Fixed`, and keeps vectors in its own saves as tables with named parts, as `save(name)` gives them.

## alpha 67: tweens

- Game code can glide a field to a new value over time: `tween hp to 0 over 0.5 [ease OutCubic] { … }`. In an engine, the engine moves tweens on, once a tick.
- **Lua:** the chunk's table has `tickTweens(dt)`.
- **Rust:** the library has `tick_tweens(dt) -> Result<(), LustError>` (in every library, so it's always there to call).
- **When:** once a tick, **after** `tick` / `tick_objects` and **before** the components' updates. An `update` then sees this tick's values, and a done block's changes (which can start more tweens, or `send` events) are in place for the same tick's updates. A tween started during an `update` first moves on the next tick. 0 moves nothing.
- **Same numbers both ways:** the steps and easing curves are worked out the same in Luau and Rust, so a game gives the same values either way.
- **What a tween keeps:** a tween on a component's (or object's, or thing's) field holds that component until the tween ends.
- **Hot reload (`swap`):** tweens live in the chunk that started them. Once the engine calls the new chunk's `tickTweens` instead of the old one's, tweens the old code started stop where they are, without their done blocks.
- **Removing an entity:** Lust has no hook to hear that an entity or component is gone, so its tweens keep running until they end: they move a component nobody ticks any more, and their done blocks still run. A `send` from one of those done blocks names an entity the engine has removed; the engine should ignore it. A way to stop a component's tweens can come later if Solis needs it.
- `Ease` becomes a built-in object and state, so a game can't use it as a name. A component's field can hold an `Ease` like any state (its schema kind is `state`, with the curves as options).
- **The engine does:** calls `tickTweens(dt)` (Lua) or `tick_tweens(dt)` (Rust) once a tick between the objects and the components, and ignores events sent to entities it has removed.

## alpha 72: toasts, App:quit() and App:notify()

- Game code can now write `toast "Saved";`, `App:quit()` and `App:notify("Title", "Body")`. They're for Lust's own windows (views), which an engine doesn't use.
- **Lua (Luau):** all three do nothing: the runtime only calls them when the host gives `ui_toast`, `app_quit` or `app_notify`, and an engine's host doesn't. Nothing is refused, so shared code still loads.
- **Rust (`lust build --library`):** a library without a view writes `toast` and `App:quit()` as nothing. `App:notify` brings in the view runtime (and winit), so leave it out of game code.
- `App` isn't reserved: a game's own class can still be called `App`.
- **The engine does:** nothing, unless it wants to show toasts or notifications its own way, which can come later if Solis needs it.

## alpha 74: states are set with =

- A state is set with `=` now, like any value: `now = Mood.Busy;`, and `state = Mood.Calm;` in a `has` or a method. `is` only asks questions. Game files that set a state with `is` (`state is Mood.Calm;`) get a mistake that shows the `=` way.
- Nothing changes in the Lua or Rust that's written: an object's state still changes through the same call, which ends what was made in the option it leaves.
- **The engine does:** changes `is` to `=` wherever its own Lust files, or the examples and templates it ships, set a state.

## alpha 81: handlers go in an on block, and Lust's own events

- A component's handlers go together in one `on { }` block, like `has { }`: `on { Hit(entity by, float damage) { … } Died() { … } }`. The old `on Hit(…) { … }` on its own is a mistake that shows the new form. Nothing changes in the Lua or Rust written for a component: `HANDLES` and `handle(event, values)` are as they were.
- `send Hit(…) to entity;` is exactly as before: the host's `send_event`, `--library` only.
- **Lust-side listeners:** any class or object can have an `on` block too, and `send Saved(…);` (no `to`) or `send Saved(…) to thing;` stays inside Lust: no host function, no engine queue. A component never hears these; only an entity send reaches it. Those events are delivered after every call into the library (`rt::catch` delivers before giving back: `update`, `handle`, `tick_objects`, `tick_tweens`, `start`, getters and setters), and in Lua after the chunk's `tick` and `tickTweens`, and after a component's `update` and `handle`. The chunk also has `settle()`, which delivers them, for an engine that runs Lust code another way.
- **Events that reach the engine** (handled by a component, or sent to an entity) still carry only the engine's kinds. Events only Lust hears can carry anything, things of the game's classes too, so `events()` (Rust) and the chunk's `events` (Lua) list only the events the engine can carry.
- A program whose only sends stay in Lust no longer needs `--library`.
- **The engine does:** changes `on X(…) { … }` to the `on { }` block in its own Lust files, samples and templates.

## alpha 82: errors carry a Problem

- Game code can tell kinds of failure apart: `try { … } error (Problem p, string message) { … }`, where `Problem` is a built-in state (`NotFound`, `BadJson`, `OutOfRange`, … and `Other`; the list is in LUST.md, section 15). `error (string message)` still works, and an error nobody catches stops with the same message as before.
- **Rust:** `LustError` has a new field, `problem: &'static str`, the failure's option by name (`"NotFound"`, `"Other"`), so the engine can tell kinds apart too. Code that only reads a `LustError` needs no change; code that builds one with `LustError { … }` needs the new field.
- **Lua:** nothing changes for the engine: an error that leaves Lust is the same message as before.
- `Problem` is a taken name now, like `Icon`: an engine's own class or state can't be called `Problem`.

## alpha 83: built-ins' fixed choices are states

- `Time:weekday(date)` gives back an option of the built-in state `Day` (`Day.Monday` … `Day.Sunday`), not text. Printing one, or `.text()`, still gives `"Monday"`, so only code that compares it with text, or keeps it in a `string`, needs a change: the mistake says which option to write.
- `Web:request` takes a `Request` (`Request.Get`, `Post`, `Put`, `Delete`) and `Web:fail` an `Answer` (`Answer.NotFound` and the rest; the table is in LUST.md, section 18). Servers aren't for game code, so an engine is unlikely to meet these.
- **Lua and Rust:** a state's option is its name in both, as before. In Rust, `rt::time_weekday` gives back `&'static str` now, and `rt::web_fail` and `rt::web_request` take one.
- `Day`, `Request` and `Answer` are taken names now, like `Icon` and `Problem`: an engine's own class or state can't be called any of them.
- **The engine does:** renames any class or state of its own called `Day`, `Request` or `Answer` in its Lust files, samples and templates.

## alpha 84: waiting, step 1

- Plain programs and windows have runs now: `wait seconds;` pauses only the piece of work that waits, and `start call;` runs a call alongside (LUST.md, section 11, Waiting; the rest is designed in WAITING.md).
- **In an engine nothing changes yet** (runs in components come in step 6): `start openGate();` just calls the method, straight through, and `wait` pauses the whole call while tweens move, as before. Event handlers and tweens' done blocks run as they did.
- **Rust:** a library still has no unsafe code and no new dependencies. Built programs that wait while other runs go on use the corosensei library for runs' stacks; that's the shared runtime's, not the engine's.
- **Lua:** a chunk for an engine is the same as before. `try` now goes through the runtime (`_L.try`), which is `pcall` when nothing waits inside it.
- `start` is a word only before a call (`start openGate();`), so names called `start`, and calls like `start();`, still work.

## alpha 85: waiting, step 2

- `wait { is …? }` waits until every question is true, and `… or after seconds { … }` puts a time limit on a wait (LUST.md, section 11).
- **In an engine nothing waits yet** (step 6): a `wait { }` whose questions are already true goes straight on, and one whose questions aren't stops the call with a message (`wait { } can't wait in an engine yet …`, problem `Other`), as nothing else runs while it would wait. `wait seconds or after limit { … }` waits the shorter of the two, as before, and runs the block when the limit was shorter.
- **Rust and Lua:** the new calls are the shared runtime's (`rt::wait_for`, `rt::wait_or`; `_L.wait_for`, `_L.wait_or`); nothing changes for the engine's own code.
- `after` is a word only straight after a wait's `or`, so engines' names called `after` still work.

## alpha 86: waiting, step 3

- Asking a website (`Web:get`, `Web:post`) and the computer's own file boxes pause only their run in plain programs and windows (LUST.md, section 11, Slow built-ins pause only their run).
- **In an engine nothing changes** (runs come in step 6): a call that asks a website holds the call up until the answer comes, with the same text and the same `Problem` on failure, as before.
- **Rust:** a library still has no unsafe code and no new dependencies. The request now happens on a thread of its own, which the library waits for, so the engine sees no difference.
- **Lua:** nothing changes for a chunk in an engine. `lust run` gives the runtime new host calls (`http_start`, `choose_open_start`, `choose_save_start`, `choose_folder_start`, `slow_look`, `slow_wait`); a host without them gets the old blocking `http` and `choose_*` calls, as before.

## alpha 87: waiting, step 4

- A `check` inside `wait { }` waits until a state is one of its `when`s' options, then runs that `when` (LUST.md, section 11).
- **In an engine nothing waits yet** (step 6): as for `wait { }`, it goes straight on, running its `when`, when the state is already one of its options and the questions are true, and otherwise stops the call with the same message (problem `Other`).
- **Rust and Lua:** it goes through the same runtime calls as `wait { }` (`rt::wait_for`, `_L.wait_for`); nothing is new for the engine.

## alpha 88: waiting, step 5

- `start gate = openGate();` names started work, `cancel gate;` stops it, and `gate.state` says how it's going, as one of the built-in state `Run`'s options (LUST.md, section 11).
- **In an engine nothing waits yet** (step 6): `start gate = …` calls the method straight through, as `start` does, so the work is `Run.Done` by the next line and cancelling it then does nothing. Work cancelled while it's going (by a hook that holds its name) stops at once: `rt::begin` and `_L.start` catch that stop, so it never reaches the engine as a failure.
- **`Run` is a built-in state now,** so a game's own state, class or object can't be called `Run` (an option can).
- **Rust:** started work, event handlers and tweens' done blocks go through `rt::begin` (which catches a cancel and passes every failure on as before); `rt::wait_for` takes a third value, used by the circle check. No new dependencies and no unsafe code in a library.
- **Lua:** event handlers and tweens' done blocks in an engine run inside a `pcall` that passes every failure on unchanged, so a cancel can stop them; a chunk is otherwise the same.


## alpha 89: waiting, step 6: runs in engines

Game code can now wait in an engine the way it does in a window (LUST.md, section 11, Waiting, and section 21, Runs in an engine). A `wait` pauses only its run, across the engine's ticks, while the engine keeps calling every update.

- **What's a run:** each event handler (a component's `on { }`, and Lust's own events), each `start`, and each tween's done block. A run goes straight away until its first wait, then goes on in a later tick.
- **What can't wait:** the code the engine calls straight into: a component's `update`, an object's `update` (`tick`, `tick_objects`), a passed method, and the program's own start. `update` is still called every tick, as before; to wait, game code starts a method from it (`start openGate();`), as game engines' coroutines are started from an update. A `wait` written straight in a component's `update` is a mistake; one reached through a method that `update` calls stops that call with "this wait would hold up the whole game: code the engine calls (an update, an object's tick, a method it calls) can't wait. Put the waiting in a method of its own and start it: start openGate();" (problem `Other`). Before, `wait` there held up the whole engine for that long, and `wait { }` stopped with "can't wait in an engine yet".
- **The engine's clock:** runs move when the engine calls **`tickTweens(dt)`** (Lua) or **`tick_tweens(dt)`** (Rust), which it already calls once a tick, after the objects and before the components' updates. It now moves Lust's clock on by `dt`, moves the tweens, then wakes every run whose wait is over, in the order they're due (and of two due together, the one that started waiting first). Runs that start waiting during it, or in the updates after it, go on in a later tick.
  - **Engine time, not wall time:** `wait 2;` is over once the engine's `dt`s add up to 2 seconds, at the first tick that reaches it. A paused game (dt 0) moves no waits, and a slowed one (smaller dt) moves them slower. `or after` counts the same time.
  - **`wait { }` and `check` in waits look once a tick,** in that call, even with dt 0. `wait 0;` goes on in the next tick.
  - **An engine that never calls it** (or only calls it with 0) never freezes: runs just stay where they wait.
- **Removing a component:** the new **`removed(component)`** (Lua: `api.removed(gun)`; Rust: `gun.removed()`) says the engine has removed it (its entity is gone, or the component was taken off). The work it started and its handlers still waiting are cancelled (none of their code runs again), things it made in its states end, and its tweens stop where they are, without their done blocks. Call it once, when the component stops running for good. Work started in a component's methods belongs to the component for this, whatever method it runs. A component's method can't be called `removed` now.
- **Failures:** a failure in a run comes back from the call that was running it: from `update` or `handle` when it fails before its first wait, and from `tickTweens` / `tick_tweens` when it fails after one, with the file and line of the run's own code. Runs due after it in that tick go on in the next. `handle` gives back true once the handler has started, whether or not it's still waiting.
- **The circle check is off in engines:** the engine keeps ticking, and its updates can change anything a `wait { }` asks about, so nothing is ever certain to be stuck.
- **New code (`swap`):** runs live in the chunk that started them, like tweens: once the engine calls the new chunk's `tickTweens` instead of the old one's, the old code's waiting runs never go on.
- **Slow built-ins** (`Web:get`, file boxes) pause only their run in a Rust library, looking for the answer once a tick; in code the engine calls straight into they hold the call up, as before. Luau has neither.
- **Rust:** a library whose code waits (or asks a website) has runs on stacks of their own, as built programs do: its `Cargo.toml` has `corosensei = "0.3"` (MIT or Apache), and it says `#![deny(unsafe_code)]` with the one runs module allowed, instead of `forbid`. A library that never waits is unchanged. Each component has `removed()`. `rt::wait_all` is gone.
- **Lua:** the chunk calls `_L.runs_engine()` as it starts; the table has `removed(component)`, and `tickTweens(dt)` moves the runs. Each handler runs as a coroutine, so a native method's body shouldn't call back into Lust code that waits (Lua can't pause across it).

```lua
-- once a tick, on each shard
api.tick(dt)                      -- the objects' updates
api.tickTweens(dt)                -- tweens, then runs whose wait is over
for _, c in ipairs(components) do c:update(dt) end
-- deliver the events: c:handle(name, values) starts each handler as a run
-- when an entity is destroyed, for each Lust component that was on it:
api.removed(c)
```

```rust
game::tick_objects(dt)?;
game::tick_tweens(dt)?;           // tweens, then runs whose wait is over
gun.update(dt)?;
gun.handle("Hit", &[Value::Float(6.0)])?;
gun.removed()?;                   // its entity is gone: its waiting work is cancelled
```

- **The engine does:**
  - keeps calling `tickTweens` / `tick_tweens` once a tick, with the game's own `dt` (0 while paused);
  - calls `removed` for every Lust component it stops for good (its entity destroyed, the component taken off, or stopped after a failure);
  - treats what runs inside `tickTweens` as it treats tweens' done blocks: their changes and sends happen there, and a failure there belongs to whichever component's run it was (the message points at the Lust line);
  - moves any `wait` out of its own Lust files' updates into a started method.
