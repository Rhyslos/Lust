# Notes for engines using Lust

This file is for programs that use Lust as a library: Solis first. When an engine updates its pinned Lust, it reads the section for each version it skips over, newest at the bottom, and does what **The engine does** says. The full rules are in [LUST.md](LUST.md), section 21.

An engine moving from alpha 48 to alpha 59 or later reads everything below.

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
