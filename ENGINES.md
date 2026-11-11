# Notes for engines using Lust

This file is for programs that use Lust as a library: Solis first. When an engine updates its pinned Lust, it reads the section for each version it skips over, newest at the bottom, and does what **The engine does** says. The full rules are in [LUST.md](LUST.md), section 21.

An engine moving from alpha 48 to alpha 66 or later reads everything below.

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
