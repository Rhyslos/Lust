# The Lust v2 Rulebook

Lust is a small, strict language for apps, games and tools. It reads like a calm mix of C# structure and Lua's lightness. Checks are written as questions, states are built in and own their memory, and every file ends with `end`.

This is the single source of truth for Lust v2. Things still being designed are marked **(to design)** and gathered in the last section.

> **What works today (alpha 99):** both outputs. `.lsc` files are checked and turned into Lua. `.lst` files are checked and built into real programs through Rust (`lust build`), and `lust run` runs either kind straight away through Lua. Built so far: types (with vec3, vec2, rotation and colour), questions, `either`, maths (with bitwise operators) and text, arrays, maps, cMaps (with key-makers), states (with options inside options, set with `=`, values carried by each option, and arrows saying which option may follow which: `Locked -> Shut`), classes, objects, `like`, records (plain groups of values, copied wherever they go), methods (with values given by name, as `spawn(x = 3, y = 5)`, and bodies for some of a thing's options: `method fire() in Battle { … }`), hooks with `new hook`, every kind of `loop`, `try`/`error` (with `Problem`, the kinds of failure, as a built-in state), `lua { }` blocks, the built-ins and the standard library (text and array tools, `Maths`, `Random`, `Files`, `Time` and `Json`, which holds vectors, rotations and colours too), with built-in states for their fixed choices (`Day`, `Request` and `Answer`). Waiting pauses only the run that waits (`wait`, `wait { }` until questions are true or a state is one of a `check`'s options, `or after` time limits, `start` to run a call alongside, `start gate = …` and `cancel gate;` to name and stop it, and `gate.state` to see how it's going), and so do asking a website and the computer's own file boxes; work started during a state is cancelled when the state ends, and a program whose runs could only wake each other stops with a message naming them. In a game, event handlers and started work wait across the engine's ticks, on the game's own time. Events in any program (`event`, `on { }` and `send`, with handlers for some of a thing's options: `Hit(…) in Battle { … }`), sharing between files (`pass`/`fetch`/`get`), things ending with their state (`keep`), nodes, assemblies, saves, databases and servers (`schema`/`route`) work too. Windows with `view` are built too: layout, styles, pieces of your own, inputs, text boxes, dropdowns, menus, dialogs, tips, dark mode, grids, splits, tabs, branches, progress bars, drawing, animation, tweens with easing, checkboxes, switches, radio buttons, number boxes, sliders, icons, tables and long lists, right-click menus, rows dragged to new places, dropped files, the mouse on a canvas, scrolling, pictures (SVG too), timers, keyboard shortcuts, fonts of your own, date and colour pickers, second windows, full screen, toasts, notifications and a tray icon, all drawn by Lust itself, sharp at any screen scaling, with input methods for typing any language and a tree for screen readers. Lust can also be part of a Rust program, with `native` methods and `lust build --library`. Not yet: LustQL as a language of its own. Lust v1 lives on the `v1` branch.

---

## 1. What Lust is aiming for

- **Easy to scan.** You should be able to look at a file and find where things happen. Familiar symbols (`==`, `&&`, `return`) where they help reading, Lust's own words where they make things clearer.
- **Strict and friendly.** Mistakes are caught before the program runs, with messages that say what to do.
- **State-driven.** States describe what things are doing, and they own memory: what's made inside a state is freed when the state ends.
- **Two outputs from one language:**

| Output | Files | Used for |
| --- | --- | --- |
| Rust, compiled to real machine code | `.lst` | Programs, apps and servers, on Windows, macOS and Linux |
| LustScript, turned into Lua | `.lsc` | Scripting and modding inside other programs (Garry's Mod, Aventail, Halberd) |

Plus **LustQL** (`.lql`) for stored data: designed in [LUSTQL.md](LUSTQL.md), not built yet.

**Why Rust:** Lust writes readable Rust, and Rust's compiler does the rest: real machine code through LLVM, every desktop platform, and WebAssembly if it's ever wanted. Rust's libraries (crates) give Lust its standard library: files, HTTP, JSON, SQLite, sound, and windows and graphics through winit and wgpu. Lust checks everything itself, so nobody ever sees a Rust error. Programs need Rust installed to build, the way v1 needs .NET.

**Building is quick.** The libraries a program can use (SQLite, windows, pictures, the web) and Lust's own runtime are compiled once per computer, for each Lust and Rust version, and every program is built against them, so `lust build` only compiles the program's own code: about a second. On Windows those libraries are downloaded ready-made (about 150 MB) instead of compiled, by the installer, by `lust update`, or by the first build that needs them; they're checked against the release's checksums, and compiled here only if that fails. Lust builds with one version of Rust (1.97.0), which rustup gets the first time, so the ready-made libraries fit every computer. `lust build main.lst --release` takes longer (about 20 seconds) and compiles the program with its own copy of the runtime, so Rust can optimise the two together: the fastest program, to share. Everyday apps won't notice the difference; heavy number-crunching runs up to about 1.5 times faster.

**The web:** Lust is for servers (`route`, `schema`). Pages in the browser stay HTML, CSS and JavaScript. Lust in the browser through WebAssembly is possible later, but not planned.

There is no .NET in v2, so no `fetch cs:`.

---

## 2. Files

- **Extensions:** `.lst` for programs, `.lsc` for LustScript, `.lql` for LustQL.
- **Every project has a `main.lst`** at its root, where the program starts.
- **Every statement ends with `;`.**
- **Every file ends with `end`** on its last line. Only blank lines and comments may follow.
- **Order inside a file:** imports (`fetch`, `get`), then states and nodes, then classes, objects, assemblies, schemas and methods, then `pass`, then the code that runs.
- **Nothing can be called `me`.** It's the old name for `this` (section 11), which a component still reads it as, so a variable called `me` would quietly become the entity. `FighterReading me = …;` is a mistake that says to pick another name, like `fighter`. The same goes for fields, parameters, loop names and the rest.
- **No dashes in names**, since `-` takes away: `int max-hp` is a mistake that suggests `maxHp`. Files and folders that other files `fetch` can't have dashes either (`fetch "my-parts/x"` is a mistake), so use `myParts` or `my_parts`. A project folder, or a file only ever run on its own, can be called anything.

### Comments

```
-- a comment, the Lua way
--[[ a block comment,
     over several lines ]]
```

`--` is the only way to start a comment: `//` is a mistake that says to write `--` instead.

Because `--` starts a comment, subtracting a negative number needs a space: `a - -b`.

---

## 3. Naming

Lust's house style (Lust doesn't check these two, but everything in this rulebook and in `lust new` follows them):

- **PascalCase for kinds of things:** classes, objects, states, schemas, assemblies, nodes. `Player`, `GameState`, `LootSpawner`.
- **camelCase for everything else:** variables, fields, methods, hooks, files and folders. `currentHp`, `takeDamage()`, `main.lst`, `entities/player.lst`.

What Lust does check:

- So `Player player = new Player;` reads clearly: the capital is the kind, the small letter is the thing.
- Map keys are text, so any text is fine: `"max_hp"`, `"item-id"`.
- ALL_CAPS names are not allowed: `int MAX_HP` suggests `maxHp`.
- **Every word Lust uses is reserved** and can't be a name (see section 22): `int loop = 3;` says `loop is one of Lust's own words, so it can't be a name.` So are the built-ins and the built-in objects.

---

## 4. Types and variables

A variable is declared with its type, its name and a value:

```
int lives = 3;
float speed = 1.5;
string name = "Ada";
bool alive = true;
```

| Type | Holds | Examples |
| --- | --- | --- |
| `int` | whole numbers | `0`, `42`, `-7` |
| `int8` … `uint32` | whole numbers kept in fewer bits, when you want to save memory (see below) | `int8 hp = 3;` |
| `float` | numbers with a point | `1.5`, `0.25` |
| `string` | text, in double quotes | `"Ada"`, `"line\n"` |
| `bool` | yes or no | `true`, `false`, `not true` |
| `vec3` | three numbers: a position, a direction or a speed (section 6) | `[1, 0, 2]` |
| `vec2` | two numbers, x and y: a place or a direction in 2D (section 6) | `[3, 4.5]` |
| `rotation` | a turn in 3D, written as degrees around x, y and z (section 6) | `[0, 0, 90]` |
| `colour` | red, green, blue and see-through, each from 0 to 1 (section 6) | `[0.4, 0.7, 1.0]`, `orange5` |
| `entity` | a thing in a game engine, given by the engine, or `nothing` (section 21) | `this`, `nothing` |
| a class name | one thing made from a class | `Player`, `Weapon` |
| a record's name | a plain group of values, copied wherever it goes (section 10) | `Loadout` |
| `T array` | a list of `T` | `int array` |
| `cMap(...)` | a table of rows, found by their key (the first column) | `cMap(int id, string itemName, int amount)` |
| `T map` | values found by a text key | `int map` |
| a state's name | one of that state's options | `GameState` |
| a schema's name | a data shape for the web | `UserPayload` |
| `hook ...` | a method kept as a value | `hook int(int n)` |

- A variable keeps its type forever.
- **`lock`** makes a variable keep the value it starts with: `lock int maxLives = 3;`. Giving it a new value (`=`, `+=` and the like) is a mistake, caught before anything runs. A field can be locked too, in `has { lock int maxHp = 20; }`. It gets its value when the thing is made (its starting value, or one given in `new Player { maxHp = 30 }`) and keeps it.
  - Locking a thing (`lock Bag bag = new Bag;`) means `bag` is always that same bag, but its fields can still change: `bag.size += 4;` is fine. The same goes for a locked array's or map's items.
  - A class's own `state` changes as it goes, so it can't be locked.
- An `int` fits where a `float` goes. The other way needs `.int()`.
- **Sized numbers, if you want them.** An `int` takes 64 bits. Where a smaller one will do, `int8`, `int16`, `int32`, `uint8`, `uint16` and `uint32` take fewer, in the real program `lust build` makes: `uint8 array tiles` with a million tiles takes 1 MB instead of 8. They're completely optional; `int` works everywhere.

  | Type | Holds |
  | --- | --- |
  | `int8` | -128 to 127 |
  | `int16` | -32768 to 32767 |
  | `int32` | -2147483648 to 2147483647 |
  | `uint8` | 0 to 255 |
  | `uint16` | 0 to 65535 |
  | `uint32` | 0 to 4294967295 |

  - Reading one gives an `int`, so maths, text and comparisons work as they do for any int: `int total = hp + 1000;`, `"hp: {hp}"`. The one difference is `~` on an unsigned one, which flips only its own bits (section 6).
  - Putting a number into one checks it fits. A number written out has to fit before anything runs (`int8 x = 300;` is a mistake). One worked out while running stops the program if it doesn't fit, with what happened: `303 doesn't fit in a uint8, which holds 0 to 255.` It never wraps round quietly.
  - They go in variables, fields, parameters, what a method gives back, arrays, maps, cMaps, hooks, saves and JSON. Not yet in components, events, database tables or native methods: use `int` there.
  - `loop (int c in colours)` goes through a `uint8 array` too.
  - Under `lust run`, numbers are Lua's, so the memory is the same there; the checks are too, so a number that doesn't fit stops the program in both.
- **How big an `int` can be.** A built program holds whole numbers up to 9223372036854775807 (and down to minus that), and stops with a message if a sum goes past it. `lust run` runs programs through Lua, whose numbers only hold whole numbers exactly up to 9007199254740991, so there a bigger one stops the program with a message saying so (when it's written out, printed, or turned into text) instead of quietly showing a wrong number. Build the program to use bigger ones.
- A variable declared inside `{ }` only exists inside those braces.
- A name can't be declared twice while it still exists.
- **`nothing`** means "no thing here yet". Only class-typed values can be `nothing`. Using a field or method on `nothing` stops the program with an error naming the line.

---

## 5. Bools and questions

- A `bool` is `true` or `false`. `not` flips one: `not alive`, `not true`.
- The Lust way to ask about a bool is to ask it straight: `is alive?` and `is not alive?`, rather than `is alive == true?` or `is alive == false?` (which work, but read worse).

### Questions

Lust has no `if`. A check is a question that starts with `is` and ends with `?`:

```
is hp == 0? then {
    print("Out cold");
} or is hp < 5? then {
    print("Barely standing");
} or {
    print("Fine");
}
```

- **`is` only asks questions:** `is hp > 0? then { … }`, `is alive?`, `is hero like Wizard?`, `is saved gameSession?`. It never changes anything. Every value is changed with `=`, states too: `mood = Mood.Busy;`. Writing `mood is Mood.Busy;` is a mistake that shows the `=` way.

### check

`check` goes through a state's options, with one `when` block for each:

```
state Mood = [Calm, Busy, Angry];

check mood {
    when Calm {
        print("relaxed");
    }
    when Busy, Angry {
        print("working");
    }
}
```

- **Every option is covered, always.** A check has no "anything else": leave an option out and Lust says which one is missing. So when a state gets a new option, Lust points at every check that needs a `when` for it. The one exception is a check inside `wait { }` (section 11), which waits until the state is one of the options it names, so it names only those.
- **One `when` can cover several options:** `when Busy, Angry { … }`.
- **Options inside options:** a `when` for an option covers the options inside it, the same way `==` does. A more particular one goes first, and the general one after it catches the rest:

  ```
  check now {
      when ActiveGame.Combat { … }     -- only Combat
      when ActiveGame { … }            -- ActiveGame itself, and Exploration
      when MainMenu { … }
  }
  ```

  The other way round, `when ActiveGame.Combat` could never run, and Lust says so.
- **Write the option on its own,** without the state's name: `when Calm`, not `when Mood.Calm`. The check already knows it's a `Mood`.
- **Bools** work too, with `when true { … }` and `when false { … }`.
- **Numbers and text** use questions instead (`is hp == 0? then { … } or is hp < 5? then { … }`), since there's no list of every value to cover.
- Each `when` has its own `{ }`, so there's nothing to forget at the end of one: the next can't run by accident.

### Comparing

| Write | Means |
| --- | --- |
| `a == b` | the same: equal numbers, text or bools, the very same thing, or for state options, the same option or one inside the other |
| `a != b` | not the same |
| `a === b` | exactly the same: for state options, only that very option |
| `a !== b` | not exactly the same |
| `<  >  <=  >=` | comparisons for numbers |

- For numbers, text and bools, `===` means the same as `==`.
- **Things made from classes** are compared by which thing they are, with either: two `new Hero`s with the same fields are two heroes, so `==` says false. To ask whether two are alike, compare their fields: `a.name == b.name`.
- **State options:** `==` counts the options inside an option. During `ActiveGame.Combat`, `now == GameState.ActiveGame` is true, and so is `GameState.ActiveGame == now`. `===` asks for that very option: `now === GameState.ActiveGame` is false then.
- Comparing different types (`"1" == 1`) is a mistake caught before running. Lust never converts types quietly.
- Join checks with `&&` and `||`. Group with `( )`. (A single `&` or `|` is for the bits of whole numbers, section 6.)

### either

A question that gives back one of two values:

```
string mood = either hp > 10? "happy" or "grumpy";
```

Inside a bigger sum or question, `either` goes in brackets, so it's clear where its answers end:

```
string label = "Mood: " + (either hp > 10? "happy" or "grumpy");
int damage = 2 * (either critical? 3 or 1);
```

Without them (`"Mood: " + either hp > 10? …`), it's a mistake that shows the line with the brackets in.

---

## 6. Maths and text

- `+ - * / %` work on `int` and `float`. `( )` groups. `& | ^ ~ << >>` work on the bits of whole numbers (below).
- `int / int` gives an `int`: `7 / 2` is `3`. Use `7.float() / 2` for `3.5`.
- **Below zero:** `int / int` drops what's after the point, towards zero: `-7 / 2` is `-3`. `%` takes the sign of the number on its left: `-7 % 3` is `-1` and `7 % -3` is `1` (`-7.5 % 2` is `-1.5`). `Maths:round` takes a half away from zero: `Maths:round(2.5)` is `3` and `Maths:round(-2.5)` is `-3`. Built programs and `lust run` agree on all of these.
- **Dividing a float by zero** doesn't stop the program: `1.0 / 0.0` is `infinity`, `-1.0 / 0.0` is `-infinity`, and `0.0 / 0.0` is `not a number`. `Maths:isFinite(x)` asks whether a float is an ordinary number. (Dividing an *int* by zero does stop it.)
- **Very big and very small numbers** can be written with `e`, a power of ten, as Lust prints them: `1e30`, `2.5e-7`, `1E+20`. They're always floats.
- `+=`, `-=`, `*=` and `/=` (and the bitwise `&=`, `|=`, `^=`, `<<=` and `>>=`, below) change a value using itself: `hp -= damage;` is `hp = hp - damage;`. They work on variables, fields (`player.hp += 5;`) and items (`scores[2] += 1;`). `+=` adds to text too (`log += "done. ";`), `+=` and `-=` move a vec3 (`at += [0, 0, 1];`), `*=` and `/=` scale one, and `*=` turns a rotation (`facing *= turn;`).
- `+` joins text: `"Hello, " + name`. It never turns other values into text by itself, so `hp + lives` always adds numbers.
- **There's no power operator.** Use `Maths:power(2, 3)`. `2 ^ 3` is a bitwise operator (below), so it's `1`, not `8`.

### Bits: & | ^ ~ << >>

A whole number is kept as bits, ones and zeros. The bitwise operators work on those bits, which is handy for flags (several yes-or-no settings packed into one number), masks and colours.

| Write | Means | Example |
| --- | --- | --- |
| `a & b` | and: the bits that are on in both | `12 & 10` is `8` |
| `a \| b` | or: the bits that are on in either | `12 \| 10` is `14` |
| `a ^ b` | xor: the bits that are on in one but not the other | `12 ^ 10` is `6` |
| `~a` | not: every bit flipped | `~0` is `-1` |
| `a << n` | the bits moved left `n` places (times 2, `n` times) | `3 << 4` is `48` |
| `a >> n` | the bits moved right `n` places (halved, rounding down, `n` times) | `-8 >> 1` is `-4` |

**Flags.** Give each setting a bit of its own, join them with `|`, and ask about one with `&`:

```
lock int walk = 1;
lock int swim = 2;
lock int fly = 4;

int moves = walk | swim;
is moves & swim != 0? then {
    print("can swim");
}
moves |= fly;      -- now it can fly too
moves &= ~walk;    -- and no longer walk
```

- They work on whole numbers only: `int`, and the sized numbers. On a `bool`, Lust says to use `&&`, `||` or `not`, which join and flip checks. On a `float` they're a mistake (use `.int()` first), and so is mixing an int with a float.
- **The order** is Rust's: `~` (like `-`) first, then `* / %`, then `+ -`, then `<< >>`, then `&`, then `^`, then `|`, then comparisons, then `&&` and `||`. So `moves & swim != 0` means `(moves & swim) != 0`, and `1 | 2 & 3` is `1 | (2 & 3)`, which is `3`. Brackets make it plain when in doubt.
- **Below zero:** an `int` is 64 bits in two's complement, as Rust's `i64` is: `-1` is all ones, so `-1 & 255` is `255` and `~5` is `-6`. `>>` keeps the sign: `-8 >> 1` is `-4`, and `-1 >> 63` is still `-1`.
- **Moving by a number of places** from 0 to 63 works. A negative number, or 64 or more, stops the program: `<< moves bits by 0 to 63 places, but this is 64.` (A number written out is a mistake before anything runs.)
- **`<<` that pushes bits off the top**, so the number would change, stops the program the way `*` does when a number gets too big for an int: `a number got too big for an int.` So `1 << 62` is fine, and `1 << 63` and `3 << 62` stop.
- `&=`, `|=`, `^=`, `<<=` and `>>=` change a value using itself, like `+=`: `moves |= fly;`.
- **Sized numbers** are read as ints, so `&`, `|`, `^`, `<<` and `>>` give an int, checked to fit when it goes back into a sized place (`uint8 x = low << 5;` stops if it comes to 480). `~` on an unsigned one (`uint8`, `uint16`, `uint32`) read straight from where it's kept flips only its own bits: with `uint8 none = 0;`, `~none` is `255`. On a signed one, `~` is the same as on an `int` (`~x` is `-x - 1`), which always fits.
- **Under `lust run`,** whole numbers are Lua's, exact up to 9007199254740991 (section 4), and Lua 5.1 has no bitwise operators, so Lust works them out itself and gets exactly what a built program does for every number in that range. A number outside it, going in or coming out, stops the program with the usual "too big for lust run" message.
- They can't be used on a table's columns in `:where( )`: work the value out first, in a variable.

### Values in text

Put a value in text by writing it in `{ }`:

```
print("Hello, {name}! You have {lives} lives.");
string card = "{name} has {hp} hp and moves at {speed}";
print("{lives + 1} next time, {either lives > 2? "plenty" or "careful"}");
```

- Anything goes in the `{ }`: a variable, a field, maths, a method call, even more text. Each value is shown the way `print` shows it, so there's no `.text()` to write.
- A `{` straight before a name, a bracket (`(` or `[`), `#`, text or a number starts a value, and so does one before `-` or `~` and then a name, number or bracket: `"{1 + lives}"`, `"{-speed}"`. Any other `{` is just a brace, so JSON (`"{\"name\": \"Ada\"}"`) and patterns (`"[0-9]{4}"`) need nothing special. A pattern's count, only numbers and commas like `{4}`, `{1,3}` or `{2,}`, is a brace too, and so is a `{` before a number, `[`, `-` or `~` that's never closed.
- **A value is read as code,** so its mistakes are mistakes, never quiet text: `"{1 + }"` says a value is missing, and `either` after an operator goes in brackets as it does anywhere else: `"{1 + (either lives > 2? 1 or 2)}"`.
- For a plain `{` before a name, write `\{`: `"\{lives} is written like this"`.


| Method | Does |
| --- | --- |
| `.text()` | anything as text |
| `.int()` | a float or text as a whole number |
| `.float()` | an int or text as a float |
| `.isNumber()` | `true` when text is a number |
| `.lines()` | text cut into its lines |

More tools for text, and `Maths` for square roots, rounding and the rest, are in section 19.

### Vectors and rotations

For games and 3D, `vec3` holds three numbers, x, y and z, and `rotation` holds a turn. Both are written as three numbers in `[ ]`. A rotation's are degrees around x, y and z:

```
vec3 pos = [0, 0, 1.5];
vec3 speed = [2, 0, 0];
rotation facing = [0, 0, 90];          -- a quarter turn around z
float dt = 1.0 / 60;

pos = pos + speed * dt;
pos.z = 0;
vec3 ahead = facing * [1, 0, 0];       -- the direction facing points
print(pos);                            -- [0.03333334, 0.0, 0.0]
```

- **Arithmetic:** `+` and `-` between vec3s, `*` and `/` by a number, and `-pos` points the other way. A rotation times a vec3 turns it, and a rotation times a rotation turns by one and then the other.
- **Their parts:** `pos.x`, `pos.y` and `pos.z` are floats, and can be changed: `pos.z = 0;`. A rotation's insides are a quaternion, kept out of the way; `facing:degrees()` gives its degrees around x, y and z as a vec3.
- **Methods:** `pos:length()`, `dir:normalized()` (the same direction, 1 long), `a:dot(b)`, `a:cross(b)`, `a:distance(b)`, `a:lerp(b, 0.25)` (a quarter of the way to b), `a:angle(b)` (the angle between two directions, in degrees, 0 to 180), `facing:inverse()` (the turn back), and `facing:lerp(goal, 0.25)` (a quarter of the smooth turn from one rotation to another, the shortest way round).
- **Making a turn:** `Rotation:toward(dir)` faces `dir`: `Rotation:toward(dir) * [1, 0, 0]` points along it. It turns only left and right and up and down, never rolling, so the turn's up stays as near +z as it can. Straight up or straight down faces up or down with no turn around z. `Rotation:around(axis, degrees)` turns that many degrees around an axis: `Rotation:around([0, 0, 1], 90)` is the same as `[0, 0, 90]`. A direction or axis of `[0, 0, 0]` points nowhere, so either stops the program, as does `a:angle(b)` with one.

```
vec3 toTarget = target - pos;
rotation goal = Rotation:toward(toTarget);
facing = facing:lerp(goal, 0.1);             -- turn a tenth of the way each frame
vec3 ahead = facing * [1, 0, 0];
is ahead:angle(toTarget) < 5? then { print("in sight"); }
```
- **Copied like numbers.** `vec3 b = a;` makes a copy, so changing `b.x` doesn't change `a`. It's the same in arrays, maps and fields.
- **As an engine keeps them.** Each number is a 32-bit float, so a vec3 holds about 7 digits, and `.text()` shows them to 7. The degrees turn in the order z, then x, then y, the same as Solis and glam's `EulerRot::ZXY`: x tilts, y rolls, and z turns like a compass. Z is up.
- **The same everywhere.** Every step is worked out the same way in a built program and through `lust run`, down to the last bit, so a game gives the same numbers in both.
- **Compare with a little room.** 32-bit numbers can't hold every turn exactly: a quarter turn gives `facing * [1, 0, 0]` as `[3.4e-08, 0.9999999, 0.0]`, not `[0, 1, 0]`. So ask whether two vectors are close rather than equal: `is a:distance(b) < 0.0001? then { … }`.
- **JSON, saves and databases** keep vectors and rotations exactly: a vec3 as `[x, y, z]`, and a rotation as its quaternion `[x, y, z, w]` (section 19, Json).

### vec2: two numbers, for 2D

A `vec2` is a vec3 without z: two numbers, x and y, for 2D games. It's written as two numbers in `[ ]`, and works like a vec3:

```
vec2 pos = [3.0, 4.5];
vec2 speed = [2, 0];
pos += speed * dt;
pos.y = 0;
vec2 left = speed:rotated(90);           -- a quarter turn counter-clockwise
print(left, pos:distance([0, 0]));       -- [0.0, 2.0] and how far pos is from [0, 0]
```

- **Arithmetic:** `+` and `-` between vec2s, `*` and `/` by a number, `-pos`, and `+=`, `-=`, `*=` and `/=`.
- **Its parts:** `pos.x` and `pos.y`, which can be changed. It has no `z`.
- **Methods:** `length()`, `normalized()`, `dot(b)`, `distance(b)`, `lerp(b, t)` and `angle(b)` (degrees, 0 to 180), as on a vec3, and:
  - `a:cross(b)`, a float: which way b turns from a. Above 0 is left (counter-clockwise), below 0 is right, and 0 means they point the same way or opposite ways.
  - `v:rotated(degrees)`: turned around `[0, 0]`, counter-clockwise for more than 0. A rotation turns only vec3s. Whole quarter turns (90, 180, -90 …) are exact, so `[1, 0]:rotated(90)` is `[0.0, 1.0]`.
- **Between 2D and 3D:** `pos3.xy` is a vec3's x and y as a vec2, and `pos:withZ(z)` makes a vec3 from a vec2. A vec2 and a vec3 don't mix, so `pos2 + pos3` is a mistake that shows both ways to make them match.
- `.text()` and `print` show `[3.0, 4.5]`. Like a vec3, it's copied like a number, each number is a 32-bit float, the results are the same to the last bit in a built program and through `lust run`, and `==` compares exactly.

### colour: red, green, blue and see-through

A `colour` holds four numbers from 0 to 1: red, green, blue and how solid it is (`a`, 1 for solid, 0 for see-through). It's written as three numbers in `[ ]`, or four with see-through, or as one of the views' colour words (section 17), or from hex text:

```
colour sky = [0.4, 0.7, 1.0];            -- solid
colour glass = [1, 1, 1, 0.5];           -- half see-through
colour warn = orange5;                   -- a colour word
colour moss = Colour:hex("#3a8f2a");     -- or "#3a8f2a80", with see-through
colour dusk = sky:lerp(warn, 0.5);
print(sky, dusk.r);                      -- #66b2ff 0.699999988079071
```

- **Parts go from 0 to 1.** Written as numbers in `[ ]`, a part outside that is a mistake before anything runs (for 0 to 255, divide by 255). Worked out while running, each part is kept between 0 and 1: `[big, 0 - big, 0.5]` is `[1, 0, 0.5]`.
- **Colour words** are the words views use, `white`, `black`, `gray1` to `gray9`, `red`, `blue1` to `blue9` and the rest (section 17), with exactly the red, green and blue the views draw them with. They're colours only where a colour is wanted: `colour c = teal;`, `paint(teal)` for a method taking a colour, `[red, blue]` for a `colour array`, `c == teal`. Anywhere else they aren't names, so a variable can still be called `teal`, and where there is one, it comes first.
- **`Colour:hex(text)`** reads `"#rrggbb"` or `"#rrggbbaa"`, in capitals or not. Text written right there that isn't one is a mistake before anything runs; text worked out while running stops the program, saying what it was. `Colour` with a capital is the object; `colour` is the type.
- **Its parts:** `c.r`, `c.g`, `c.b` and `c.a`, floats that can be changed, and are kept between 0 and 1.
- **Methods:** `c:fade(alpha)` (the same colour with this see-through), `a:lerp(b, t)` (blends all four parts, `t` of the way to b) and `c:hex()` (`"#rrggbb"`, or `"#rrggbbaa"` when it isn't solid).
- `.text()` and `print` show its hex text, `==` compares exactly, and there's no arithmetic on colours. Each part is a 32-bit float, the same everywhere.
- **In a view**, a colour value goes wherever a colour word does after `background`, `color` and `border`: `[background sky]`. It's drawn exactly as it is, in dark mode too, and a background shows its see-through.
- **JSON, saves and databases** keep them exactly: a vec2 as `[x, y]`, and a colour as its hex text when that's exact, or its four numbers when it isn't (section 19, Json).

---

## 7. Arrays, cMaps and maps

### Arrays

```
int array scores = [10, 20, 30];
```

- Counted from 0: `scores[0]`.
- `#scores` is how many items. `#"Hello"` is how many letters.
- `scores:inject(40);` adds to the end. `scores:remove(0);` removes by position.

### cMaps

A table whose rows are found by their key. The first column is always the key, and you choose its name and whether it's an `int` or a `string`:

```
cMap(int id, string itemName, int amount) bag = [
    [1, "Sword", 1],
    [2, "Apple", 5]
];

cMap(string name, int price, int stock) shop = [
    ["tea", 2, 10],
    ["cake", 5, 3]
];
```

- `bag[1]` is the row with key 1, `shop["tea"]` the row with key "tea". `shop["tea"].price` reads a column, `shop["tea"].price = 3;` changes one.
- `shop:inject(["bread", 3, 8]);` adds a row. `shop:remove("cake");` and `shop:has("cake")`. `#shop` is how many rows.
- A key can't be changed. To give a row a new key, remove it and add it again.
- `shop:sortBy(price);` puts the rows in order by a column (rows with the same value keep their order), and `shop:reverse();` turns them around.
- `loop (string name, int price, int stock in shop) { … }` goes through the rows in the order they were added.

**Keys Lust makes.** A new row can leave its key out, and Lust makes one:

- **An `int` key counts up** from the biggest key so far: `bag:inject(["Map", 1]);` gets 3.
- **A key-maker** makes it instead. It's a method or hook that takes nothing and gives back a key, written after the key column like a starting value:

```
method int rollId() {
    return Random[1000, 9999];
}

cMap(int id = rollId, string item, int amount) chest = [];
chest:inject(["Ruby", 1]);    -- gets a key from rollId
```

- If the key-maker gives a key that's taken, Lust asks it again. After 100 taken keys in a row it stops with an error.
- A key-maker goes where a cMap is made: on a variable or a field, not in a method's parameters.
- **A `string` key with no key-maker** has to be given every time.

A cMap with one column after its key does the same job as a map, but maps are quicker to write for that, so both exist.

### Maps

```
int map prices = ["apple" 3, "seed" 1];
```

- `prices["apple"]` reads, `prices["pear"] = 5;` adds or changes, `prices:has("pear")` checks.
- `prices:remove("seed");` removes a key.

---

## 8. States and memory

A state is a list of options, and options can hold options of their own:

```
state GameState = [
    MainMenu,
    ActiveGame [
        Exploration,
        Combat
    ]
];
```

- A state is set with `=`, like any value. A variable holding a state option changes like any variable: `now = GameState.ActiveGame.Combat;`.
- A class or object can have a state of its own, written `state = GameState.MainMenu;` in its `has`, always with the state's name in front of the option. Inside its methods, `state = GameState.ActiveGame.Combat;` changes it, and `state` reads it. Outside a class or object there's no `state` to change; use a variable.
- **Only a thing's own methods change its state** (its class's, or a class `like` it). From outside, `player.state` can be read, and given a starting value in `new Player { state = … }`, but `player.state = …;` is a mistake: call a method that changes it, like `player:startGame();`. So a class's methods are the one way in and out of its states, and what they do on the way (start the music, drop the shield) can't be skipped. A method can change another thing of its own class too: `other.state = state;`.
- `is` is only for questions (section 5). The old way of setting a state, `now is GameState.MainMenu;` or `state is GameState.MainMenu;`, is a mistake: `a state is set with =, so write state = GameState.MainMenu;`.
- A question about an option includes everything inside it: during `Combat`, `is state == GameState.ActiveGame?` is yes, whichever side the option is written on. `===` asks for that very option (section 5).

### Values on options

Each option can carry values of its own, written in `{ }` after its name. Code then reads them instead of a long `check` over every option:

```
state Pose = [
    Idle   { clip = Clip.Idle,  speed = 0.0 },
    Run    { clip = Clip.Run,   speed = 6.5 },
    Battle { clip = Clip.Fight, speed = 4.0 } [
        Melee  { clip = Clip.Swing },              -- speed 4.0 comes from Battle
        Ranged { clip = Clip.Aim, speed = 2.5 },
    ],
];

sprite:animate(pose.clip);
float s = Pose.Run.speed;
```

- **Lust works out the shape.** Every option has the same names, and each name keeps one type on every option. A whole number where another option has a `float` counts as a `float` (`Idle { speed = 0 }` beside `Run { speed = 6.5 }`). A name with two types (`speed = 4.0` on one option, `speed = "fast"` on another) is a mistake that names both options.
- **Options inside options take their outer one's values,** and can change any of them: above, `Battle.Melee.speed` is `4.0` and `Battle.Ranged.speed` is `2.5`. Every option ends up with every value, the outer ones too, so one missing is a mistake that names the option and the value: `Pose.Run has no speed, but other options of Pose do.`
- **Values go before inner options:** `Battle { … } [ … ]`.
- **Read anywhere an option is:** on a variable or field holding one (`pose.speed`, `hero.state.speed`, or `state.speed` in a thing's own methods), on an option written out (`Pose.Run.speed`), in a loop over the options (`loop (Pose p in Pose) { print(p.speed); }`), and in engines. `Pose.speed`, without an option, is a mistake that says to name one.
- **Fixed:** the values are part of the state, so `pose.speed = 7.0;` and `Pose.Run.speed = 7.0;` are mistakes. To change one, change what's written in the state.
- **What they hold:** values that never change, written out right there: numbers (negative ones too), text without `{ }` in it, bools, another state's option (or this state's: `next = Level.Hard`), vectors, colours and records (section 10). Two numbers in `[ ]` are a `vec2` and three a `vec3`; a colour is a colour word (`red`), `Colour:hex("#3366ff")`, or three or four numbers beside one (`[0, 0, 1, 0.5]`). A record is `new Loadout { … }` with each value it's given written out too, and the starting values of those it isn't given must be written out in the record (`int grenades = 2;`, not `Random[1, 3]`). Variables, sums, calls and `nothing` are mistakes. Reading one gives a copy, as reading a record always does: `Pose.Run.kit.grenades = 3;` is a mistake, since the values are fixed.
- **Names start with a small letter,** and can't be one of the words that turn a value with `( )` (`text`, `int`, `float`, `isNumber`, `lines`, `upper`, `lower` and `trim`), since `pose.text()` already means the option's name as text. Options start with a capital, so a value is never confused with an inner option.
- **An option with values is the same option.** `==`, `===`, `check`, `:find`, `print`, `.text()`, saves, JSON and native methods all see just the option, as before.
- **Fast:** reading `pose.speed` looks the option up in a table made once, in both outputs; `Pose.Run.speed`, written out, is just its value.

### Allowed moves between options

A state can say which option may follow which, with arrows, so a move the game's rules don't allow is caught:

```
state Door = [Locked -> Shut, Shut -> [Open, Locked], Open -> Shut];

state Phase = [
    Calm { speed = 1.0 } -> Battle,
    Battle { speed = 4.0 } [
        Melee -> Battle.Ranged,
        Ranged -> [Battle.Melee, Calm],
    ] -> Dead,
    Dead { speed = 0.0 },
];
```

- **`Option -> Next`,** or `Option -> [A, B]` for several. As soon as any option of a state has an arrow, only the moves written are allowed. An option with no arrow (and no outer option with one) is an **end**: nothing follows it, so above, a `Phase` that's `Dead` stays `Dead`. A state with no arrows works exactly as before: every move is allowed, and nothing is checked.
- **Written last:** an option's name, then its values `{ }`, then its inner options `[ ]`, then its arrow: `Battle { … } [ … ] -> Dead`. An arrow anywhere else is a mistake that shows this order.
- **Options inside options** have their arrows inside their outer option's `[ ]`, written after them as any option's are. **An arrow names options by their path from the state**, wherever it's written: inside `Battle`, `Melee -> Battle.Ranged`, not `-> Ranged` (which is a mistake that suggests `Battle.Ranged`).
- **An arrow from an outer option covers its inner ones:** above, `Battle.Melee` can go to `Battle.Ranged` (its own arrow) and to `Dead` (Battle's). An inner option's arrow adds to its outer one's; it never takes moves away.
- **An arrow to an outer option** lets the thing go to that option or any option inside it, since `Battle.Melee` is a `Battle` too: above, `Calm` can go to `Battle`, `Battle.Melee` or `Battle.Ranged`.
- **Moving between inner options** of the same outer one is a move like any other, so it needs an arrow: `Melee -> Battle.Ranged`.
- **Staying where it is is always allowed:** setting the option it's in already changes nothing, so it needs no arrow (and an arrow to itself is a mistake).
- **Checked when a value changes,** wherever it's kept: a thing's own `state` (`state = Door.Open;`, or `other.state = …;`), a variable, a field, a record's value, an array's item, and a map's value for a key it already has. A move that isn't allowed stops with `Problem.WrongState` (section 15), where the change was written, and `try` catches it like any other:

  ```
  a Door can't go from Locked to Open. From Locked it can go to Shut.
  a Phase can't go from Dead to Calm. Nothing follows Dead.
  ```

  The value is left as it was.
- **First values are free:** a thing's starting state can be any option (in its `has`, or `new House { state = Door.Open }`), and so can a variable's when it's made, a map's value for a new key, and an item added to an array. So are values that come from outside a program's own code: a save or JSON being loaded, and a game engine setting a component's state (from its editor, say).
- **Screens:** an input bound to a field (section 17) changes it as code does, so a dropdown or radio buttons bound to a state with arrows are a mistake: they'd offer every option, and some choices would be refused. Bind them to a string holding the options that can follow (`dropdown pick in next { … }`), and change the state in their block.
- **Waits, events, `check` and the rest don't change:** arrows only say which moves are allowed.
- **Mistakes:** an arrow to an option the state doesn't have (with a suggestion when there's one close), an option listed twice after one arrow, or listed with an option it's inside (`-> [Battle, Battle.Melee]`: `Battle` covers it), an arrow to itself, the state's name in front of an option (`-> Door.Shut`: write `-> Shut`), `-> [ ]` with nothing in it, two arrows on one option, an arrow before an option's values or inner options, and a dropdown or radio buttons bound to a state with arrows.
- **Fast:** the moves each option allows are worked out once, when the program is checked. A change is one look-up in a table made once, in both outputs, and a state without arrows costs nothing at all.

### Things made in a state end with it

An object with a state of its own (`state = …;` in its `has`) owns what its methods make. Anything made with `new` in one of its methods belongs to the option the object is in at that moment, and **ends when the object leaves that option**:

```
class Hero {
    has {
        state = Phase.Calm;
        Item array bag = [];
    }

    method grab(string name) {
        Item sword = new Item { name = name };
        keep sword in bag;      -- without keep, the sword would end when Calm does
    }
}
```

- Once something has ended, its fields are let go, and using it stops the program with a message that says why: `this was made during Battle.Melee, and ended with it.` It isn't `nothing`, and it's no longer `like` anything.
- **`keep thing in place;`** holds onto it: it no longer belongs to the state, and it goes into the place. The place can be an array (it's added to the end), a field or a variable.
- **`keep thing;`** holds onto it without putting it anywhere.
- Every change of the object's state ends what belongs to the options it leaves, however it's written: `state = …;`, `other.state = …;` for another of the same class, or a value from a variable (`state = next;`). `lust run` and built programs do exactly the same.
- Options inside options count separately: something made during `Battle.Melee` ends when the object moves on to `Battle.Ranged`.
- Starting values in `has`, and anything made by code outside a class's methods, belong to no state.
- **Work started with `start`** in a method belongs to the state too, and is cancelled when it ends (section 11, States cancel their work).
- If one ended thing owned things of its own, those end with it.
- Lust catches the simple slips before running: storing something just made in a field or array without `keep` is a mistake.

---

## 9. node and assembly

### node: an object that opens and closes

A node is like an object, but it can be opened and closed again and again: a room in a game, a level, a menu. Closing it ends everything it made.

```
node Cave {
    has {
        Item chest = new Item { name = "chest" };
        int visits = 0;
    }

    method onOpen() {
        print("You enter the cave.");
    }

    method onClose() {
        print("You leave the cave.");
    }
}

Cave:open();
Cave.visits = Cave.visits + 1;
Cave:close();
```

- A node starts closed. `Cave:open();` starts it fresh, with its starting values, then runs `onOpen` if it has one.
- `Cave:close();` runs `onClose`, then ends everything the node made: its starting values and anything its methods made. Using one afterwards says `this was made in Cave, and ended when it closed.` Use `keep` to hold onto something.
- While it's closed, using its fields or methods says to open it first. `Cave:isOpen()` asks.
- Opening an open node, or closing a closed one, is an error.
- A node can't be `like` a class. `onOpen` and `onClose` take nothing and give nothing back.

### assembly: a factory

An assembly makes ready-filled things. It holds one recipe per class:

```
assembly Loot {
    create Item {
        name = "Potion";
        value = 50;
    }
    create Sword {
        name = "Old sword";
        sharpness = Random[1, 5];
    }
}

Item drop = Loot:create(Item);
Sword blade = Loot:create(Sword);
```

- A recipe sets fields, and its values are worked out each time something is made, so `Random` gives each sword its own sharpness.
- What an assembly makes in a method of an object with a state, or of a node, belongs to it, just like `new`.

---

## 10. Classes, objects and like

```
state Mood = [Happy, Grumpy];

class Dog {
    has {
        string name = "Rex";
        int hp = 10;
        state = Mood.Happy;
    }

    method bark() {
        print(name + " says woof!");
    }
}

Dog pet = new Dog { name = "Bo" };
pet:bark();
```

- **Fields are written like variables**, all inside one `has { }` block. The same style is used everywhere: `has`, `new`, `create` and `schema`.
- `new` makes a thing. The braces set fields; anything not mentioned keeps its starting value.
- `.` reads a field, `:` calls a method.
- An **object** is one ready-made thing, used by its name: `object Game { … }`, `Game:start();`.
- **`like`** builds on another class: `class Wizard like Player { … }`. A Wizard fits anywhere a Player does, and its methods replace Player's of the same name.
- Ask what something really is: `is hero like Wizard?`.

### Records: plain groups of values

A record groups a few simple values under one name, so they go everywhere together: into a variable, a field, a method, an event, even a game engine's component. Unlike a thing made from a class, a record is **copied** whenever it's given, the way a number is:

```
state FighterClass = [Scout, Heavy];
state Gun = [M4a1, Glock, Shotgun];

record Loadout {
    FighterClass role;
    Gun primary;
    Gun secondary;
    int grenades = 2;
}

Loadout kit = new Loadout { role = FighterClass.Scout, primary = Gun.M4a1, secondary = Gun.Glock };
Loadout spare = kit;
spare.grenades = 3;
print(kit.grenades, spare.grenades);       -- 2 3
send MatchStarted(kit) to player;
```

- **Its values are written like a `has` block's fields,** straight inside the record's `{ }`: no `has`, and no methods, `on` block or view (a record has nothing to do; make it a class to give it methods). It can sit at the top of any file, and be passed and fetched like a class: `pass Loadout;`, `fetch "gear":Loadout;`.
- **It holds only simple values:** numbers (`int` and `float`), text, bools, states' options, vectors, rotations, colours, entities and other records. Not things made from classes (they're shared, not copied), arrays, maps, cMaps or hooks. A record can't hold itself, even through others (`Loadout` holding a `Kit` that holds a `Loadout` is a mistake): it would never end.
- **Starting values** work as in `has`. A value with no starting value is given in every `new`, by name (`new Loadout { role = …, primary = …, secondary = … }`); leaving one out is a mistake that names it.
- **Copied when given:** setting a variable or a field, passing it to a method, giving it back with `return`, sending it in an event, putting it in an array, a map or a cMap, and a hook keeping it as it was when it was made each make a copy. Changing one copy never changes another. A loop's current item and each handler of an event get their own copy too.
- **Changing a value in place** works through any path that leads to where it's kept: `kit.grenades = 3;`, `hero.kit.grenades -= 1;`, `kits[0].primary = Gun.Glock;`, `squad.lead.role = FighterClass.Heavy;`. A copy that isn't kept anywhere can't be changed (`newKit().grenades = 3;` is a mistake: keep it in a variable first). A locked record (`lock Loadout kit = …;`) and a loop's current item can't be changed at all.
- **`==` and `!=` compare every value,** records inside records too. A state's option is compared as that very option, as `===` asks of states. `===` on records is a mistake: a record is only its values, so there's no "very same one" to ask about.
- **Never nothing:** a record always has its values, so `Loadout kit = nothing;` is a mistake, as is a field of a record's type with no starting value (`Loadout kit = new Loadout { … };`).
- **Printing it, and `.text()`,** give its name and its values in order, text in quotes, as a list's items are: `Loadout { role = Scout, primary = M4a1, secondary = Glock, grenades = 2 }`. A record with no values is `Empty {}`. The same in both outputs.
- **Named values** (section 11) and **values on states' options** (section 8) take records like any value.
- **JSON and saves** hold records as objects of their values (`{"role":"Scout",…,"grenades":2}`); reading one, a value with a starting value may be left out, the others must be there, and `null` is a mistake (a record is never nothing). A record holding an entity can't be JSON, as an entity can't. Database tables don't hold records: give each value a column.
- Things made from a class can be `like` one another, kept with `keep`, and end with a state; records can't and don't need to. A tween can't move a value inside a record: keep it in a field of its own.
- **In an engine** (section 21), a component's fields and the events the engine carries can hold records, once the engine says it keeps them.
- **Speed:** in Lua a record is a plain table, copied as it's read from where it's kept (one made right there, or given back by a method, isn't copied again); in a built program it's a Rust struct, copied like any value.

---

## 11. Methods and hooks

Everything callable is a `method`, inside a class or object or on its own:

```
method int doubleIt(int value) {
    return value * 2;
}

method greet(string who) {
    print("Hello, " + who);
}
```

- The return type comes first, and is left out when nothing comes back.
- `return` gives back a value and ends the method. A plain `return;` leaves early.
- Parameters are typed: `(int a, string b)`.

### Named values

A call or a `send` can name each value, so a long list reads clearly and two values can't be swapped by mistake:

```
spawn(x = 3, y = 5);
spawn(y = 5, x = 3);                -- the same call
body:move(speed = 2.0, x = 1.0, y = 4.0);
send Wants(jump = true, crouch = false, fire = true) to body;
```

- **The names are the method's or event's own:** its parameters' names (`method spawn(int x, int y)`), or what `event Wants(bool jump, …)` says it carries. A wrong name is a mistake that suggests the nearest one (`spawn has no value called xx. Did you mean x?`), and a missing one says which (`spawn needs y too, but it isn't named here.`).
- **All or none:** a call names every value or none of them, never some.
- **Any order,** and each name once.
- **Works for** the program's own methods (with `:` or bare inside a class), methods on their own, `send` (to everything or `to` one thing), the engine's `entity` methods, and hooks whose type names their values (`hook(int amount, string why) onHurt;` is called `body:onHurt(why = "fell", amount = 3);`). A hook whose type leaves out the names (`hook int(int) f`) takes its values in order.
- **Built-in methods** (`print`, `Maths:sqrt`, `name:contains`, an array's `:inject` and the rest) take their values in order, without names: naming one is a mistake.
- **Nothing changes on the receiving side,** and nothing costs anything while the program runs: Lust puts the values in the method's order before it writes the program out, so a named call is exactly the same call written in order.
- **Values are worked out in the method's order.** Usually that can't change anything. But when two values swap places and one does work of its own (calls a method, makes something with `new`, picks a `Random` number, or could stop the program, like `a[i]` or `/`) while the other could see what it did (it does work too, or reads a field or a variable at the top of the file), the order would matter, so it's a mistake: `spawn takes x before y, and y's value does work …`. Write those two in the method's order, or work one out into a variable first. Values that can't change (numbers, text, options, the method's own variables) can go anywhere.

### Hooks: methods kept as values

`hook` is the type of a method kept in a variable, field, array or map. The names in a hook's type are the names its body uses.

```
hook int(int n) triple = new hook {
    return n * 3;
};

hook int(int) f = doubleIt;            -- an existing method works too

class Shop {
    has {
        hook(string item, int price) onSell = nothing;
    }
}

Shop shop = new Shop;
shop.onSell = new hook {
    print("Sold " + item + " for " + price.text());
};
```

- `new hook { }` is just the body. What it takes and gives back comes from where it's going.
- A `new hook` can read the variables around it. It keeps their values as they were when the hook was made, and it can't change them (keep a value in an object's field to share changes: objects are shared, not copied).
- A hook kept in a variable is called like a method on its own: `triple(5)`. One kept in a field is called with `:`, like a method: `shop:onSell("bread", 3);`, or `onSell("bread", 3);` inside the class.
- Calling a hook that's `nothing` stops the program, so check first: `is shop.onSell !== nothing?`.

### Events: telling things what happened

An event says that something happened, and what comes with it. Any class, object, node or assembly can listen for events in an `on { }` block, written like `has { }`, and `send` hands one out:

```
event Saved(string file);
event Closed();

object AutoBackup {
    has {
        int copies = 0;
    }

    on {
        Saved(string file) {
            Files:write(file + ".bak", Files:read(file));
            copies += 1;
        }
        Closed() {
            print("Backed up {copies} times.");
        }
    }
}

send Saved("notes.txt");            -- every thing that handles Saved hears it
send Saved("notes.txt") to editor;  -- only editor hears it
```

- **`event Name(…);`** says what an event carries, in order. It can carry anything a variable can hold, things of your own classes too: `event Picked(Item it);`. What's carried is shared, like any thing handed over: a handler that changes `it.name` changes the item.
- **`on { }`** holds a thing's handlers, one for each event it hears: the event's name, what it carries in ( ) (the names can be its own), and a block. A thing has one `on` block, and each event appears in it once, unless its entries are for different options (Events in some options, below). Inside a handler, fields and methods are used as in a method.
- **Values can be named,** in any order: `send Saved(file = "notes.txt");` (Named values, above).
- **`send Name(…);`** reaches every thing that exists right now and handles the event. **`send Name(…) to thing;`** reaches only that one: a thing of a class, an object, a node or an assembly's piece. Sending to a thing whose class doesn't handle the event is a mistake (`Editor doesn't handle Saved. Give it on { Saved(string file) { … } }`), unless a class `like` it does: then the things of that class hear it, and the rest don't. An event no one handles does nothing.
- **`like`:** a class hears the events the class it's like handles, and its own handler for an event replaces that class's, as methods do.
- **Nothing runs inside `send`.** Events wait in a queue, and are heard **once the code running now is done**: after each statement at the top of the file (so a method that sends three events has them heard after it returns), after each thing a window does (a click, a key, a timer, a tween's done block), after a run waiting goes on, and as a `wait` starts. Each handler that hears an event is a run of its own (below), so one that waits pauses only itself, and the rest of the queue doesn't wait for it. So `send Saved(f); print("sent");` prints `sent` before any handler runs.
- **In order:** events are delivered in the order they were sent. An event sent while one is being delivered (Saved sends Closed) joins the end of the same queue. Each event reaches the things in the order they were made, objects first, in the order they're written, since they're made as the program starts. `lust run` and built programs do exactly the same.
- **Things that end stop hearing.** A thing made in a state (section 8) that ended with it no longer hears events, nor does a closed node, until it opens again. A thing with an `on` block hears events from when it's made until it ends, **even when nothing holds it any more**: `send` can always reach it. So a thing made over and over that handles events should belong to a state that ends it, or be an object.
- Sending to a thing that's `nothing`, or that has ended, stops the program, as calling its methods would.
- Events can also go to the entities of a game engine, and components hear them there (section 21).

### Events in some options: in

A thing with a state of its own (section 8) can say which of its options a handler is for. The same event can then have several **entries**, each for different options:

```
state Phase = [Calm, Battle [Melee, Ranged], Dead];

class Hero {
    has {
        float hp = 10;
        state = Phase.Calm;
    }

    on {
        Hit(entity by, float damage) in Battle {
            hp -= damage;
        }
        Hit(entity by, float damage) in Calm {
            state = Phase.Battle;
        }
    }
}
```

- **`in Option`** goes after an entry's values, naming an option of the thing's own state, written on its own as in a `check`'s `when` (`in Battle`, not `in Phase.Battle`). Options inside options are written with their path: `in Battle.Melee`.
- **Options inside options:** `in Battle` covers `Battle.Melee` and `Battle.Ranged` too. When two entries cover an option, the most precise one wins, whatever order they're written in: with `in Battle` and `in Battle.Ranged`, a hit during `Battle.Ranged` runs only the second.
- **An entry without `in`** covers every option no other entry covers.
- **An event that comes in an option no entry covers is ignored:** nothing runs, and it isn't a mistake. So a Hero with only the two entries above shrugs off a `Hit` once it's `Dead`.
- **The entry is chosen when the event is heard,** by the option the thing is in then, not when it was sent. Events wait in a queue until the code running now is done (above), so a method that sends `Hit` and then changes the hero's state has the hit heard in the new option.
- **Every entry for an event takes the same values,** with the same names and types: `Hit(entity by, float damage)` each time. Each entry is still a run of its own when it's heard, and can wait.
- **Mistakes:** two entries for the same option, or two without `in`; `in` on a thing with no state of its own (`Plain has no state of its own, so an entry can't be in Calm.`); and `in` naming an option its state doesn't have, with a suggestion when there's one close (`Phase has no option called Batle. Did you mean Battle?`).
- **`like`:** a class with entries of its own for an event replaces all of its parent's entries for that event, as a handler without `in` would; it never mixes its entries with its parent's. For events it has no entries for, it hears them as its parent does, choosing by its own current option.
- **Everywhere `on` is:** classes, objects, nodes, windows, assemblies' pieces and components (whose `has` can hold `state = …;` too). It's quick: choosing the entry is one look-up by the current option, from a table made once, in both outputs.

### Methods in some options: in

A thing with a state of its own (section 8) can say which of its options a method is for. The same method can then have several **bodies**, each for different options, the way an event has entries (above):

```
state Phase = [Calm, Battle [Melee, Ranged], Dead];

class Hero {
    has {
        int ammo = 3;
        state = Phase.Calm;
    }

    method fire() in Battle {
        ammo -= 1;
    }

    method fire() in Battle.Ranged {
        print("aim first");
    }

    method int power(int base) in Calm {
        return base;
    }

    method int power(int base) {
        return base * 2;
    }
}

Hero hero = new Hero;
hero:fire();             -- stops: fire is only for Battle, but this Hero is Calm.
```

- **`in Option`** goes after the method's values, naming an option of the thing's own state, written on its own as in a `check`'s `when` (`in Battle`, not `in Phase.Battle`). Options inside options are written with their path: `in Battle.Melee`.
- **Options inside options:** `in Battle` covers `Battle.Melee` and `Battle.Ranged` too. When two bodies cover an option, the most precise one wins, whatever order they're written in.
- **A body without `in`** covers every option no other body covers.
- **Calling a method while no body covers the thing's option** is an error of a kind of its own, `Problem.WrongState` (section 15): `fire is only for Battle, but this Hero is Calm.` The error is where the method was called, and `try` catches it like any other. The message names the thing by its class (`this Hero`, or `this Ghost` for a Ghost `like` Hero), or by its own name for an object or a node (`Gate`): a call doesn't carry the variable's name, which keeps every call as quick as before. To have no error at all, give the method a body without `in`.
- **The body is chosen when the method is called,** by the option the thing is in then.
- **Every body takes the same values and gives back the same,** with the same names and types: `int power(int base)` each time. Called with named values, `start`ed, or waiting, a method with bodies works like any other.
- **Mistakes:** two bodies for the same option, or two without `in`; bodies that differ in their values or what they give back; `in` on a thing with no state of its own (`Plain has no state of its own, so fire can't be in Calm.`) or on a method on its own; `in` naming an option its state doesn't have, with a suggestion when there's one close; a `native` method with `in`; and a component's `update` with bodies only in some options, since it runs every tick whatever the option (give it a body without `in` too).
- **`like`:** a class with bodies of its own for a method (or a plain method of that name) replaces all of its parent's bodies for it; it never mixes its bodies with its parent's. A method it doesn't write is its parent's, chosen by its own current option.
- **Everywhere methods are:** classes, objects, nodes, windows, assemblies' pieces and components. It's quick: choosing the body is one look-up by the current option, from a table made once, in both outputs.

### Waiting: wait and start

`wait seconds;` lets that many seconds go by, and `start` runs a method call alongside. Only the piece of work that waits pauses, never the window or the rest of the program:

```
method openGate() {
    print("creak");
    wait 2;                      -- only this piece of work pauses
    print("gate open");
}

start openGate();                -- runs alongside: the next line runs at once
print("Gate opening");

openGate();                      -- waits here until the gate is open
print("Gate open");
```

- **A run** is a piece of work that waits on its own: the program's own code from the top, one click's, key's or timer's code in a window, one event's handler (an `on` block's), a tween's done block, or a call started with `start`.
- **`wait seconds;` pauses its run,** and everything else carries on: the window draws and answers clicks, other runs go on, and tweens and timers move. `wait 0;` lets the other runs that are due now go first. A wait below 0 seconds stops with `Problem.OutOfRange`.
- **`start call;`** runs the call as a run of its own. It goes straight away, until its first `wait` (or its end), and then the line after `start` runs. It gives nothing back, and what it's given is worked out as it starts. Only a call can be started: `start gate;` is a mistake. `start` is a word only before a call, so a variable can still be called `start`.
- **`start gate = openGate();`** does the same, and gives the work a name, to stop it with `cancel` or see how it's going (below).
- **A call waits for what it calls.** A method that waits makes whoever calls it wait too, as it would for a long loop. Nothing is marked: a method that waits is called like any other, and can give back a value (`int n = slowAdd(1, 2);`).
- **In order:** runs due at the same moment go on in the order they started waiting. A tween's step due then goes before them. `lust run` and built programs run them in exactly the same order.
- **A plain program ends** once its own code and every run it started are done.
- **In a window,** each callback is a run, so a button whose code waits doesn't freeze the window: the other buttons still work. A wait in the code that draws the view (outside any callback) stops the program with a message saying where to put it instead.
- **Failures:** a `try` inside a run catches that run's own failures, waits and all. A failure no `try` in its own run catches stops the program, even if the code that started the run was inside a `try`.
- **Lust's own clock.** Runs wait on the same clock as tweens and timers. In a view's tests, `wait 2.5` in the test moves every run's clock along (section 17); `lust test` runs waits without taking the real time, so tests stay quick and give the same results every time.
- **Failures stop the program,** so started work never ends in a failure of its own: it's done, cancelled, or the whole program has stopped.
- **In an engine** (a library or components), the engine's ticks move the runs, and `update` can't wait: see In an engine, below.

### Naming and stopping work: start name = and cancel

`start gate = openGate();` gives the work a name, and `cancel gate;` stops it wherever it is:

```
start gate = openGate();         -- gate names this work
wait 0.5;
cancel gate;                     -- openGate() stops: its wait never finishes

wait {
    check gate.state {           -- how it went
        when Done { print("Open!"); }
        when Cancelled { print("Gave up."); }
    }
}
```

- **The name is a variable,** declared where the `start` is, and it lives as long as a variable there would: inside `{ }`, only until the `}`. It always names that work, so `gate = …;` is a mistake, and so is a second `start gate = …;` while the first `gate` still exists. A `new hook` can use it, as it can any variable around it.
- **Only variables, for now.** A name can't be a field, go in an array or a map, be handed to a method or be given back, as started work has no type to write there. To stop work from another method, start it during a state instead, and change the state (below).
- **What a name can do:** `cancel gate;`, `gate.state`, and `keep gate;` (below). Printing it, putting it in text or comparing it is a mistake that says to use its state.
- **`cancel gate;`** stops the work wherever it is: the `wait` it's in never finishes, and none of its code runs again. It never fails:
  - Cancelling work that's already done, or already cancelled, does nothing.
  - **Work it started carries on,** each a run of its own, unless that work has a name and is cancelled too.
  - **A `try` around the cancelled wait does nothing:** a cancel isn't a failure, so no `error` block runs, and nothing after it either.
  - **Cancelling itself** (through a hook that holds its name, say) stops the work at once: the line after `cancel` doesn't run.
  - **Work that's going but not waiting** (one that started the work going now, or that's waiting for an event's handler to finish) is marked cancelled at once, and stops as soon as it would go on.
  - `cancel` on anything else is a mistake. `cancel` is a word only before a name, so a variable can still be called `cancel`.

### How work is going: Run

A name's `state` is one of the built-in state `Run`'s options, and it changes by itself as the work goes on:

| Option | Means |
| --- | --- |
| `Run.Waiting` | paused in a `wait` (or a slow built-in) |
| `Run.Going` | running now (or it started work that's running now) |
| `Run.Done` | finished |
| `Run.Cancelled` | stopped by `cancel`, or because its state ended |

- It's waited for like any state: `wait { is gate.state == Run.Done? }`, or a `check` inside `wait { }`.
- Only the work and `cancel` change it, so `gate.state = …;` is a mistake, as for any thing's state.
- Work that starts and finishes without waiting is `Run.Done` by the line after the `start`.

### States cancel their work

Work started during a state is cancelled when that state ends, like anything else made in it (section 8):

```
class Hero {
    has {
        state = Phase.Calm;
    }

    method fight() {
        state = Phase.Battle;
        start drums();           -- belongs to Battle: cancelled when Battle ends
        start song = music();
        keep song;               -- kept: it carries on
    }

    method rest() {
        state = Phase.Calm;      -- drums() stops here
    }
}
```

- **Which state:** work started in a method of a thing with a state of its own belongs to the option the thing is in at the `start`, exactly as something made with `new` there would. Work started in a node's method belongs to the node, and is cancelled when it closes.
- **The thing it runs on:** work also belongs to the thing whose method it runs, so it's cancelled when that thing ends: `start guard:patrol();` (with the guard in a variable or field) or `start patrol();` in the guard's own method stops if the guard was made in a state that's ended, and so does an event's handler that was waiting when its thing ended. So no run wakes up inside a thing that's gone.
- **Work that ends its own state** (its first part changes the state it was started in) is cancelled there and then, and stops at once.
- **`keep gate;`** holds onto named work: it no longer belongs to any state, and carries on. Work started outside a class or node's methods belongs to no state.

### Waiting in a circle

When every run that's waiting is in a `wait { }` that only other code could end, and nothing else could ever run, nothing can change: Lust stops the program with a message naming each run and where it waits:

```
gate waits at line 22, and guard waits at line 27, each for something only the other could change, so neither can ever move.
```

- **Only what's certain is noticed.** A run counts as stuck when its `wait { }` has no time limit and its questions (and its `check`'s state) call nothing: no method, hook, built-in, `Random`, save or database, any of which could change by itself. Lust stops only when every waiting run is stuck, no tween is moving, no window is open, and each has looked again since anything last ran.
- So **a run waiting seconds, for a slow built-in or with `or after`,** or one whose questions call a method, keeps the program going: its time can come, and the code after it could change things.
- **Names:** named work is called by its name, other started work by its call (`openGate()`), an event's handler `the Saved handler`, and the program's own code `the program's own code`. The lines are where each `wait { }` is. The program stops as on any failure, and no `try` catches it.

### Slow built-ins pause only their run

Some built-ins take a while: asking a website (`Web:get`, `Web:post`) and the computer's own file boxes (`Files:chooseOpen`, `chooseSave`, `chooseFolder`). They're written as ordinary calls, with no `wait` in front, and while one is out only the run that called it pauses, as if it were waiting:

```
method showWeather() {
    string page = Web:get("https://api.example.com/weather?city=Oslo");   -- only this run pauses
    print(page);
}

start showWeather();
print("asking…");                -- runs straight away, while the request is out
```

- **While it's out,** the window draws and answers clicks, other runs go on, and tweens and timers move. The work happens on a thread of its own, and the run goes on once the answer has come.
- **It pauses at least as `wait 0;` does,** so the other runs due now go first, even when the answer is quick. After that, Lust looks for the answer each time the clock moves a 1/60 s step (each frame in a window), as for `wait { }`, and the run goes on there and then.
- **What comes back is the same** as when it held everything up: the same text, and on a failure the same `Problem` (`NoAnswer`, `NotFound`, `NotAllowed`, `BadText`), caught by a `try` in that run.
- **A method that calls one waits for it,** as for any wait, so the code that calls it waits too, and can be `start`ed to go alongside instead.
- **In the program's own code** it pauses that code, so runs started earlier go on while the request is out.
- **Where a run can't pause, it holds everything up,** as it always did: in the code that draws a view (outside any callback), in a route's method answering a request (`Web:request`, `Web:serve`), and in an engine's update or other code the engine calls straight into (below).
- **In tests** (`lust test`), the answer takes no time on Lust's clock: the run pauses for the runs due now, then waits for the answer before the clock moves on. So a test that asks a website prints the same every time, however long the answer takes. In a view's tests, a file box's answer (`choose` in the test) is seen once the test's clock moves: `click Open…` then `wait 0`.
- **The rest stay as they are:** reading and writing files, databases, `ask(…)` and `Web:serve` hold everything up while they work. Files and databases are quick, and runs reading and writing the same file or table at once would see each other's half-done work; `ask` types into the console, which other runs would be printing over; and a server that pauses only its run (so `cancel` can stop it) needs both outputs' servers to look for requests on Lust's clock instead of waiting for them, which is still to come (WAITING.md).

### Waiting for things to be true: wait { }

`wait { … }` waits until things are true. It holds questions, each `is …?` on its own line, and every one of them has to be true. Lust waits for things to change, not for code to finish: a run that loads something sets a field or a state when it's done, and the code that needs it waits for that.

```
start openGate();
wait {                           -- until the gate is open
    is gate.state == Door.Open?
}
print("Through the gate");

start map:load();                -- each sets its own state when it's finished
start sounds:load();
wait {                           -- until both are: every question has to be true
    is map.state == Load.Done?
    is sounds.state == Load.Done?
}

wait {
    is door.state == Door.Open?
} or after 5 {                   -- a time limit: this runs if 5 seconds go by first
    print("The door never opened.");
}
```

- **Only questions:** a question asks about anything a question can, fields, states, other things' fields, and methods that give back a bool (`is gate:isOpen()?`), and needs a yes-or-no answer. Anything else in the block, a `print`, a call, a variable, is a mistake that says the block holds only questions; put that code after the wait. A `wait { }` with no questions is a mistake too. The one other thing it can hold is a `check` (below).
- **If they're all true straight away, nothing waits.** Events sent before the wait are heard first, as for `wait seconds;`.
- **Lust looks again** each time the clock moves a 1/60 s step: once a frame in a window, and in a plain program on the same clock as `wait seconds;` and tweens, so something another run sets at 0.5 seconds is seen by 0.5 seconds and one step at the latest, and the code after the wait goes on there and then. Runs waiting at the same moment look in the order they started waiting.
- **Looking runs the questions,** so a method in a question is called at every look. Lust has no `=` or `+=` inside a question, but a method can still change things: keep the methods a wait calls to asking.
- **Where:** it pauses only its run, as `wait seconds;` does, so it works in the program's own code, in methods started with `start`, in event handlers, in a tween's done block and in a window's callbacks. In the code that draws a view (outside any callback) it stops the program with a message, as `wait seconds;` does. In an engine it looks again once a tick (below).
- **A wait no one can end** (nothing will ever make the questions true) waits for ever, so give it a time limit when that can happen. When nothing else could ever run, Lust notices and stops (Waiting in a circle, above).

### Waiting for one of several options: check inside wait { }

A `check` inside a `wait { }` waits until a state is one of its `when`s' options. When it is, that `when`'s block runs, once, and the wait is over:

```
wait {                           -- until the boss is in one of these, then that one runs
    check boss.state {
        when Dead { print("Won!"); }
        when Fleeing { chase(boss); }
    }
} or after 60 {
    print("Too slow.");
}
```

- **It names only the options to wait for.** An ordinary `check` covers every option, but this one waits until the state is one of these, so it needn't: above, the wait goes on while the boss is `Calm`. Everywhere else, a `check` still has to cover every option.
- **Options inside options** work as in any `check` (section 5): `when Battle` matches `Battle.Melee` too, a more particular `when` goes first, and the first `when` that matches is the one that runs.
- **With questions beside it,** they have to be true too before the `when` runs: `wait { is door.open? check boss.state { … } }` waits until the door is open and the boss is in one of the options, in either order.
- **One `check` per `wait { }`.** Two would leave it unclear which block runs, or whether both do, so a second one is a mistake. Wait for anything else with a question beside it: `is gate.state == Door.Open?`.
- **The `when`'s block runs in the waiting run,** straight after the wait ends and on the option the last look saw, so it's ordinary code: it can wait itself, and `return`, `stop` and `skip` work as anywhere else.
- **If it already matches,** nothing waits, and its `when` runs straight away. **If time runs out first** (`or after`), no `when` runs, only the time limit's block.
- **Only states:** a check over a bool or anything else in a `wait { }` is a mistake that shows the question to ask instead (`wait { is door.open? }`). A check with no `when`, or a `when` naming an option the state doesn't have, is a mistake too.

### Time limits: or after

**`… or after seconds { … }`** puts a time limit on a wait: `wait { … } or after 5 { … }`, or `wait 10 or after 2 { … }`. If time runs out first, the block runs; if the wait is over first, it doesn't. Either way, the code after it carries on.

- The block is ordinary code, and `stop`, `skip` and `return` in it work as anywhere else.
- When the questions come true at the very moment time runs out, the wait wins and the block doesn't run. On `wait seconds;`, the limit only matters when it's shorter: `wait 10 or after 2 { … }` waits 2 seconds and runs the block. A limit that's never reached because both are written as numbers (`wait 2 or after 3 { … }`) is a mistake, as its block could never run; with variables, a longer limit just never runs out.
- The limit is a number of seconds, 0 or more: anything else is a mistake, and a limit below 0 (from a variable) stops with `Problem.OutOfRange`.
- `after` is a word only straight after a wait's `or`, so it can still be a name. A wait's `or` always comes with `after`: `wait { … } or { … }` is a mistake that shows how to write it.

### In an engine

In a game (a library or components), everything above works, with the engine's ticks as the clock:

```
component Gate {
    has {
        bool opening = false;
        bool flash = false;
    }

    method update(float dt) {
        is Lever.pulled && not opening? then {
            opening = true;
            start open();                -- update goes on; open() waits on its own
        }
    }

    method open() {
        play("creak");
        wait 2;                          -- two seconds of game time, across ticks
        spawnEnemies();
    }

    on {
        Hit(entity by, float damage) {
            flash = true;
            wait 0.2;                    -- only this handler pauses
            flash = false;
        }
    }
}
```

- **Runs:** each event handler (a component's `on { }`, and Lust's own events), each `start`, and each tween's done block is a run. It goes straight away until its first wait, and goes on in a later tick, while the engine keeps calling every component's `update`.
- **`update` can't wait.** It runs every tick, and a second one would start while the first still waited, so the waiting goes in a method of its own, started from `update` (`start open();`), as a game engine's coroutines are. A `wait` written in a component's `update` is a mistake that says so, and one reached through a method `update` calls stops with the same advice. So does a wait in anything else the engine calls straight into: an object's `update`, a method the engine calls, and the program's own code as it starts. A `wait { }` there whose questions are already true goes straight on.
- **The game's own time:** `wait 2;` is over once two seconds of the engine's ticks (their `dt`) have gone by, at the first tick that reaches it. A paused game pauses waits too, and a slowed one slows them. `or after` counts the same time. `wait 0;` goes on in the next tick.
- **`wait { }` and a `check` in a wait look again once a tick,** even while the game is paused.
- **In order:** each tick, after the objects' updates, tweens move, then the runs whose wait is over go on, in the order they're due (of two due together, the one that started waiting first), then the components' updates. A run that starts waiting during a tick goes on in a later one.
- **Removed components:** when the engine removes a component (its entity is destroyed, or the component taken off), its handlers still waiting and the work started in its methods are cancelled, wherever they wait, what it made in its states ends, and its tweens stop. Work started in a component's method belongs to the component, whatever method it runs.
- **Waiting in a circle** is never noticed in an engine: the engine keeps ticking, and an update could change anything a wait asks about.
- **Asking a website** (in a game built as a Rust library) pauses only its run, looking for the answer once a tick; in code the engine calls straight into it holds the call up, as before.

How an engine drives this is in section 21, Runs in an engine.

---

## 12. Loops

`loop` does every kind of repeating:

```
loop { … }                                      -- forever, until stop
loop 3 { … }                                    -- three times
loop (hp > 0) { … }                             -- while true
loop (int score in scores) { … }                -- each item of an array
loop (int id, string item, int qty in bag) { … } -- each row of a cMap
loop (string key, int value in prices) { … }    -- each key of a map
loop (GameState s in GameState) { … }           -- each option of a state
loop (int i = 0; i < 10; i += 1) { … }          -- counting
```

- `stop;` leaves the loop. `skip;` jumps to the next round.

**Counting** has three parts, split by `;`: a new variable to count with, the check that keeps the loop going, and the change after each round.

```
loop (int i = 0; i < #names; i += 1) {
    print(i, names[i]);          -- each name with its place
}
loop (int i = 10; i > 0; i -= 2) { … }          -- down, by twos
```

- The counter belongs to its loop: it's gone after the `}`, so the next loop can use `i` again.
- The check is asked before every round, so a check that's false at the start means no rounds.
- The change still happens after `skip;`. Changing the counter inside the loop counts too.
- The counter can't be `lock`ed, since it changes every round.

---

## 13. Sharing between files

Everything in a file is private until it's passed:

```
pass Player;
pass GameState, Mood;
```

Another file brings things in with `fetch` (classes, objects, states) and `get` (methods). The path is in quotes, from the project's folder (the one with `main.lst` in it), with `/` between folders and without `.lst`:

```
fetch "system/gameState":GameState;
fetch "entities/player":[Player, Party];
get "helpers/mathUtils":clamp;
```

- Only what's brought in can be used. Lust says where something lives if a file forgets to bring it in.
- Two files can each have a private thing with the same name. They never mix.
- Files can bring things in from each other both ways.
- **Only the file being run runs its code.** The code at the bottom of a fetched file is left out, so any file can be run on its own to try it out, from any folder: `lust run entities/player.lst` from the project's folder, or `lust run player.lst` from inside `entities`. Lust finds the project's folder (the nearest one up with a `main.lst`) either way.
- Mistakes and errors name the file they're in: `entities/player.lst:12  …`.
- LustScript works the same way, with `.lsc` files. A LustScript program turns into one Lua file.

---

## 14. Saves and databases

### Saves

```
int currentDay = 1;
save gameSession = [Farm, player, currentDay];

is saved gameSession? then {
    load gameSession;
}

save gameSession;
```

- A save lists what it holds once, at the top of the file: objects, and variables declared above it. Then `save` and `load` do the rest.
- The file is `gameSession.save`, as readable JSON, in the folder the program runs from: the folder you're in when you start it (`lust run sub/game.lst` from the project's folder writes `./gameSession.save`), or the program's own folder when it's double-clicked. Database files (`in "shop.db"`) are found the same way.
- Loading is strict: everything is read first, and if the save doesn't fit the program, nothing changes and the error says why, like `gameSession's save doesn't fit this program, so nothing was loaded: it has no currentDay.`
- `saved gameSession` is true when there's a save to load.
- Vectors, rotations and colours are saved as they're written in JSON (section 19, Json), so loading gives back exactly the same value, to the last bit. Records are saved as objects of their values (section 10).
- Hooks aren't saved, and nodes can't be: save what's in them.
- A save that holds variables can only be saved and loaded from the top of the file, where those variables are seen.

### Databases

A database is a file of tables, kept by SQLite. Each table is written like a cMap, and is used like one:

```
database Shop in "shop.db" {
    cMap(string name, float price, int stock) items;
    cMap(int id, string who, string item) orders;
}

Shop.items:inject(["tea", 2.5, 10]);
Shop.items["tea"].stock = 9;            -- changes the database straight away
print(Shop.items["tea"].price);
print(#Shop.items);
```

- The first column is the key, as in a cMap. An `int` key counts up when a row leaves it out, from the biggest key the table has ever had, so a removed row's key isn't given to a new row. (Tables made before alpha 17 give one more than the biggest key they have now.) A `string` key has to be given.
- Columns hold numbers, text, bools, state options, vec3s, vec2s, rotations and colours. The key is an int or a string.
- **Vectors, rotations and colours** are kept as the same text `Json:write` gives (`[1.0,2.5,-0.1]`, or `#ff922b` for a colour), in a column SQLite sees as text, so they read back exactly. Since it's text, `:where` and `:orderBy` can't use them: `:where(pos == [0, 0, 0])` is a mistake. Loop through the rows and check each one instead.
- `:inject`, `:has(key)`, `:remove(key)` and `[key]` work as they do on cMaps. A row found with `[key]` reads and changes the database itself.
- If a table is already in the file with other columns, Lust says so instead of guessing.

**Queries** pick rows. They're written in Lust, and Lust turns them into SQL:

```
loop (string name, float price, int stock in Shop.items:where(price <= 3.0 && stock > 0)) {
    print(name);
}

cMap(string name, float price, int stock) cheapest = Shop.items:orderBy(price):first(3);
Shop.items:where(stock == 0):remove();
```

| | Gives |
| --- | --- |
| `:where(condition)` | the rows where it's true. The columns can be used by name |
| `:orderBy(column)` | the rows in that column's order, smallest first |
| `:reverse()` | the same rows the other way round |
| `:first(n)` | only the first n |
| `:count()`, or `#` | how many |
| `:remove()` | removes the rows the query picked |

- Queries can be chained: `Shop.items:where(stock > 0):orderBy(price):first(5)`.
- Inside `:where`, the columns work with maths, comparisons, `&&`, `||`, `not`, `#`, and `:contains`, `:startsWith` and `:endsWith`. Anything that doesn't use the columns, like a variable or a method call, is worked out first.
- Looping through a table or a query goes through its rows. Putting one in a cMap variable copies its rows into the cMap, so changing the cMap doesn't change the database.
- Programs with a database are built against Lust's libraries, which include SQLite. They're compiled the first time a program needs them on a computer, which takes a few minutes; after that, builds are quick (section 1).
- In LustScript, databases work under `lust run`. Other programs, like Garry's Mod, don't have them yet.

---

## 15. Errors

```
try {
    Files:write("config.json", data);
} error (string message) {
    print("That didn't work: " + message);
}
```

- `try` runs the block. If something goes wrong inside it, the program carries on in `error` instead of stopping.
- Without `try`, an error stops the program with the file, line and a plain message.

### What kind of problem

Fixed choices are states, not text, and the kinds of failure are too. **`Problem` is a built-in state**, and `error` can take one before the message, so code can tell kinds of failure apart without comparing text:

```
try {
    string text = Files:read("save.json");
    Game g = Json:read(text);
} error (Problem p, string message) {
    is p == Problem.NotFound? then {
        startNewGame();
    } or is p == Problem.BadJson? then {
        print("The save is damaged: {message}");
    } or {
        print(message);
    }
}
```

- **Two shapes:** `error (string message)`, for code that only shows the message, and `error (Problem p, string message)`. The `Problem` comes first, then the message; any other shape is a mistake that shows both.
- **`p` is a state option** like any other: `is p == Problem.NotFound?`, `check p { … }`, `print(p)` (which shows `NotFound`), or kept in a variable (`Problem last = p;`). A misspelt option is a mistake before running that suggests the nearest one: `Problem.NotFund` says `Did you mean NotFound?`.
- **`check` covers every option,** as always, so it suits code that does something for each kind. To pick out one or two, `is` is shorter (above).
- **Each failure has exactly one kind,** the same in `lust run` and built programs. A kind Lust doesn't name more closely is `Other`.
- **An error nobody catches** stops the program with its message, as before. The kind isn't shown there.
- `Problem` is one of Lust's names, so a class, state or anything else of your own can't be called `Problem` (section 22).

These are `Problem`'s options:

| Option | What went wrong |
| --- | --- |
| `NotFound` | something asked for isn't there: a file or folder (`Files:read`, `:list`, `:remove`, or writing into a folder that doesn't exist), a map's key, a cMap's or database's row (or a row that's been removed), a save that hasn't been made yet (`load`), or a web page that answered 404 |
| `NotAllowed` | the computer doesn't allow it: a file or folder that can't be read or written, or a website that answered 401 or 403 |
| `AlreadyThere` | a key that's taken: a cMap's or database's row with the same key, or a folder where a file of that name is |
| `BadJson` | text that isn't JSON, JSON that doesn't fit where it's going (`Json:read`), or a value JSON can't hold (`Json:write` of infinity) |
| `BadText` | text that isn't what it should be: a number (`.int()`, `.float()`), a date, a colour for `Colour:hex`, a pattern (`:matches` and the rest) or a web address |
| `BadSave` | a save that's damaged, or doesn't fit this program, so nothing was loaded |
| `OutOfRange` | a number outside what's allowed: an array's item that isn't there (or one from an empty array), a slice past the end, a number too big for its type, maths with no answer (the square root of a number below zero, `asin(2.0)`), `Random` or `clamp` with its low above its high, a wait or tween below 0 seconds |
| `DividedByZero` | a whole number divided by zero, or its remainder (`%`) |
| `Nothing` | something that's nothing was used, or a thing that has ended with its state, or whose node is closed |
| `NoAnswer` | a website couldn't be reached, or answered with an error other than 401, 403 or 404 |
| `Database` | the database itself failed: it couldn't be opened, or SQLite couldn't do what was asked |
| `Stopped` | `Web:fail` stopped a route's method |
| `WrongState` | a method was called while its thing is in an option none of the method's bodies is for (Methods in some options, section 11), or a state's value moved to an option its arrows don't allow (Allowed moves between options, section 8) |
| `Other` | anything else, like a method calling itself forever |

---

## 16. LustScript (.lsc)

LustScript is Lust turned into Lua, for running inside programs that already speak Lua. It's the same language with the same rules and checks. Only the output differs.

- `fetch lua;` allows raw Lua inside a `lua { }` block, for anything the library doesn't cover:

```
fetch lua;

method registerHook() {
    lua {
        hook.Add("PlayerSpawn", "lust_spawn", function(ply)
            ply:ChatPrint("Welcome!")
        end)
    };
}
```

- Lua has no separate whole numbers, so `int` is checked strictly by Lust and becomes a plain number in Lua.

---

## 17. Screens: view { }

A screen is described, not built step by step. `view` says what the window should look like right now, given the data. After anything happens (a click, a key, a timer), Lust looks at the view again and updates the window to match. You change data, never the screen.

### What's there

An assembly with a `view` opens a window with `App:open();`, through `lust run` and as a built program. `lust new name app` starts a project with a window and a piece in `ui/` to build on.

- **Pieces:** `column`, `row`, `stack`, `space`, `text` and `button`. A button's `{ }` runs when it's clicked, and the window redraws afterwards. Like a hook, it keeps the values of the variables around it as they were when it was drawn, so a button made in a loop sees that time round's values.
- **Scrolling, pictures and timers:** `scroll`, `image` and `every` (see below).
- **Inputs:** `input`, `textarea` (several lines), `checkbox`, `switch`, `radio`, `number` and `slider`, each bound to a field (see below), with the mouse, the keyboard and the clipboard.
- **Pickers:** `datePicker` for a date, with a month's calendar, and `colourPicker` for a colour, with a square of colours, a hue strip and swatches (see below).
- **Icons:** `icon Icon.Trash`, from a built-in set of about 150, on their own or in buttons (see below).
- **Your own pieces:** any assembly with a view, used inside another view as `TodoRow { label = task };`, from the same file or fetched from files in `ui/` folders (see below).
- **Styles and layout:** every word in the list below, `hover:[ … ]` included, plus named styles (`style chip = [pad 6, round];`). Text wraps onto more lines when it doesn't fit.
- **Questions and loops** inside a view.
- **The window's settings:** `title`, `width`, `height`, `minWidth`, `minHeight`, `background`, `font`, `dark`, `fullscreen` and `visible`. They're worked out each time the view is drawn, so a title can show the data: `view [title "To do (" + (#todos).text() + ")"]`.
- **`disabled`** greys out a piece and stops it being clicked, typed in or reached with Tab. It can depend on the data: `button "Save" [disabled name == ""] { … }`.
- **Undo and redo** in inputs: Ctrl+Z, and Ctrl+Shift+Z or Ctrl+Y (Cmd on a Mac). A run of typing undoes in one go.
- **Enter and shortcuts:** `input newTodo enter { add(); }` runs a block when Enter is pressed in the input, and `key "ctrl+s" { save(); }` is a keyboard shortcut (see below).
- **Fonts:** `font "fonts/Lora.ttf"` draws a piece's words in a font file of your own, and on the view's `[ ]` it's the whole window's font.
- **Feedback:** hover looks fade in and out, and a button looks pressed in while it's held down.
- **Laying out bigger apps:** `grid` for cards in columns that rewrap as the window changes size, `split` for panels with a line between them to drag, `tabs`, `branch` for parts that fold open and shut (trees), and `progress` bars (see below).
- **Drawing and motion:** `canvas` with `box`, `circle` and `line` placed by `x` and `y`, for charts, maps and games, and `animate` for pieces that glide to where they go and fade in (see below).
- **Things that show over the view:** `dropdown` lists to choose from, `menu`s of `item`s, `dialog`s that pop up while a bool is true, and `tip "…"` for a note when the mouse rests on a piece (see below).
- **Tables and long lists:** `table` shows rows in columns that sort, pick and resize, and `list` is a scrolling column of rows; both stay quick with 100,000 rows (see below).
- **The mouse:** a right-click `menu { }` after any piece, `[reorder]` for a list's rows to be dragged to new places, `drop` for files dropped on the window, and `press`, `drag`, `release` and `hover` on a canvas (see below).
- **More windows:** `window` for a second window that opens while a bool is true, `fullscreen` for full screen, `visible` to hide the main window, and `close { }` for what its X does (see below).
- **Telling people things:** `toast "Saved";` for a small message in the window's corner, `App:notify(…)` for the computer's own notification, and `tray` for an icon in the system tray, with a menu (see below). `App:quit()` ends the program from anywhere.
- **For everyone:** windows are sharp at any screen scaling (125%, 150%, 200%, and moving between screens), take typing through input methods for Chinese, Japanese, Korean and other languages, and describe every piece to screen readers like Narrator and NVDA, with `label "…"` to name a piece that has no words (see below).

```
assembly App {
    has {
        int count = 0;
    }

    view [title "Counter", width 360, height 200] {
        column [center, gap 12, pad 24] {
            text "Clicked " + count.text() + " times" [size 24, weight 600];
            button "Add one" [blue, round] {
                count = count + 1;
            }
        }
    }
}

App:open();
end
```

**Testing a view without a screen.** With `LUST_VIEW_TEST=1` set, no window opens. The program reads commands, one per line, from what's typed in (or a `.in` file in tests):

| | |
| --- | --- |
| `dump` | prints every piece with its place and size |
| `click Add one` | clicks the button showing that text, or a row of an open list (`click Cupboard`, `click Save`), or a table's or list's row by words it shows (`click Ada`) |
| `click Age` | clicks a table's header, sorting by it |
| `dblclick Ada` | double-clicks the row of a table or list showing those words, opening it |
| `rightclick Notes`, `rightclick Ada`, `rightclick 120 80` | right-clicks the piece or row showing those words, or a place, opening its right-click menu (a row is picked first); the menu shows in `dump` as an open list, and `click Copy` chooses from it |
| `drag Ada Cleo`, `drag 2 5` | drags the row showing Ada onto the row showing Cleo, so it takes Cleo's place; with numbers, the second row on the screen of the first list that reorders onto the fifth. Words with spaces go in quotes: `drag "Row 5" "Row 9"`. While a row is held, `dump` ends with where it would land |
| `drop C:/a.txt "C:/My files/b.png"` | drops files on the window (paths with spaces in quotes) |
| `dragover` | holds files over the window without dropping them, showing its soft edge; `dump` ends with `files held over the window` |
| `scroll table 1 2000`, `scroll list 1 -300` | scrolls the first table (or list) down 2000 (up with a minus) |
| `open menu File`, `open dropdown 1` | clicks a menu or the first dropdown, opening its list |
| `click Weapons`, `click Stats` | opens or shuts a branch, or shows a tab, by its name |
| `press 184 200`, `move 260 200`, `release 260 200` | drags the line between a split's panels, like any drag |
| `choose cat.png`, `choose cancel` | the answer the next `Files:chooseOpen`, `chooseSave` or `chooseFolder` box gives (the code after the box goes on once the clock moves: `wait 0`) |
| `hover Delete` | puts the mouse on the button showing that text, and shows its tip straight away |
| `focus input 1` | gives the first input focus (also `textarea 1`, `checkbox 2`, `switch 1`, `radio 1`, `number 1`, `slider 1`, `button 3`, `table 1`, `list 1`) |
| `type Hello` | types into the input that has focus |
| `key ctrl+left` | presses a key: `backspace`, `delete`, `left`, `right`, `home`, `end`, `up`, `down`, `pageup`, `pagedown`, `tab`, `enter`, `space`, `escape`, with `shift+`, `ctrl+` or `alt+` in front, and `ctrl+a`, `ctrl+c`, `ctrl+x`, `ctrl+v`; `shift+f10` or `menu` (the Menu key) opens the right-click menu of what has focus; any key a shortcut uses works too (`key ctrl+s`, `key f1`) |
| `check 1` | clicks the first checkbox |
| `flip 1` | clicks the first switch |
| `click Medium`, `click Sound` | clicks a radio button, or a checkbox or switch, by its words |
| `wheel 1 up`, `wheel 1 down` | turns the mouse wheel one notch over the first number box (it changes only while the box has focus) |
| `slide 1 70` | drags the first slider to 70 |
| `open datePicker 1`, `open colourPicker 2` | opens the first date picker's calendar (clicking its icon), or the second colour picker |
| `today 2026-11-13` | the date a date picker takes as today, so a test gives the same every day |
| `pickday 14` | clicks the 14th of the month the open calendar shows; `click Today`, `click ‹`, `click ›`, `click November 2026` (its title), `click Feb` and `click 2030` click its other parts |
| `pickcolour 0.5 0.2`, `hue 0.6`, `alpha 0.5` | clicks the open colour picker's square (half way across, a fifth of the way down), its hue strip, or its alpha strip, by how far along each is from 0 to 1 |
| `swatch 3` | clicks the open colour picker's third swatch |
| `press 40 20`, `move 90 20`, `release 90 20`, `dblclick 40 20` | the mouse, at a place in the window (a canvas's `press`, `drag`, `release` and `hover` run from these; `move` with nothing held is hovering) |
| `scroll 1 100` | scrolls the first scroll down 100 (up with a minus) |
| `wait 2.5` | lets 2.5 seconds go by on the test's own clock, running the timers, tween steps and waiting runs that come due (in that order, when due together) |
| `size 800 600` | resizes the window |
| `snapshot shot.png` | draws the window into a picture |
| `in Settings click Save`, `in "My settings" dump` | sends a command to the second window with that title (in quotes when it has spaces) instead of the main one |
| `close`, `in Settings close` | clicks the window's X: the main window runs its `close { }` (or ends), and a second window's bool becomes false |
| `tray click`, `tray item Quit` | clicks the tray icon, or chooses one of its items |
| `scale 1.25` | draws the snapshots after it as on a screen scaled to 125% (from 0.5 to 4; 2 unless told), and `dump` says so in its heading |
| `ime "にほん"`, `ime commit "日本"`, `ime cancel` | an input method's text being put together at the caret (`dump` shows it as `composing "にほん"`), the text it finished, typed in, or giving up |
| `dump access` | prints the tree a screen reader is given: each piece's role, name, value and states |
| `act click Save`, `act set Name Ada`, `act up Volume`, `act focus "row 5"` | does what a screen reader would ask of the first piece with that name (or role: `act up slider`, `act click "row 2"`): `click`, `focus`, `show` (scroll into view), `open`, `close`, `set` a value, or a step `up` or `down` |

`lust test` sets it for every test, so a view's tests compare what `dump` prints. A table or list dumps only the rows on the screen, after a line saying how many it has and which show: `table 100000 rows, showing 41–51`. `dump` shows each open window under a heading of its own, `window "Settings" 300x200`, with `(fullscreen)` or `(hidden)` after the main window's when it is; the toasts showing, as `toast "Saved"` in a `layer "toast"`; and the tray, as `tray "Notes" (2 items)`. `App:notify("Title", "Body")` prints `notify: Title — Body` instead of showing anything. `wait` moves the toasts along too, and every window's timers. In tests, letters Lust's font doesn't have are drawn as boxes rather than taken from the computer's own fonts, so a test measures the same on every computer.

### Pieces are assemblies

A piece of screen is an `assembly` with a `view`. Each piece goes in its own file, in folders that mirror the app: `ui/components/navbar.lst`, `ui/kanban/column.lst`.

```
-- ui/todoRow.lst
assembly TodoRow {
    has {
        string label = "";
        bool done = false;
        hook() onToggle = nothing;
        hook() onRemove = nothing;
    }

    view {
        row [gap 8, pad 8, round, hover:[background gray1]] {
            checkbox done { onToggle(); }

            is done? then {
                text label [muted, strike];
            } or {
                text label;
            }

            space;
            button "✕" [subtle, small] { onRemove(); }
        }
    }
}

pass TodoRow;
end
```

```
-- main.lst
fetch "ui/todoRow":TodoRow;

style card = [pad 16, round, shadow, background white];

assembly App {
    has {
        cMap(int id, string task, bool done) todos = [];
        string newTodo = "";
    }

    method add() {
        is newTodo.trim() != ""? then {
            todos:inject([newTodo.trim(), false]);
            newTodo = "";
        }
    }

    view [title "To do", width 420, height 600] {
        column [gap 12, pad 24] {
            text "To do" [size 24, weight 600];

            row [gap 8] {
                input newTodo [width fill, placeholder "Something to do…"];
                button "Add" [blue, round] { add(); }
            }

            column [card, gap 4] {
                loop (int id, string task, bool done in todos) {
                    TodoRow {
                        label = task,
                        done = done,
                        onToggle = new hook { todos[id].done = not todos[id].done; },
                        onRemove = new hook { todos:remove(id); }
                    };
                }

                is #todos == 0? then {
                    text "Nothing to do!" [muted, center];
                }
            }

            text (#todos).text() + " things" [muted, small];
        }
    }
}

App:open();
end
```

- **`App:open();`** opens the top piece as a window. The view's own `[ ]` holds the window's settings: `title`, `width`, `height`.
- **Using a piece:** `TodoRow { label = task };`, the same shape as `new TodoRow { … }` without `new`. The first time, Lust makes it and keeps it. Later redraws reuse it.
- **Values given in `{ }` are given again on every redraw**, so a piece always shows the current data. Fields not given are the piece's own memory, kept between redraws.
- **The data lives in one place.** A piece shows what it's given, and asks for changes through hooks.
- **Which piece is which.** In a loop over a map or cMap, each row keeps its own piece, found by the row's key, so removing the first row doesn't hand its memory to the next one. Anywhere else, pieces are matched by their place: the third `TodoRow` written in the same spot is the same one next time.
- When a piece leaves the screen, it's forgotten, and so is its memory. If it shows up again, it starts fresh.
- A piece can be given `[styles]` like any other: `TodoRow [width 300] { label = task };`.
- The `[ ]` after a piece's own `view` is only for windows (`title`, `width`, …), so a piece that's only ever used inside others doesn't need one.
- Assemblies can still have `create` recipes next to a view. Nodes stay for game rooms and levels.

### Inputs are bound to fields

`input newTodo;` shows the field and changes it as you type. `checkbox done;` and `slider volume [from 0, to 100];` work the same way. A block runs after each change, once the field already holds the new value: `input search { results = find(search); }`.

```
assembly Settings {
    has {
        string name = "";
        bool dark = false;
        int volume = 50;
    }

    view [title "Settings"] {
        column [gap 12, pad 20] {
            input name [placeholder "Your name", width fill];
            row [gap 8] {
                checkbox dark;
                text "Dark mode";
            }
            row [gap 8] {
                slider volume [from 0, to 100, step 5];
                text volume.text();
            }
        }
    }
}
```

- **What they change.** An `input` changes a string, a `checkbox` or `switch` a bool, a `radio` a string or a state (like a dropdown), a `slider` or `number` an int or a float, a `datePicker` a string holding a date, and a `colourPicker` a colour. It's bound to a field (or a field of something, like `todos[id].task`), not a variable of the view, because the view's variables are made again each time it's drawn.
- **Styles of their own:** `placeholder "…"` is what an empty input shows. A slider goes `from` one number `to` another, one `step` at a time (`step 0` slides smoothly). A colour word colours a checkbox's tick and a slider's bar: `slider volume [purple]`. Inputs are 220 wide and sliders 200 unless they're given a `width`.
- **The mouse:** click to put the caret somewhere, drag to select, double-click for a word, three clicks for everything, Shift+click to select up to there. Drag a slider's knob, or click anywhere along it.
- **The keyboard:** Tab and Shift+Tab move between inputs, checkboxes, sliders and buttons, and what has focus gets a ring. In an input: the arrows, Home and End (with Shift to select, and with Ctrl, or Option on a Mac, to go by words), Backspace and Delete, and Ctrl+A, C, X and V (Cmd on a Mac) to select all, copy, cut and paste. Space ticks a checkbox and presses a button, and so does Enter on a button. The arrows move a slider by one step. Escape takes focus away.

### The pieces

| | |
| --- | --- |
| Layout | `column`, `row`, `stack` (layers), `scroll`, `space`, `grid`, `split` |
| Rows of data | `table` with `column`s, `list` |
| Showing parts | `tabs` with `tab`, `branch` |
| Drawing | `canvas` with `box`, `circle`, `line` |
| Content | `text`, `button`, `input`, `textarea`, `checkbox`, `switch`, `number`, `slider`, `image`, `icon`, `progress` |
| Choosing | `dropdown`, `radio`, `menu` with `item` and `divider`, `datePicker`, `colourPicker` |
| Over the view | `dialog`, and `window` beside it |
| Time, keys and files | `every`, `key`, `drop`, `tray` |

Questions and loops work inside a view: showing something only sometimes is an `is … ? then`, and a list is a `loop`.

### Text boxes

`textarea notes;` is an input with several lines, for notes, descriptions and messages. It's bound to a string field like an input, and the string holds the lines with `\n` between them:

```
textarea notes [placeholder "Write something…", width fill, height 160] {
    upToDate = false;
}
```

- **Lines:** Enter starts a new line, and lines too long for the box wrap between words. The text box is 300 wide and four lines tall unless it's given a `width` and `height`; when there's more, it scrolls (with the wheel, or by itself to keep the caret in view).
- **Keys:** everything an input has, and Up and Down go a line up or down, Home and End to the start or end of the line, and Ctrl+Home and Ctrl+End (Cmd+Up and Cmd+Down on a Mac) to the start or end of the whole text. Pasting keeps the copied lines.
- **`enter { }`** on a text box runs on Ctrl+Enter (Cmd+Enter on a Mac), since Enter starts a new line: `textarea message enter { send(); }`.

### Grids

`grid { … }` puts what it holds in columns, left to right and then onto the next row, like cards in a shop. `columns 3` gives it three columns; `cell 180` instead fits as many columns as there's room for with each at least 180 wide, so the cards rewrap as the window gets narrower or wider:

```
grid [cell 180, gap 12] {
    loop (int id, string name, float price in shop) {
        column [pad 12, round, background gray1] {
            text name [bold];
            text price.text() [muted];
        }
    }
}
```

A grid is as wide as the room it's in, its columns share that room equally, and the pieces in a row are all as tall as the tallest, so cards line up. It has a `gap` of 12 unless told otherwise. Without `columns` or `cell`, cells are at least 160 wide.

### Splits

`split { … }` puts its pieces side by side as panels, with a line between each two that can be dragged to make one bigger and the other smaller. `split [down]` puts them one above the other:

```
split {
    column [width 220] { … }      -- a side panel, 220 wide to start with
    column { … }                  -- the main area takes the rest
}
```

A split fills the room it's in. Panels given a `width` (or a `height`, going down) start at that size, and the others share the rest. The window remembers where each line was dragged to while it's open. Panels can't be dragged smaller than 40.

### Tabs

`tabs { … }` holds `tab`s, and shows the names of them all in a bar with one tab's pieces under it. Clicking a name shows that tab:

```
tabs {
    tab "Items" { … }
    tab "Stats" { … }
    tab "Settings" { … }
}
```

The window remembers which tab shows. Bound to a string field, `tabs page { … }` shows the tab whose name the field holds, and clicking a name changes the field, so the code can switch tabs too: `page = "Stats";`. Tabs can come from a `loop`. A colour word colours the name of the tab that shows: `tabs [purple]`. Only the tab that shows is drawn, and timers and shortcuts in the others wait until it shows. From the keyboard, Tab reaches the names, Left and Right go to the tab before or after, and Enter or Space shows it.

### Branches

`branch "Weapons" { … }` is a name with an arrow that opens and shuts what's under it when it's clicked. Branches inside branches make a tree, like folders:

```
branch "Weapons" [open] {
    text "Sword";
    branch "Bows" {
        text "Longbow";
        text "Crossbow";
    }
}
```

`open` makes a branch start open. The window remembers which branches are open by their names and the names of the branches around them, so two branches with the same name in the same place open and shut together. From the keyboard, Enter or Space opens or shuts one, Right opens it and Left shuts it.

### Progress bars

`progress done [to total];` shows how far along something is, as a bar that fills up. It goes `from` 0 `to` 100 unless told otherwise, is 200 wide and 8 tall unless given a `width` and `height`, and a colour word colours the part that's filled: `progress loaded [to 1.0, width fill, green]`.

### Drawing: canvas

`canvas [width 300, height 150] { … }` is a box to draw in. What's in it goes where its `x` and `y` say, counted from the canvas's top left, instead of one after another, and anything outside the canvas is cut off. Three shapes are drawn only in a canvas:

| | |
| --- | --- |
| `box [x 10, y 20, width 40, height 60, blue]` | a rectangle, `round` for rounded corners, `border` for an outline |
| `circle [x 150, y 60, radius 20, red]` | a circle around the point x, y |
| `line [x 0, y 100, toX 300, toY 20, thick 2, gray6]` | a line from x, y to toX, toY, `thick` wide |

A colour word fills a box or circle and colours a line. Any other piece can go in a canvas too, placed the same way: `text "Score" [x 10, y 8, bold]`, a `button`, a picture. A box or circle with a `{ }` runs it when it's clicked. With a `loop`, a canvas draws from the data, like a bar chart:

```
canvas [width 320, height 160, background gray1, round 8] {
    line [x 10, y 140, toX 310, toY 140, thick 1, gray5];
    int i = 0;
    loop (int s in sales) {
        box [x 20 + i * 60, y 140 - s * 14, width 40, height s * 14, round 4, blue5] { picked = s; }
        i = i + 1;
    }
}
```

A canvas is 300 by 150 unless it's given a `width` and `height`.

**The mouse on a canvas.** Blocks written after a canvas's `{ }` run when the mouse does something on it, each given where the mouse is as a `vec2`, counted from the canvas's top left (inside its `pad`) like `x` and `y`:

| | |
| --- | --- |
| `press (vec2 at) { … }` | the left button goes down on the canvas |
| `drag (vec2 at) { … }` | the mouse moves with the left button held, after a press on this canvas. It keeps coming when the mouse leaves the canvas, until the button is let go, so `at` can be outside it (below 0, or past its width) |
| `release (vec2 at) { … }` | the left button is let go, after a press on this canvas |
| `hover (vec2 at) { … }` | the mouse moves over the canvas with no button held |

Each is there only if it's written, and any of them can leave out its `( )` when it doesn't need the place: `release { strokes += 1; }`. A box or circle with its own `{ }` gets a click on it first, and then the canvas's `press` doesn't run. The right button doesn't press: it opens the canvas's right-click `menu { }`, if it has one. A paint program, with a dot wherever the mouse is dragged:

```
assembly Paint {
    has {
        vec2 array dots = [];
        string where = "";
    }

    view [title "Paint", width 420, height 320] {
        column [gap 8, pad 12] {
            canvas [width 400, height 260, background gray1] {
                loop (vec2 d in dots) {
                    circle [x d.x, y d.y, radius 3, blue];
                }
            } press (vec2 at) {
                dots:inject(at);
            } drag (vec2 at) {
                dots:inject(at);
            } hover (vec2 at) {
                where = at.x.text() + ", " + at.y.text();
            } menu {
                item "Clear" { dots = []; }
            }
            text where [muted];
        }
    }
}

Paint:open();
end
```

The mouse pointer is a cross over a canvas that has `press`, `drag` or `release`.

### Motion: animate

`animate` on a piece makes it glide to its new place when it moves, instead of jumping, and fade in when it first shows up. In a list, when a row is removed the rows under it slide up, and a new row fades in:

```
loop (string n in names) {
    text n [animate, pad 6, round, background gray1];
}
```

A moving piece is known by the pieces around it, its kind and its first words, so it's followed even when the rows above it change. Scrolling and changing the window's size don't make pieces glide. Hover looks fade on their own, without `animate`.

### Dropdowns

A dropdown shows what's chosen, and clicking it opens a list to choose from. Bound to a state, its choices are the state's options; bound to a string, it chooses from a list written after `in`:

```
state Shelf = [Fridge, Freezer, Cupboard];
…
Shelf shelf = Shelf.Fridge;
string size = "Medium";
…
dropdown shelf;
dropdown size in ["Small", "Medium", "Large"] { price = priceOf(size); }
```

The list opens below the dropdown (above, near the bottom of the window), scrolls when it's long, and closes when something is chosen, when Escape is pressed, or when the mouse goes down anywhere else. From the keyboard, Enter or Space opens it, Up and Down pick a row and Enter chooses it; with the list closed, Up and Down choose the one before or after straight away. A dropdown is as wide as its longest choice, so it doesn't change size as the choice changes.

### Radio buttons

`radio size;` shows every choice at once, each with a round button, and the one picked is filled in. It works like a dropdown: bound to a state, its choices are the state's options; bound to a string, it chooses from a list written after `in`. Clicking a button or its words picks it:

```
state Size = [Small, Medium, Large];
…
Size size = Size.Medium;
string fruit = "Pear";
…
radio size { price = priceOf(size); }
radio fruit in ["Apple", "Pear", "Plum"] [across];
```

The buttons go down in a column, or with `across` side by side in a row. A colour word colours the picked one: `radio size [purple]`. From the keyboard, Tab reaches the group as one piece, with a ring round the picked button; the arrows pick the one before or after (going round at the ends, as on Windows), and Space picks the first when none is picked yet. A radio is for a few choices that should all be seen; for many, use a dropdown.

### Switches

`switch sound;` is an on and off switch for a bool, like a light switch: a knob that slides right when it's on. Words after the field go beside it, and clicking them flips it too: `switch sound "Sound";`. A checkbox can have words the same way: `checkbox done "Done";`. Space or Enter flips the switch that has focus. A colour word colours it when it's on: `switch sound [green]`. Use a switch for a setting that takes effect straight away, and a checkbox for choices in a form.

`switch` is a piece only inside a view. Outside one it's still the word other languages use for `check`, and Lust says so.

### Number boxes

`number count;` is a box for an int or a float, with small up and down arrows on its right:

```
number count [min 0, max 99, step 1];
number price [step 0.5, min 0] { total = price * amount; }
```

- **Typing** works as in an input. While what's typed isn't a number that fits, the box gets a red edge and the field keeps its last good number; when focus leaves the box (or Enter is pressed) it shows the field's number again. So the field only ever holds numbers that fit.
- **What fits:** a number written with digits, and a minus in front if it's below zero. Bound to an int, it's a whole number: `2.5` doesn't fit. With `min` and `max`, it's between them (both are kept: `[min 0, max 10]` takes 0 and 10).
- **The arrows** (the up and down ones on the box, the Up and Down keys, and the mouse wheel over the box while it has focus) move the number one `step` up or down, 1 unless told otherwise, and keep it between `min` and `max`. The wheel only works on a box with focus, as on Windows, so scrolling down a long form doesn't change a number on the way past.
- A number box is 120 wide unless it's given a `width`. `min` greater than `max`, written right there, is a mistake before anything runs.

### Date pickers

`datePicker due;` is a box for a date, bound to a string field holding the date the way `Time:date()` writes one: `"2026-11-13"`. An empty string is no date, and the box shows its `placeholder`.

```
string due = "";
…
datePicker due [placeholder "When?"] { reminded = false; }
datePicker trip [min "2026-03-01", max "2026-08-31"];
```

- **Typing.** The date can be typed straight into the box. What isn't a real date (as `Time:isDate` says), or is outside `min` and `max`, gets a red edge and isn't written to the field; once it is one, it is. Leaving the box shows the field's date again. Emptying the box empties the field.
- **The calendar.** Clicking the calendar icon on the right, Alt+Down, or Enter in an empty box opens a month's calendar below the box (above, near the window's bottom), the way a dropdown's list opens. Weeks start on Monday. Today has a ring, the chosen day is filled in blue (or in the picker's colour word: `datePicker due [purple]`), and days of the months before and after are paler. Clicking the month's name shows the year's twelve months, and clicking the year shows twelve years; ‹ and › go back and on. Today, at the bottom, chooses today.
- **Keys while it's open:** the arrows move a day or a week; Page Up and Page Down a month, and with Shift a year; Home and End to the week's Monday and Sunday; Enter chooses the day; Escape or Alt+Up closes it without choosing. Clicking a day chooses it and closes the calendar, and a click anywhere else closes it too.
- **`min` and `max`** are dates, written in place or from fields: days outside them are pale and can't be chosen. Written in place, one that isn't a real date is a mistake before anything runs, and so is a `min` after the `max`.
- The `{ }` runs whenever the date changes, after the field holds it. A date picker is 150 wide unless given a `width`.

### Colour pickers

`colourPicker tint;` is a small button showing a colour field's colour and its hex text. Clicking it, or Enter or Space when it has focus, opens a pop-up to choose another:

```
colour tint = Colour:hex("#3b82f6");
colour glass = [1, 1, 1, 0.5];
…
colourPicker tint { preview = tint:fade(0.3); }
colourPicker glass [alpha, swatches [white, gray3, Colour:hex("#ffd43b80")]];
```

- **The pop-up** has a square of colours (more saturated to the right, brighter to the top), a strip of hues under it, a box with the hex text, and a row of swatches to choose from. Dragging in the square or along a strip changes the field as the mouse moves, and the `{ }` runs each time, as with a slider.
- **See-through.** With `alpha`, an alpha strip shows too, and the hex can be `#rrggbbaa`. Without it, a colour chosen is always solid.
- **The hex box** takes `#rrggbb`, `#rgb` (each digit twice), and with `alpha` `#rrggbbaa` and `#rgba`, with or without the #, in capitals or not. While what's typed isn't one of those, it has a red edge and the colour stays as it was.
- **Swatches** are Lust's colour words at their middle shade (`white`, `gray5`, `black`, `red5`, `orange5`, `yellow5`, `green5`, `teal5`, `blue5`, `indigo5`, `purple5` and `pink5`), or a colour array of the program's own: `swatches [red5, myBrand]`.
- **Closing.** Enter, or a click outside, keeps the colour. Escape puts back the colour it had when it opened.
- **Keys while it's open:** Tab goes from the square to the hue strip, the alpha strip, the hex box and the swatches (Shift+Tab back). The arrows move the square's spot (Left and Right for saturation, Up and Down for brightness), the strips' knobs and the picked swatch, one step at a time, or ten with Shift.
- **Exactly the same colour.** Opening and closing the pop-up never changes the field, not even by a rounding step, and the hue stays put while the colour is a grey. A colour chosen is the same in `lust run` and in a built program, to the bit: `#3b82f6` typed in the box is exactly `Colour:hex("#3b82f6")`.
- The field has to be a `colour`. `colourPicker` is spelled like the type; `colorPicker` is a mistake that says so.

### Icons

`icon Icon.Trash;` draws a small picture from Lust's own set, in the text's colour, sharp at any size. `size` sets how big it is (20 unless told), and a colour word colours it:

```
row [gap 8] {
    icon Icon.Star [size 18, yellow6];
    text "Favourites";
}
button "Delete" [icon Icon.Trash, red] { remove(); }
button [icon Icon.Settings, tip "Settings"] { showSettings = true; }
```

On a button, `icon` in its `[ ]` puts the icon before the words, or on its own when the button has no words. **Icons are a built-in state, `Icon`,** so an icon is written like any state's option: `Icon.Trash`, `Icon.ArrowLeft`, `Icon.TriangleAlert`. A name Lust doesn't have is a mistake before running, with the nearest one it does have. One that changes is kept in a variable like any state (`Icon shown = Icon.Star;`, then `icon shown;`), and `loop (Icon i in Icon)` goes through them all. Icons used to be written as text (`icon "trash"`); that's a mistake now, which says which option to write.

The icons are a subset of [Lucide](https://lucide.dev) (ISC licence, see `LICENSES/lucide.txt`), drawn from their shapes by Lust itself. These are `Icon`'s options:

| | |
| --- | --- |
| arrows | `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`, `ArrowUpRight`, `ArrowUpDown`, `ArrowLeftRight`, `Undo`, `Redo`, `CornerDownLeft`, `Move`, `Maximize`, `Minimize`, `ExternalLink`, `LogIn`, `LogOut` |
| chevrons | `ChevronUp`, `ChevronDown`, `ChevronLeft`, `ChevronRight`, `ChevronsUp`, `ChevronsDown`, `ChevronsLeft`, `ChevronsRight` |
| marks | `Check`, `X`, `Plus`, `Minus`, `CircleCheck`, `CircleX`, `CirclePlus`, `CircleMinus`, `CircleAlert`, `CircleHelp`, `Info`, `TriangleAlert`, `Ban`, `Circle`, `Square` |
| editing | `Trash`, `Trash2`, `Pencil`, `SquarePen`, `Scissors`, `Copy`, `Clipboard`, `Paperclip`, `Bold`, `Italic`, `Type`, `AlignLeft` |
| finding your way | `House`, `Menu`, `Ellipsis`, `EllipsisVertical`, `Search`, `ZoomIn`, `ZoomOut`, `Settings`, `Funnel`, `ListFilter`, `List`, `LayoutGrid`, `PanelLeft`, `GripVertical` |
| people and files | `User`, `Users`, `Folder`, `FolderOpen`, `File`, `FileText`, `Save`, `Download`, `Upload`, `Cloud`, `Archive`, `Inbox`, `Printer`, `Share`, `Link` |
| media | `Play`, `Pause`, `SkipBack`, `SkipForward`, `Volume2`, `VolumeX`, `Mic`, `Camera`, `Image`, `Video`, `Music` |
| state | `RefreshCw`, `RefreshCcw`, `RotateCw`, `RotateCcw`, `Loader`, `Eye`, `EyeOff`, `Lock`, `LockOpen`, `Key`, `Shield`, `ShieldCheck`, `Bell`, `Star`, `Heart`, `ThumbsUp`, `Bookmark`, `Flag`, `Tag`, `MapPin`, `Globe`, `Wifi`, `Zap`, `Power` |
| time and talking | `Calendar`, `Clock`, `Timer`, `History`, `Mail`, `Send`, `MessageCircle`, `Phone` |
| things | `Sun`, `Moon`, `Gift`, `ShoppingCart`, `ChartColumn`, `ChartLine`, `Activity`, `Code`, `Terminal`, `Database`, `Monitor`, `Gamepad2`, `Trophy`, `Sword`, `Coins`, `Rocket`, `Palette`, `Book`, `Bug`, `Wrench`, `Lightbulb`, `Layers`, `Box`, `Puzzle` |

### Menus

`menu "File" { … }` is a word that opens a list of `item`s, each running its block when it's chosen. `divider;` draws a line between items, and an item can be `disabled`:

```
row [gap 4] {
    menu "File" {
        item "New" { clear(); }
        item "Save" [disabled upToDate] { save(); }
        divider;
        item "Quit" { confirming = true; }
    }
    menu "Help" {
        item "About" { showingAbout = true; }
    }
}
```

A row of menus at the top of the window is a menu bar, but a menu can go anywhere, like a "⋯" menu on a row of a list. Items can come from a `loop`. The keyboard works as in a dropdown, and Down opens a menu too.

### Right-click menus

`menu { … }` written right after a piece, with no name, is that piece's right-click menu. It's written the way `enter { }` is: in place of the piece's `;`, or after its `{ }`. A right-click on the piece opens it at the mouse:

```
text note [width fill] menu {
    item "Pin" { pinned = note; }
    divider;
    item "Delete" [disabled locked] { remove(); }
}
button "Save" { store(); } menu {
    item "Save as…" { storeAs(); }
}
```

- **The same menu.** Its `item`s, `divider`s, `disabled` items, items from a `loop` and its keys are those of `menu "File" { }`; only how it opens differs. A `menu` with a name is a menu of its own that shows on the screen; a `menu` with none, right after a piece, is that piece's right-click menu.
- **Opening it.** The right button opens it where the mouse is let go: below and to the right of the mouse, or above or to the left near the window's edges. From the keyboard, Shift+F10 or the Menu key opens the menu of what has focus (or of the piece around it), at its bottom left, with its first item picked.
- **Closing it.** Choosing an item, Escape, or a click anywhere else closes it. A right-click elsewhere closes it and opens the menu there instead.
- **In a table or list,** the menu belongs to the rows, so its items can use the row's variable, and it's written after the table's `{ }` (and its `open { }`, if it has one). A right-click picks the row under the mouse first, as Windows Explorer does, so `select`'s field holds it too:

```
table (Person p in people) [select chosen] {
    column "Name" p.name;
} menu {
    item "Forget " + p.name { forget(p); }
}
```

- A `disabled` piece's menu doesn't open, and neither do the rows' menus of a disabled table or list. A right-click outside an open dialog does nothing.
- A menu, item, divider, dropdown, dialog, tab, `key`, `every` or `drop` can't have one, and one with no items is a mistake before anything runs.

### Dialogs

`dialog confirming { … }` pops up in the middle of the window while the bool it's bound to is true, over the rest of the view, which is dimmed. Only the dialog can be used while it's open. Setting the bool to false closes it, and so does Escape or a click outside it, which set the bool to false themselves:

```
button "Delete" [red] { confirming = true; }

dialog confirming [width 320] {
    text "Delete " + name + "?" [bold];
    text "This can't be undone." [muted];
    row [gap 8, right] {
        button "Cancel" { confirming = false; }
        button "Delete" [red] {
            remove();
            confirming = false;
        }
    }
}
```

A dialog is a column with a white background, rounded corners and room around what it holds; styles change it like any other piece. It takes no room where it's written, so it can go anywhere in the view. Tab goes round the dialog's own pieces, and its `key` shortcuts and timers work only while it's open.

### Second windows: window

`window showingSettings [title "Settings", width 400, height 300] { … }` is a real second window of the computer's, open while the bool it's bound to is true. It's written inside a view, like a dialog, and takes no room where it's written:

```
assembly Notes {
    has {
        bool showingSettings = false;
        string name = "";
    }

    view [title "Notes", width 480, height 320] {
        column [gap 8, pad 16] {
            text "Hello " + name;
            button [icon Icon.Settings, tip "Settings"] { showingSettings = true; }
        }

        window showingSettings [title "Settings", width 300, height 160, escape] {
            column [gap 8, pad 16] {
                input name [placeholder "Your name"];
                button "Done" { showingSettings = false; }
            }
        }
    }
}
```

- **Opening and closing.** Setting the bool to true opens the window, and false closes it. Its X sets the bool to false, and so does Escape when the window is written with `[escape]` (once Escape has taken focus out of an input, and closed any dialog or list open in the window).
- **The data is shared.** A window is part of its view, so it reads and changes the same fields: typing a name in Settings changes the main window's text straight away, and anything that changes the data draws every open window again.
- **Its own focus, keys and timers.** Each window has its own Tab order, its own `key` shortcuts and `every` timers, and its own dialogs, dropdowns and toasts. Its shortcuts and timers work only while it's open, and its keys only while it's the window in front, as a dialog's do. The main window's shortcuts don't work in it.
- **Several at once.** Any number of windows can be open together, and a window can hold any piece, a dialog or another window too. A window is open while its bool is true and the view reaches it: inside a question that's false, or a loop with no rows, it's closed.
- **Closing the main window ends the program** and closes the others with it.
- **Its settings** are those of the view: `title`, `width`, `height`, `minWidth`, `minHeight`, `background`, `font`, `dark` and `fullscreen`, worked out from the data each time, plus `escape`. It's dark when the view is, unless it says `dark` itself: `window help [dark false] { … }`.
- The bool has to be a field, as for a dialog: `window count { … }` with an int is a mistake, and so is `close { }` after a window, since its X closes it through its bool.

### Full screen

`fullscreen isFull` in the view's `[ ]` (or a window's) makes the window fill its screen, with no edges or title bar, while the bool is true. It's bound to the field both ways: when the window leaves full screen by the computer's own means, the field becomes false.

Lust gives full screen no key of its own. Programs add one, usually F11 as in a browser, with a shortcut:

```
view [title "Slides", fullscreen isFull] {
    …
    key "f11" { isFull = not isFull; }
    key "escape" { isFull = false; }
}
```

`fullscreen` on its own, without a bool, is always full screen. Bound to anything but a bool, it's a mistake.

### Hiding and closing the window

`visible shown` in the view's `[ ]` hides the main window while the bool is false, without ending the program. It's for programs that live in the tray (below), which show their window again from the tray's menu.

`close { … }` written after the view's `{ }` is what the main window's X does. With it, the X doesn't end the program: the block runs instead, and can hide the window, or ask first:

```
view [title "Notes", visible shown] {
    …
    dialog asking {
        text "Save first?";
        button "Save" { save(); App:quit(); }
        button "Don't save" { App:quit(); }
    }
} close {
    is unsaved? then { asking = true; } or { App:quit(); }
}
```

`App:quit()` ends the program from any block a view runs: every window closes, as when the main window's X is clicked without a `close { }`, so `App:open()` gives back and the code after it runs, nodes' `onClose`s and all. Outside a window it does nothing, since a program without one ends when it reaches its end. A `close { }` after anything but the view is a mistake.

### Toasts

`toast "Saved";` shows a small message in the bottom right corner of the window, which fades after about 4 seconds. It's a statement, so it goes in any block a view runs (a button's, a timer's, a method it calls), not straight in the view:

```
button "Save" {
    save();
    toast "Saved" [green, icon Icon.Check];
}
button "Delete" { toast "Can't delete a note that's open" [red, icon Icon.TriangleAlert, seconds 8]; }
```

- **How it looks:** white words on dark grey, or on its colour with a colour word, like a button: `[green]`, `[red]`. `icon Icon.Check` puts one of Lust's icons before the words, and `seconds 8` keeps it for 8 seconds (more than 0).
- **Several toasts** stack up, the newest at the bottom, up to five. A click on one puts it away.
- **Which window:** a toast shows in the window whose code made it. Made from the tray, it shows in the main window. With no window open, it has nowhere to show and is let go.
- `toast` is a word only at the start of a statement, so a variable can still be called `toast`. A toast of something that isn't text is a mistake, as with `text`: `toast count.text();`.

### Notifications: App:notify

`App:notify("Saved", "Your notes are safe.")` shows the computer's own notification, with a title and words under it, which stays in the system's list of notifications. It works from anywhere, with a window or without one.

- **On Windows** it's a Windows notification, shown at the bottom right like any app's. A program that isn't installed has no name of its own in Windows' list of apps, so Windows names the notification after PowerShell, as it does for other small programs.
- **On Linux** it goes to the desktop's notification service (through D-Bus). On a computer without one, nothing shows.
- **On a Mac** it does nothing yet.
- In tests it prints `notify: Saved — Your notes are safe.` instead.

`App` is a built-in object with `quit` and `notify`. An assembly of the program's own can still be called `App`: `App:open()` opens it, and `App:quit()` and `App:notify(…)` are still the built-ins, unless it has methods called `quit` or `notify` of its own.

### The tray icon: tray

`tray` puts an icon in the system tray (the notification area at the bottom right on Windows) while it's in the view. Its `{ }` holds `item`s and `divider`s, as a menu does, shown when the icon is right-clicked, and `click { }` after it runs when the icon is clicked:

```
view [title "Notes", visible shown] {
    …
    tray [icon Icon.Bell, tip "Notes"] {
        item "Show" { shown = true; }
        item "New note" { add(); shown = true; }
        divider;
        item "Quit" { App:quit(); }
    } click {
        shown = true;
    }
} close {
    shown = false;
}
```

- **The icon** is one of Lust's icons, drawn white on a blue square: `icon Icon.Bell`. `picture "tray.png"` uses a picture file instead, found the way `image` finds files. `tip` is the words shown when the mouse rests on it.
- **The menu** is Windows' own tray menu, with the items' words; a `disabled` item is greyed out. Items can come from a loop or a question, and the menu follows the data.
- **Clicks:** a right-click opens the menu, and a left click runs `click { }`, if there is one.
- **One tray** in a view: a second is a mistake.
- **Only on Windows.** On Linux and on a Mac, a tray does nothing (the libraries for it there need GTK), so a program that hides its window should still offer a way back, or not hide it there.

### Dark mode

`dark` in the view's `[ ]` turns the whole window dark. It can depend on the data, so a checkbox or a setting can switch it: `view [title "Notes", dark darkMode]`.

In dark mode the greys turn round: `white` backgrounds become dark ones, the `gray` shades swap ends (so `gray1`, the palest, becomes the darkest), and dark words become light. The window, inputs, pickers and their pop-ups, buttons, lists and dialogs follow. Other colours stay as they are, except as backgrounds, where their palest shades (`blue1` to `blue3`, and so on) become deep tints of the colour, so a pale background stays a quiet one.

`dark:[ … ]` on a piece holds styles it has only in dark mode, for anything that needs a different colour there: `text "New" [background green1, color green8, dark:[color green3]]`.

Second windows and toasts follow dark mode too. A window is dark when the view is, unless it has `dark` of its own.

### Tips

`tip "…"` on any piece shows a small note below it when the mouse rests on it for a moment, and hides it when the mouse moves away or clicks: `button "↻" [tip "Load the list again"] { reload(); }`.

### Enter and keyboard shortcuts

`enter { … }` after an input runs when Enter is pressed in it, so a form can be sent from the keyboard. The input keeps focus, ready for the next one:

```
input newTodo [placeholder "Something to do…"] enter { add(); }
input search { results = find(search); } enter { open(results[0]); }
```

`key "ctrl+s" { save(); }` is a keyboard shortcut: its block runs when those keys are pressed anywhere in the window. Spaces and capitals are fine too: `key "Ctrl + Shift + S"`. Like `every`, it takes no room, and it works only while it's in the view, so a shortcut inside a question is there only some of the time:

```
key "ctrl+s" { save(); }
key "f1" { showHelp = true; }
is showHelp? then {
    key "escape" { showHelp = false; }
}
```

- **Writing the keys:** `ctrl`, `shift` and `alt` in front of one key, joined with `+`: `"ctrl+shift+s"`. The key is a letter, number or sign (`"ctrl+1"`, `"ctrl+="`), `f1` to `f12`, or `enter`, `escape`, `tab`, `space`, `backspace`, `delete`, `up`, `down`, `left`, `right`, `home`, `end`, `pageup` or `pagedown`. On a Mac, `ctrl` means Cmd, so `"ctrl+s"` is Cmd+S there, as Mac users expect.
- **Typing comes first.** While an input has focus, the keys it uses (letters, the arrows, Backspace, Escape, Ctrl+C and the rest) go to the input, so a shortcut like `"s"` doesn't fire while someone types an "s". Other shortcuts with Ctrl, and the F keys, work while typing. Enter and Space press the button that has focus, and Tab moves focus.

### Files dropped on the window

`drop (string array paths) { … }` runs when files are dragged from the computer's own folders and dropped on the window, given their whole paths. Like `key`, it takes no room, and it works only while it's in the view:

```
drop (string array paths) {
    loop (string p in paths) {
        is p:endsWith(".png")? then {
            pictures:inject(p);
        }
    }
}
```

- **One block for each drop,** given every file dropped together. While files are held over the window, a soft blue edge shows round it.
- **Which block runs.** A drop runs one block: the first `drop` in the dialog opened last, if it has one, or else the first `drop` in the view, as they're written. The window's computer doesn't say where in the window the files were dropped, so a drop is for the whole window, not for a piece.
- `drop` is given a `string array`: `drop (int x)` or `drop { }` without its paths is a mistake.

### Fonts

Lust draws its words in Inter, in four weights. `font "fonts/Lora.ttf"` draws a piece's words in a TrueType or OpenType font file of your own instead, found the way `image` finds its files. On the view's own `[ ]` it's the font for the whole window, and pieces can still have their own:

```
view [title "Recipes", font "fonts/Lora.ttf"] {
    column [pad 20, gap 8] {
        text "Pancakes" [huge, font "fonts/Lobster.ttf"];
        text "Mix, rest, fry.";
    }
}
```

A font file is one weight, so `bold` doesn't change a font of your own: use the bold file of that font instead. A file that isn't there, or isn't a font, leaves the words in Inter. Fonts are read from their file as they're needed, so even a big font costs little memory.

**Letters a font doesn't have.** Inter has no Chinese, Japanese or Korean letters, and few symbols. A letter the font lacks is taken from the computer's own fonts instead, read the first time one is needed: on Windows Yu Gothic UI, Microsoft YaHei UI, Malgun Gothic and Segoe UI Symbol, and on Linux Noto Sans CJK where it's installed. Words are measured with the same letters they're drawn with, so carets and selections line up. Where no font has the letter, it shows as a box.

### Sharp on any screen: scaling

Many screens are set to show everything bigger than its pixels: 125%, 150% or 200% in Windows' display settings, so words stay easy to read on a small, sharp laptop screen. A program that ignored this would look tiny there, or be stretched up by the computer and come out blurry.

Lust draws each window at its screen's own scale. Sizes in a view are the view's own units, not pixels: `width 400` is 400 units, which is 400 pixels at 100%, 500 at 125% and 800 at 200%. So a view looks the same size on every screen, only sharper on a sharper one:

- **Letters, icons and SVG pictures** are drawn again at the screen's scale, so they're sharp at any size. Thin lines, borders and rounded corners sit on whole pixels, so a line one unit thick is one crisp pixel at 125%, not a blur across two.
- **PNG and JPEG pictures** are as big as their file in units, so a 32 × 32 picture is 32 units across, and smoothed when it's drawn bigger or smaller. For screens at 150% and more, a sharper picture twice the size can sit beside it, named with `@2x`: with `cat.png` and `cat@2x.png` in the same folder, `image "cat.png";` uses `cat@2x.png` there, at the same size.
- **Moving between screens.** When a window is dragged to a screen with another scale, it keeps its size in units and is drawn again sharp for the new screen.
- **Everything else stays in units:** a window's `width`, `height` and `minWidth`, where the mouse is (a canvas's `vec2`s too), how far it moves before a row is dragged, the edges of a table's columns, and how far the wheel scrolls.

On Windows, Lust tells the computer it draws for each screen's scale itself (per-monitor DPI awareness), so Windows doesn't stretch its windows.

### Typing in any language: input methods

Chinese, Japanese, Korean and many other languages have far more letters than a keyboard has keys, so they're typed through an input method (IME): keys put a few letters together (Japanese "nihon" shows as にほん), and choosing from a little list turns them into the finished words (日本).

Lust's inputs work with the computer's input method: an `input`, a text box, a number box, a date picker's box and a colour picker's hex box. While one has focus, the input method is on, and its list of choices shows beside the caret.

- **While it's putting letters together,** they show at the caret with a line under them, but aren't in the field yet, so the field and its `{ }` don't change. Enter, Escape, Backspace and the arrows go to the input method then, not to the field, `enter { }` or the view's shortcuts.
- **When it's finished,** the words go in as if they'd been typed, and the bound field changes, with its `{ }`.
- Letters Lust's own font doesn't have come from the computer's fonts (see Fonts, above).

### Screen readers

A screen reader reads out what's on the screen, for people who can't see it well or at all, and lets them use a program from the keyboard: Narrator and NVDA on Windows, Orca on Linux. It can't see Lust's pixels, so Lust also describes each window as a tree of its pieces: what each is (a button, a check box, a slider, a table), what it's called, its value, and whether it's checked, picked, open, shut or disabled. Screen readers read that tree aloud as focus moves.

Every piece is in it, with no extra code: buttons, checkboxes, switches, radio buttons, sliders, number boxes, inputs and text boxes (with their caret and selection), dropdowns, menus and their items, dialogs (which a screen reader keeps to while they're open), tabs, tables (with their headers, how they're sorted, and their rows and cells), lists, date and colour pickers, progress bars, text, pictures and second windows. Toasts are read out when they show, without moving focus. A table or list says how many rows it has, but only the rows on the screen are in the tree, so 100,000 rows cost no more than ten.

**Names.** A screen reader says each piece's name. It's the piece's words (`button "Save"`), or else its `tip`, or else its icon's name. An icon-only button or a picture should be given a name of its own with `label`, which is said instead and isn't shown:

```
button [icon Icon.Trash, label "Delete the note"] { remove(); }
image "chart.png" [label "Sales went up in March"];
input search [label "Search", placeholder "Find…"];
```

An input's name is its `label`, or its `tip`; its `placeholder` is said too while it's empty. `label` takes text. A window's name is its `title`.

**What a screen reader can do.** It can click a piece, move focus to it, give an input, number box, slider, date or colour a new value, move a slider or number box a step up or down, open and shut a dropdown, menu, picker or branch, and scroll a piece into view. Each goes through the same paths as the mouse and keyboard, so fields change through their bindings and blocks run as they would. Focus is shared both ways: where Lust's focus moves, the screen reader follows, and where the screen reader moves focus, so does Lust's.

`dump access` in a test prints the tree, and `act` does what a screen reader would, so a view can be checked without one.

### Scrolling

`scroll { … }` is a column whose content can be taller than it is. The mouse wheel or touchpad scrolls it, a knob on its right shows where you are and can be dragged, and Tab scrolls to what it reaches.

A scroll is as wide as it can be, and in a column it takes the room that's left (and so does whatever holds it), so `column { text "Title"; scroll { … } }` fills the window with a title above a list. Give it a `height` to make it a fixed size instead.

A scroll makes, lays out and draws everything in it, even what's scrolled out of sight, which is fine for a few hundred rows. For long lists, of thousands of rows or more, use `list` or `table` (below), which make only the rows on the screen.

### Tables

`table` shows a row for each item of an array, in columns with a header each. Its `( )` reads like a loop's: what each row is called, and where the rows come from:

```
table (Person p in people) [select chosen, height 300] {
    column "Name" p.name [width 160];
    column "Age" p.age;
    column "" [width 40] {
        button [icon Icon.Trash, subtle] { forget(p); }
    }
} open {
    edit(chosen);
}
```

- **Columns.** `column "Age" p.age;` shows a value: text, a number, a bool or a state. `column "" { … }` holds any pieces instead, like a button on each row. The header is text written in place. A column with a `width` has it, and the others share the rest. Dragging the line at a header's right edge makes the column wider or narrower (40 at least), and the window remembers the widths while it's open, as a split remembers its lines.
- **Sorting.** Clicking the header of a column that shows a value sorts the rows by it, smallest first, with an arrow pointing up; clicking again sorts largest first, with the arrow down; a third time puts the rows back in the array's own order. Numbers sort as numbers, text without minding capitals (`ada` next to `Ada`), bools false first, and states in the order their options are written. Rows that are the same keep their order. A column of pieces sorts too when it's told by what: `column "Note" [sort p.note] { … }`. Sorting only changes what's shown, never the array.
- **Picking a row.** `select chosen` puts the row that's clicked in `chosen`, a field of the rows' type (a `Person` field, which can be `nothing` when none is picked). The picked row is blue, and stays picked wherever it moves, since it's the row itself that's picked, not its place. Up and Down pick the row above or below, Home and End the first and last, and Page Up and Page Down go a page at a time, scrolling to keep the picked row in sight.
- **Opening a row.** `open { … }` after the table runs on a double-click on a row, or Enter, after the row is picked, the same shape as `enter { }` on an input.
- **The keyboard.** Tab reaches the table as one piece, with a ring around it, and then the buttons and inputs in the rows on the screen.
- **How it looks.** The header stays put while the rows scroll under it, thin lines go between the rows, the row under the mouse is shaded softly, and `[stripes]` shades every other row. In dark mode it all turns dark with the rest.
- **Where the rows come from.** An array, or a cMap or database table, written as in a loop: `table (int id, string name, float price in shop)`. For a cMap or a table, `select` picks the row's key (the first column), so it's bound to an int or string field.
- **Size.** A table scrolls by itself. It's as wide as it can be, and with no `height` it fills the room it's given, like a split; inside a scroll it's as tall as about eight rows.
- **Quick with any number of rows.** Only the rows on the screen are made, laid out and drawn, plus a few spare, so 100,000 rows scroll as smoothly as ten. That needs every row to be one height: the first row's, or `rowHeight 28`.

A piece of your own in a row keeps its memory with its row (by the row itself, or a cMap's key), and an input in a row keeps its text and focus while the table scrolls, as long as its row stays on the screen.

### Long lists: list

`list` is a scrolling column of rows, one for each item, made only while they're on the screen, so it stays quick with tens of thousands of them. Each row holds the pieces in its `{ }`, and every row is one height (the first row's, or `rowHeight`):

```
list (Message m in inbox) [rowHeight 48, select current] {
    row [gap 8, padX 12] {
        text m.from [bold];
        text m.subject [muted];
    }
} open {
    show(current);
}
```

It picks rows (`select`), opens them (`open { }`) and takes the same keys as a table, with no columns or headers. Like a table it fills the room it's given and scrolls by itself.

### Dragging rows to new places: reorder

`[reorder]` on a `list` or `table` lets the rows be dragged to new places with the mouse, and the array itself changes order, the way an input changes its field:

```
list (string job in jobs) [reorder, select current, rowHeight 32] {
    text job [padX 8];
}
```

- **Dragging.** Hold a row and move the mouse: once it has gone about 4 points, the row is dragged, and a blue line shows where it will land. Letting go moves it there in the array, and picks it. Near the list's top or bottom edge, the list scrolls by itself, faster the nearer the edge. Escape lets go without moving anything.
- **The keyboard.** With the list (or table) focused, Alt+Up and Alt+Down move the picked row one place up or down.
- **The picked row stays picked** wherever it moves, since it's the row that's picked, not its place. A list of 100,000 rows still makes only the rows on the screen.
- **A sorted table can't be reordered.** While a header sorts it, the rows show in another order than the array's, so dragging and Alt+Up and Alt+Down do nothing. Clicking the header until it shows the array's own order again (the third click) lets the rows be dragged again.
- **Only an array's rows** can be reordered: a cMap's or a database table's rows are found by their keys and have no order of their own, so `[reorder]` on one is a mistake before anything runs. A `disabled` list or table can't be reordered.

### Pictures

`image "cat.png";` shows a PNG, JPEG or SVG file, found the way `Files` finds files: from the folder the program runs in, or anywhere with its whole path. Without a size it's as big as its file (in the view's units; a `cat@2x.png` beside it is used on screens scaled 150% and more, see Scaling), shrinking to fit if there isn't room. Give it a `width` or a `height` and the other follows, keeping its shape. Give both and it fits inside that box; add `cover` and it fills the box instead, cut to fit. `round` rounds its corners, so `[width 48, height 48, cover, round 24]` makes a round picture. A file that isn't there shows as a grey box.

An SVG picture is drawn again at the size it shows, so it's sharp at any size, and without a size it's as big as the SVG says. Everything in SVG's own rules is drawn (shapes, paths, colours, gradients, see-through, groups and transforms) except words written in the SVG and pictures inside it, which are left out.

### Timers

`every 1.0 { seconds = seconds + 1; }` runs its block every second, and the window redraws after. It runs only while it's in the view, so a timer inside a question starts and stops with it:

```
is running? then {
    every 1.0 {
        seconds = seconds + 1;
    }
}
```

A timer takes no room on the screen. If the program was busy and a timer is late, it runs once, not once for every time it missed.

### Tweens: values that glide

A `tween` moves a value from where it is now to a new one over some seconds, a little each moment, instead of jumping: a health bar draining, a door swinging open, the sky turning orange at sunset. Tweens work everywhere, not only in views:

```
tween hp to 0 over 0.5;
tween pos to [10, 0, 0] over 2 [ease OutCubic];
tween sky to orange5 over 1 [ease InOutSine] {
    print("sunset done");      -- runs when it ends
}

float t = Ease:apply(Ease.OutBack, 0.3);   -- easing on its own
```

- **The shape:** `tween place to value over seconds;`, then, if you want them, `[ease Name]` and a `{ }` that runs when it ends (its **done block**). With a done block, there's no `;` after the `}`, the same as `every`.
- **What it moves:** a field (`hp`, the thing's own; `Clock.hour`, an object's; `player.hp`, another thing's) or a variable at the top of the file, holding an `int`, `float`, `vec2`, `vec3`, `colour` or `rotation`. An int is rounded each step the way `Maths:round` rounds; a vec2's, vec3's and colour's parts move together, and a rotation turns the shortest smooth way, as `lerp` does on them.
- **Not a method's own variable.** A tween keeps going after the method ends, so `tween local to 1 over 1;` is a mistake: keep it in a field. The same goes for a variable inside `{ }` at the top of the file. Text, bools, arrays, things and states can't be tweened, and neither can one part of a vector (`pos.x`): tween the whole `pos` instead.
- **Where it's going is worked out once,** when the tween starts. A tween on another thing's field (`player.hp`) keeps that thing for as long as it runs, even when nothing else does.
- **At the end** the place is exactly the value, then the done block runs. The done block is like a hook (section 11): it can read the variables around it as they were when the tween started, and change fields, but not those variables. It can start another tween.
- **Changing the place yourself** (`hp = 5;`) while it's tweening doesn't stop the tween: its next step moves the place on again. Start a new tween to change where it's going.
- **A new tween replaces the old.** Starting a tween on a place that's already tweening stops the old one where it is and starts from there; the old one's done block never runs.
- **`over 0`** sets the place straight away and runs the done block. A time below zero stops the program with a message saying so; one written right there (`over -1`) is a mistake before anything runs.
- **`tween`, `to` and `over`** are words only in a tween, so a variable can still be called `to` or `over`.

**Easing** shapes how fast the value moves. With no `[ease …]` it moves at one speed, `Linear`. The others start slow (`In…`), end slow (`Out…`), or both (`InOut…`), in a family of curves, from the gentlest to the most lively:

| Family | The eases |
| --- | --- |
| straight | `Linear` |
| Sine | `InSine`, `OutSine`, `InOutSine`: the gentlest |
| Quad, Cubic, Quart, Quint | `InQuad` … `InOutQuint`: each a little sharper than the one before |
| Expo | `InExpo`, `OutExpo`, `InOutExpo`: very slow, then very fast |
| Circ | `InCirc`, `OutCirc`, `InOutCirc`: like a quarter circle |
| Back | `InBack`, `OutBack`, `InOutBack`: goes a little past the end (or back before the start) first |
| Elastic | `InElastic`, `OutElastic`, `InOutElastic`: wobbles like a spring |
| Bounce | `InBounce`, `OutBounce`, `InOutBounce`: bounces like a dropped ball |

Each family has `In`, `Out` and `InOut`: `InCubic`, `OutCubic`, `InOutCubic`. They're the standard curves from [easings.net](https://easings.net), which draws each one. `Back` and `Elastic` go past where they're going on the way, so a colour's parts are kept between 0 and 1 then, and an int can go past its end before settling.

- **`Ease` is a state** whose options are the curves, so `[ease OutCubic]` writes the option on its own, as `when` does, and a variable can hold one, chosen while running: `Ease chosen = Ease.OutBack; tween x to 1 over 0.3 [ease chosen];`. `check` and `loop (Ease e in Ease)` work as for any state. An ease Lust doesn't know is a mistake that suggests the nearest one.
- **`Ease` is an object too:** `Ease:apply(Ease.OutBack, t)` gives the curve's value for `t` from 0 to 1 (a `t` outside that is kept to 0 or 1), for easing things yourself.

**The clock.** A tween moves when the program's clock moves, in steps:

- **In a window**, each frame, on the same clock as `every` timers and `animate`, and the view redraws after each step. In a view's tests, `wait 2.5` moves tweens and timers along together, a tween 1/60 s at a time.
- **In a plain program**, a `wait` lets the clock run: tweens move on during it, in steps of 1/60 s, so `tween x to 10 over 1; wait 1;` leaves `x` at 10 with its done block run. They move while any run waits, and without a wait, nothing moves. When the program ends, tweens still running just stop. A done block is a run of its own (section 11, Waiting), so a `wait` inside it pauses only it.
- **In a game engine**, the engine moves them once a tick (section 21).

A tween takes the same steps in `lust run` and a built program, so both end with exactly the same numbers. The curves that use sines and powers (Sine, Expo, Elastic) use the computer's own maths library, as `Maths:sin` does, so they can differ in the last digit between Windows and Linux; on one computer, both ways always agree.

### Styles

Short words in `[ ]`, in the spirit of Tailwind:

| | |
| --- | --- |
| spacing | `pad 16`, `padX 8`, `padY 4`, `padTop`, `padRight`, `padBottom`, `padLeft`, and `gap 8` between children |
| size | `width 200`, `height 40`, `width fill`, `height fill`, `grow 2`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight` |
| text | `size 24`, `tiny`, `small`, `large`, `huge`, `weight 600`, `bold`, `muted`, `strike`, `nowrap` |
| colour | a colour word on its own colours text (or a whole button), `color …` and `background …`, and `subtle` for a button with no background until the mouse is over it |
| shape | `round`, `round 12`, `pill`, `border`, `border red`, `borderWidth 2`, `shadow`, `opacity 0.5` |
| placement | `left`, `center`, `right` side to side; `top`, `middle`, `bottom` up and down; `spread` |
| under the mouse | `hover:[ … ]`, `tip "…"` |
| for screen readers | `label "Delete the note"` |
| in dark mode | `dark:[ … ]` |
| windows | `title`, `width`, `height`, `minWidth`, `minHeight`, `background`, `font`, `dark`, `fullscreen`, `visible` (the main window), `escape` (a second window) |
| toasts | a colour word, `icon Icon.Check`, `seconds 8` |
| the tray | `icon Icon.Bell`, `picture "tray.png"`, `tip "Notes"` |
| inputs and pictures | `placeholder "…"`, `from`, `to`, `step` (sliders), `min`, `max`, `step` (number boxes), `min`, `max` (date pickers, as dates), `alpha`, `swatches [ … ]` (colour pickers), `across` (radios), `cover` |
| icons | `icon Icon.Trash` on a button |
| grids, splits, branches | `columns 3`, `cell 180`, `down`, `open` |
| tables and lists | `select chosen`, `rowHeight 28`, `stripes`, `reorder`, and `sort p.age` on a column |
| in a canvas | `x`, `y`, `toX`, `toY`, `radius`, `thick` |
| motion | `animate` |
| words | `font "file.ttf"` |
| off | `disabled`, or `disabled` with a bool: `disabled not ready` |

**Colours** are `white`, `black`, `gray1` to `gray9`, and `red`, `orange`, `yellow`, `green`, `teal`, `blue`, `indigo`, `purple` and `pink`. Each of those has nine shades from light to dark: `blue1` is a pale blue for backgrounds, `blue9` a deep one. Any other colour is written `"#2f9e44"`, or held in a `colour` (section 6): `[background sky]` with `colour sky = [0.4, 0.7, 1.0];`. A colour's see-through (`"#2f9e4480"`) shows on a background. The shades are the Open Color palette.

**Sharing the room.** Pieces are as big as what's in them. `width fill` in a row (or `height fill` in a column) takes the room that's left over; several fills share it equally, and `grow 2` takes twice the share of `grow 1`. A `space` is an empty fill, so it pushes what comes after it to the end. `minWidth` and `maxWidth` keep a piece between two sizes.

**Placing.** On a column, `left`, `center` and `right` place its children side to side, and `top`, `middle` and `bottom` up and down in any room to spare. On a row it's the same, and its children sit in the middle up and down unless told otherwise. `spread` puts the spare room evenly between the children: `row [spread] { text "Back"; text "Next"; }` puts one at each end. A row or column told `center`, `right` or `spread` stretches across the room it's in, so there's somewhere to place its children. On a text, `center` and `right` place its lines.

A `stack` lays its children on top of each other, the later ones in front, placed by these same words: a picture with a label on it, or a badge in a corner.

**Text wraps** onto more lines when it's wider than the room it has, breaking between words. `\n` in the text starts a new line, and `nowrap` keeps it on one line.

**Values can come from the data.** After a style word, a value can be any number, text or bool: `[grow done, background shade, size 12 + extra, disabled not ready]`, with fields and variables. Only colour words (`red`, `gray3`…) and `fill` are read as words.

Name a style to reuse it: `style card = [pad 16, round, shadow];`, then `[card]`.

### Underneath

Lust draws everything itself, from the pixels up: layout, every piece, text editing, focus, scrolling, styles and animation. Small permissive libraries only open the window (winit), hand over the pixels (softbuffer), read font files (ab_glyph), describe the window to screen readers (AccessKit: UI Automation on Windows, AT-SPI on Linux), show the computer's own boxes for choosing files (rfd), reach the system clipboard (arboard), read PNG and JPEG files (png, jpeg-decoder) and draw SVG files (resvg). On Windows, tray-icon puts the tray icon in the notification area, and tauri-winrt-notification shows notifications; on Linux, zbus sends notifications over D-Bus. Icons are Lucide's shapes, drawn by Lust. Views are for `.lst` programs on Windows, macOS and Linux. Web pages stay HTML, CSS and JavaScript.

---

## 18. Web: schema and route

A Lust program can be a server: it answers web requests with JSON. Pages in the browser stay HTML, CSS and JavaScript.

### schema: the shape of data

A schema describes data coming in. A field with no value must be filled in; a field with a value is optional:

```
schema NewUser {
    has {
        string name;
        int age = 0;
    }
}
```

- A schema is a type like a class, but it only holds fields: no methods, no `like`.
- `new NewUser { name = "Ada" }` must fill in every field with no value.
- When JSON is read into a schema, a missing field without a value is an error that says which field.

### route: answering requests

```
method User getUser(int id) { … }
method User createUser(NewUser body) { … }
method removeAll() { … }

route "/api" {
    get "/users/:id" => getUser;
    post "/users" => createUser;
    delete "/users" => removeAll;
}

Web:serve(8080);
```

- A route block lists paths, each answered by a method. `get`, `post`, `put` and `delete` are the kinds of request. The text after `route` goes in front of every path, and can be left out.
- **The method's values come from the request.** A value named like a `:name` in the path comes from the path: an `int`, a `float` or a `string`. Any other value comes from the request's body, read as JSON: usually a schema. There can be one of those at most.
- **What the method gives back is the answer,** as JSON, with status 200. A method that gives nothing back answers 204.
- **`Web:fail(Answer.NotFound, "there's no user 9.");`** stops and answers with that status, as `{"error": "…"}`. The answer is an option of `Answer`, a built-in state, and goes over the network as its number (below). A path value that isn't a number, or a body that doesn't fit, answers 400 by itself. Any other error answers 500 with its message.
- A path nobody answers gives 404. One that's answered, but not for that kind of request, gives 405.
- **`Web:serve(8080);`** answers requests on that port until the program is stopped. Every file's routes are served. `pass route;` is fine to write but not needed.
- **`Web:request(Request.Get, "/api/users/2", "")`** tries a route without a network, and gives back the status and the answer as text: `200 {"id":2,"name":"Bo"}`. Good for testing. The kind of request is an option of `Request`, a built-in state, one for each route word.
- Requests are read and answered several at once, so a slow visitor doesn't hold up the others. The methods that answer run one at a time, so they never get in each other's way.

### Request and Answer

The kinds of request and the answers a server sends are fixed choices, so they're built-in states, not text or numbers.

| `Request` | Sent as | Answered by |
| --- | --- | --- |
| `Request.Get` | GET | `get "…" => …;` |
| `Request.Post` | POST | `post "…" => …;` |
| `Request.Put` | PUT | `put "…" => …;` |
| `Request.Delete` | DELETE | `delete "…" => …;` |

| `Answer` | Number | When |
| --- | --- | --- |
| `Answer.BadRequest` | 400 | the request doesn't make sense: something's missing or wrong in it |
| `Answer.NotSignedIn` | 401 | whoever asked has to sign in first |
| `Answer.NotAllowed` | 403 | whoever asked is known, but isn't allowed to do this |
| `Answer.NotFound` | 404 | what was asked for isn't there |
| `Answer.Conflict` | 409 | it can't be done as things are now: the name is taken, say |
| `Answer.Gone` | 410 | it was there, but it's been removed for good |
| `Answer.TooBig` | 413 | what was sent is too big |
| `Answer.TooMany` | 429 | too many requests: slow down and try again later |
| `Answer.Broken` | 500 | something went wrong on the server |
| `Answer.Busy` | 503 | the server can't answer just now: try again later |

- Both are states like any other: kept in variables (`Request kind = Request.Put;`), gone through with `loop (Answer a in Answer)`, and shown as their names.
- The old way, text for the request (`Web:request("GET", …)`) and a number for the answer (`Web:fail(404, …)`), is a mistake that says which option to write: `Write Request.Get here.`, `Write Answer.NotFound here.` A number with no option (`Web:fail(418, …)`) lists them all.
- What `Web:request` gives back still starts with the status as a number (`404 {"error":…}`), the way it arrives over the network.
- `Request` and `Answer` are Lust's names, so nothing of your own can be called either (section 22).

### Asking other websites

```
string answer = Web:get("https://api.example.com/weather?city=Oslo");
Weather w = Json:read(answer);
string reply = Web:post("https://example.com/scores", Json:write(score));
```

- Both give back the answer's text. Turn JSON into Lust values with `Json:read`.
- **Only the run that asks waits** while the request is out: the window, other runs, tweens and timers go on (section 11, Slow built-ins pause only their run). `start` a method that asks to keep going straight away.
- If the site can't be reached, or answers with an error (400 and up), the program stops with what happened, like `https://example.com/x answered 404: …`. Use `try` to carry on: its `Problem` is `NotFound` for 404, `NotAllowed` for 401 and 403, and `NoAnswer` for the rest (section 15).
- A program that asks websites is built against Lust's libraries, like ones with databases. In LustScript, it needs `lust run`.

---

## 19. Built-ins and the standard library

| Built-in | Does |
| --- | --- |
| `print(x);` | shows `x` on its own line. Any type: numbers, text, bools, lists, vectors, colours, states and `nothing` |
| `print(a, b, c);` | shows several values on one line, with a space between each: `print("Score:", score, "lives:", lives);` |
| `wait seconds;` | pauses its own run while everything else goes on (section 11, Waiting) |
| `wait { is …? }` | pauses its own run until every question is true, with `or after seconds { … }` for a time limit (section 11, Waiting) |
| `ask(question)` | shows a question and gives back what's typed |

### Random numbers

```
int dice = Random[1, 6];          -- a whole number from 1 to 6
float chance = Random[0.0, 1.0];  -- a decimal from 0 up to (not quite) 1
```

- With two ints, and going into an int, it's a whole number, and both ends can come up.
- With a float, or going into a float, it's a decimal from the lowest up to just below the highest.
- Each run rolls different numbers, even two runs started at the same moment. To get the same numbers every time (for tests, or a world made from a seed), set `LUST_SEED` to a number before running, or write `Random:seed(42);` in the program: `lust run` and the built program then roll exactly the same numbers. `lust test` sets `LUST_SEED` to 1.

`Random` has a few more tools:

| | Gives |
| --- | --- |
| `Random:seed(42);` | starts the rolls again from that seed (an int), so what follows is the same every run |
| `Random:chance(0.25)` | `true` about a quarter of the time. It takes 0 (never) to 1 (always); anything else stops the program |
| `Random:direction()` | a random vec3 1 long, any way equally likely: for sparks, wandering, aiming a little off |
| `names:pick()` | a random item of an array. An empty array stops the program |
| `cards:shuffle();` | puts the array in a random order, in place, like `sort();` |

All of them use the same rolls as `Random[1, 6]`, so a seed makes them repeat too.

### Text

Positions count letters (not bytes), from 0, like arrays.

| | Gives |
| --- | --- |
| `name.upper()`, `name.lower()` | the text in capitals, or in small letters (a–z and accented letters like æ, ø, å, é) |
| `name.trim()` | the text without spaces and line breaks at either end |
| `name:contains("a")`, `:startsWith("a")`, `:endsWith("a")` | true or false |
| `name:find("a")` | where it first is, or -1 if it isn't there |
| `name:replace("old", "new")` | the text with every `old` changed to `new` |
| `line:split(",")` | a string array of the parts between the commas. `:split("")` gives every letter |
| `name:slice(2, 5)` | letters 2, 3 and 4 |
| `line:matches("[0-9]+")` | true if the pattern is in it somewhere |
| `line:findAll("[0-9]+")` | every piece that fits the pattern, as a string array |
| `line:capture("Order ([0-9]+)")` | the parts in `( )` of the first fit, as a string array (empty if nothing fits) |
| `line:replaceAll("[0-9]+", "#")` | every fit replaced. `$1`, `$2` put back the parts in `( )` |

Patterns are regular expressions, the kind most languages use: `[0-9]` is a digit, `+` means one or more, `( )` marks a part to keep. In LustScript they need `lust run`.

### Arrays

| | Does |
| --- | --- |
| `words:join(", ")` | one text from a string array, with `", "` between the items |
| `scores:has(10)`, `scores:find(10)` | whether it's there, and where (or -1). For arrays of numbers, text, bools, states, entities and things. A state is found only as that very option, as `===` asks (`Light.On` doesn't find `Light.On.Dim`), an entity by which one it is, and a thing by which thing it is, as `==` compares things: another `new Hero` with the same fields isn't found |
| `scores:sort();` | puts numbers or text in order, smallest first |
| `scores:reverse();` | turns the array around |
| `team:sortBy(score);` | puts an array of things in order by one of their fields. Things with the same value keep their order |
| `scores:sum()` | all the items added up: an int for an int array, a float for a float array. An empty array sums to 0 |
| `scores:average()` | the sum divided by how many there are, always a float |
| `scores:smallest()`, `scores:largest()` | the smallest and largest item, the same kind as the items |
| `names:pick()`, `cards:shuffle();` | a random item, and a random order (see Random numbers above) |

`sum`, `average`, `smallest` and `largest` work on arrays of ints or floats; on anything else they're a mistake before running. An empty array has no average, smallest or largest, so asking stops the program: "the array has no items, so it has no average."

### Built-in objects

Lust comes with these objects. Their methods are called like any object's: `Maths:sqrt(2.0)`. `App` (`App:quit()`, `App:notify("Title", "Body")`) is with views in section 17, `Web` is in section 18, `Random` under Random numbers above, `Rotation` and `Colour` (`Colour:hex("#3a8f2a")`) with vectors and colours in section 6, and `Ease` (`Ease:apply(Ease.OutBack, t)`) with tweens in section 17.

**Maths**

| | Gives |
| --- | --- |
| `Maths:abs(x)`, `Maths:min(a, b)`, `Maths:max(a, b)`, `Maths:clamp(x, low, high)` | an int for ints, a float when there's a float |
| `Maths:wrap(n, count)` | n wrapped round into 0 to count - 1, counting backwards below 0: `Maths:wrap(-1, 5)` is `4` and `Maths:wrap(5, 5)` is `0`. For going round a list both ways: `at = Maths:wrap(at - 1, #items);`. With floats it wraps into 0 up to (not including) count, as for angles: `Maths:wrap(-90.0, 360.0)` is `270.0`. An int for ints, a float when there's a float. A count of 0 or below stops with `Problem.OutOfRange` |
| `Maths:round(x)`, `Maths:floor(x)`, `Maths:ceil(x)` | the nearest whole number, the one below, the one above (an int) |
| `Maths:trunc(x)` | the whole part, dropping what's after the point (towards zero): `Maths:trunc(-2.75)` is `-2`. An int |
| `Maths:fraction(x)` | what's after the point, with x's sign: `Maths:fraction(-2.75)` is `-0.75`. A float |
| `Maths:sign(x)` | -1, 0 or 1: which side of zero x is. An int for an int, a float for a float |
| `Maths:sqrt(x)`, `Maths:power(x, p)`, `Maths:sin(x)`, `Maths:cos(x)`, `Maths:tan(x)` | a float. Angles are in radians |
| `Maths:asin(x)`, `Maths:acos(x)`, `Maths:atan(x)` | the angle (in radians) whose sine, cosine or tangent is x. `asin` and `acos` take -1 to 1 |
| `Maths:atan2(y, x)` | the angle of the direction (x, y), from -π to π: which way something points. `Maths:atan2(d.y, d.x)` for a vec3 `d` |
| `Maths:degrees(r)`, `Maths:radians(d)` | radians to degrees, and back |
| `Maths:log(x)`, `Maths:exp(x)` | the natural log (x above zero), and e to the power x. Floats |
| `Maths:lerp(a, b, t)` | the number t of the way from a to b: `Maths:lerp(0, 10, 0.25)` is `2.5`. t isn't held to 0–1, so 1.5 goes past b |
| `Maths:moveTowards(current, target, step)` | current moved towards target by at most step, never past it: one frame's movement. step can't be below zero |
| `Maths:smoothstep(low, high, x)` | 0 below low, 1 above high, and a smooth S curve between: for fades that start and stop gently. low and high must differ |
| `Maths:isFinite(x)` | `false` for `infinity`, `-infinity` and `not a number`, `true` for every ordinary number |
| `Maths:noise(x, y)`, `Maths:noise(x, y, z)` | smooth noise (Perlin noise) from -1 to 1: rolling hills for terrain, clouds, wobble. The same numbers give the same answer every run, everywhere. It's 0 at whole numbers, so step through it by fractions: `Maths:noise(x * 0.1, y * 0.1)` |
| `Maths.pi`, `Maths.e` | 3.14159… and 2.71828… |

**Files.** Paths are from the folder the program runs in.

| | Does |
| --- | --- |
| `Files:read(path)` | the whole file as text |
| `Files:write(path, text);`, `Files:append(path, text);` | writes the file, or adds to its end |
| `Files:exists(path)` | true if there's a file or folder there |
| `Files:remove(path);` | removes a file, or an empty folder |
| `Files:list(folder)` | a string array of what's in the folder, in order |
| `Files:makeFolder(path);` | makes the folder, and any folders it's inside |
| `Files:chooseOpen("Open a picture", ["png", "jpg"])` | shows the computer's own box for choosing a file to open, of those types (`[]` for any), and gives back its whole path, or `""` if it's cancelled |
| `Files:chooseSave("Save the notes", "notes.txt")` | the box for choosing where to save, suggesting a name |
| `Files:chooseFolder("Where to keep them")` | the box for choosing a folder |

**Time**

| | Gives |
| --- | --- |
| `Time:now()` | seconds since 1 January 1970, as a float |
| `Time:clock()` | seconds since the program started: good for timing things |
| `Time:date()`, `Time:time()` | today's date as `"2026-10-21"`, and the time as `"14:05:09"`, by the computer's clock |
| `Time:addDays(date, n)` | the date n days later (or earlier, with a minus) |
| `Time:daysBetween(from, to)` | how many days from one date to the other |
| `Time:weekday(date)` | the day of the week, as an option of the built-in state `Day`: `Day.Monday` to `Day.Sunday` |
| `Time:isDate(text)` | true if it's a real date, written like `"2026-10-21"` |

**Days are a state.** A day of the week is a fixed choice, so `Time:weekday` gives back an option of `Day`, a built-in state, not text:

```
Day d = Time:weekday("2026-11-26");     -- Day.Thursday
is d == Day.Saturday || d == Day.Sunday? then { print("weekend"); }
check Time:weekday(due) {
    when Saturday, Sunday { print("it's due at the weekend"); }
    when Monday, Tuesday, Wednesday, Thursday, Friday { print("it's due on a weekday"); }
}
```

| `Day`'s options, in order |
| --- |
| `Monday`, `Tuesday`, `Wednesday`, `Thursday`, `Friday`, `Saturday`, `Sunday` |

- Like any state, a `check` covers every option, `loop (Day d in Day)` goes through them from Monday, and `d.text()` (or `print(d)`) gives its name, `"Thursday"`.
- Comparing one with text (`Time:weekday(due) == "Monday"`) is a mistake that says to write `Day.Monday` instead.
- `Day` is one of Lust's names, so nothing of your own can be called `Day` (section 22). Months have no state: no built-in gives one back.

**Json** turns values into JSON text and back:

```
string text = Json:write(player);     -- {"name":"Ada","hp":20}
string nice = Json:pretty(player);    -- the same, over several lines
Player back = Json:read(text);
```

- JSON holds numbers, text, bools, state options (as their names), vectors, rotations and colours (below), arrays, maps (as JSON objects), cMaps (as arrays of objects), records (as objects of their values; section 10) and things made from classes (as objects of their fields). Hooks stay out.
- **Vectors, rotations and colours** are written so that reading them back gives exactly the same value:

  | | Written as | Also read |
  | --- | --- | --- |
  | `vec3` | `[x, y, z]`: `[1.0, 2.5, -0.1]` | |
  | `vec2` | `[x, y]` | |
  | `rotation` | its quaternion, `[x, y, z, w]`: `[0.0, 0.0, 0.70710677, 0.70710677]` for `[0, 0, 90]` | three numbers, as degrees around x, y and z: `[0, 0, 90]` |
  | `colour` | its hex text, `"#ff922b"` or `"#3a8f2a80"`, when every part is a whole number of 255ths; otherwise its four numbers, `[0.4, 0.7, 1.0, 1.0]` | three numbers (solid), and hex in capitals |

  - **Each number** is written as the shortest text that reads back as exactly the same 32-bit number: `0.1`, not `0.10000000149011612`. Very big and very small ones are written like `3.4028235e38` and `1e-45`. A number that grew past 32 bits is infinity, and JSON can't hold it: `Json:write` stops, saying so. `Json:write` leaves out the spaces after the commas: `[1.0,2.5,-0.1]`.
  - **Why a rotation isn't its degrees:** 32-bit numbers can't hold every turn exactly, so a turn made from the degrees `facing:degrees()` gives is very close to `facing`, but often not the same to the last bit. The quaternion is exact. Degrees written by hand are read as a rotation written in Lust would be.
  - **Why a colour isn't always hex:** hex holds 256 steps for each part, and a colour made from `[0.4, 0.7, 1.0]` or worked out with `lerp` falls between them. Colours from colour words and hex text are always hex.
  - **Reading checks the shape:** the wrong number of numbers, something that isn't a number, a colour's part outside 0 to 1, hex text that isn't `"#rrggbb"` or `"#rrggbbaa"`, a number too big for 32 bits, or a quaternion that isn't 1 long stops with a plain message: `the JSON at home.at should be a vec3, three numbers in [ ], but it has 2 items.`
- `Json:read` needs a place to go, so Lust knows what it's reading: `Player back = Json:read(text);`.
- A class's fields that are missing from the JSON keep their starting values. Anything extra is left out.
- If the JSON doesn't fit, the error says where: `the JSON at bag[0] is missing item.`

The `choose` boxes are the ones the computer itself has (Windows', macOS', or the desktop's on Linux). While one is open only the run that opened it waits, so the window keeps drawing and its other buttons still work (section 11, Slow built-ins pause only their run); on macOS, where the boxes open only on the program's main thread, the window waits too. In a view's tests no box opens: `choose pictures/cat.png` in the test's input gives the answer the next box gives, and `choose cancel` cancels it. The code after the box goes on once the test's clock moves (`wait 0`).

In LustScript, `Files:list`, `Files:makeFolder`, the `choose` boxes and the precise clock need `lust run`. Inside other programs, like Garry's Mod, they say they aren't available.

**Testing:** `lust test <folder>` runs every program that has an expected result next to it: `.out` for what it prints, `.err` for the mistakes it should report, `.in` for what's typed in.

---

## 20. A complete file

```
-- main.lst
fetch "system/gameState":GameState;
fetch "entities/player":Player;

state AppState = [
    Booting,
    Running
];

method start() {
    print("Starting up...");

    Player hero = new Player { name = "Ada", hp = 100 };

    is hero.hp > 0? then {
        print("Ready: " + hero.name);
    } or {
        print("Knocked out");
    }
}

pass AppState;

start();

end
```

---

## 21. Lust inside other programs

Lust can be one part of a bigger program, like a game engine written in Rust: the engine does the drawing, physics and sound, and Lust holds the game's own logic (weapons, inventories, rules).

### Native methods

A method can say its body is somewhere else, in the program Lust is part of, by ending with `native;` instead of `{ … }`:

```
-- engine/entity.lst: made by the engine. It describes what the engine offers.
class Entity {
    has {
        int id = 0;
    }

    method float x() native;
    method moveBy(float dx) native;
    method say(string words) native;
}

method float now() native;

pass Entity, now;
end
```

The game uses it like any other file: `fetch "engine/entity":Entity;` and `get "engine/entity":now;`. Lust checks every call against it, so a typo or a wrong type is caught before anything runs, even though the bodies are elsewhere.

- A native method takes and gives back ints, floats, text, bools, `vec3`s, `vec2`s, `rotation`s, `colour`s, states' options, things made from classes, and arrays of any of those (`Thing array`, `vec3 array`).
- **States' options** cross by their full name, inside options and all: `method animate(Clip clip) native;` hands the program `"Farmer.Walk"` for `Clip.Farmer.Walk`. One given back is checked by name: if its state has no such option, the program stops on the line that asked, saying what came back and which options there are. So `==`, `===` and `check` work on what a native gives back like on any option.
- Where the body comes from:
  - **built as a Rust library** (below): from the Rust program using it;
  - **LustScript** (`.lsc`) and **Lua for an engine**: from a Lua function in the table `LUST_NATIVES`, by the method's name, or `Class_method` for a class's method, which is given the thing first. A function with the same global name works too, except for a method on its own: Lust's own function for it has that name, so its body goes in `LUST_NATIVES`;
  - **on its own** (`lust run`, or a program built without `--library`): nowhere, so calling it stops with a message saying where its body lives.

### Methods on entity

An engine can give its `entity` type methods of its own, so what it does to one entity reads as a method on it: `target:position()`, `crop:destroy()`. Its own files declare them, always `native`:

```
-- solis/entity.lst: made by the engine.
method vec3 entity:position() native;
method entity:moveTo(vec3 place) native;
end
```

```
-- a game's file: nothing to bring in.
entity p = player();
p:moveTo(p:position() + [1, 0, 0]);
moveTo(position() + [speed * dt, 0, 0]);      -- in a component: abilities are called bare
```

- **Only the engine's own files** give them: the files in its **host folders** (`solis/` for Solis). An engine names them when it uses the compiler (`lust::Engine`, below); on the disk, a project names them in a `lust.host` file in its folder, one to a line (for trying an engine's files with `lust check` and tests; an engine using the compiler only goes by its own `lust::Engine`, never a `lust.host`). Those files come with every program, so the methods work everywhere without `fetch` or `get`; anything else in them is still brought in as usual. In particular, a component or object in the engine's folders (a ready-made one, say) is part of a game only once the game fetches it, so it never clashes with a game's own of the same name.
- They're checked like any method: what goes in and what comes back. Each is given once.
- Their bodies: in Rust, the program's `entity_move_to(&self, this: Option<u64>, place: Vec3)`, the entity first; in Lua, the function `entity_moveTo(e, place)` in `LUST_NATIVES`.
- `.text()` and `==` work as before, and stay Lust's own: an `entity:text()` is a mistake.
- `entity:len` goes by `entity_len` in Rust and Lua, so a method of the game's called `entity_len` is a mistake that says so.

### Lust as a Rust library

`lust build game.lst --library` makes `game_library`, a folder holding a Rust library, next to the file. A Rust program uses it with one line in its `Cargo.toml`, which Lust prints:

```
[dependencies]
game = { path = "game_library" }
```

What the Rust program sees:

- **`game::start()`** runs the Lust file's own start: it makes its objects, then runs the code written outside classes and methods. Call it once, first.
- **Each class the first file `pass`es** is a Rust type: `Weapon::new()`, a getter and a setter for each field (`ammo()`, `set_ammo(…)`), and each method (`fire(…)`), with names in Rust's style (`lastShot` becomes `last_shot`). An object is `Score::get()`. Methods the first file passes are functions: `game::damage(10, true)`.
- **Classes with native methods** are always there in full, since they belong to the program: it makes them, and fills in their fields.
- **The native methods' bodies:** the program implements `game::Host`, one function each (`entity_move_by(&self, this: &Entity, dx: f64)`, `now(&self)`), and hands it over with `game::set_host(…)` before `start`.
- **Failures come back as a `LustError`.** Everything gives back a `Result`, and a Lust failure says where it happened: `file`, `line` and `column` (of the statement that failed), `message`, and `problem`, its kind as the name of a `Problem` option (`"NotFound"`, section 15). Shown with `{}`, it's the message Lust prints: `game.lst:24  this is nothing, so it has no fields or methods.` The program decides what to do with it.
- **`game::LUST_VERSION`** is the version of Lust that wrote the library.
- **No unsafe code.** A library starts with `#![forbid(unsafe_code)]`, so Rust itself checks it has none. The one exception is a library whose code waits (or asks a website): its runs get stacks of their own, as built programs' do (section 11), so it says `#![deny(unsafe_code)]` with that one module allowed, and its `Cargo.toml` has the corosensei library. The one thing that needed it, the computer's time zone for `Time:date()` and `Time:time()`, comes from the program instead: when the Lust uses either, `Host` has `local_time_offset(&self, unix_seconds: i64) -> i64`, the seconds the local time is ahead of UTC (0 for UTC). A program of its own (`lust build`) still asks the computer.
- Ints, floats, text and bools are Rust's `i64`, `f64`, `String` and `bool`. A state's option comes in as `&'static str` (its full name) and goes back as a `String`, checked by name. A thing made from a class is `Option<Weapon>`, since it can be `nothing`. A `vec3` is the library's `Vec3 { x, y, z }` and a `rotation` its `Rotation { x, y, z, w }` (a quaternion), all `f32`, the same layout as Solis's and glam's. A `vec2` is its `Vec2 { x, y }`, and a `colour` its `Colour { r, g, b, a }` (`to_array()` gives `[f32; 4]`, and `Colour::from([r, g, b, a])` makes one); a colour coming in from the program has each part kept between 0 and 1. An array is a `Vec` of its items, copied each way.
- **In LustScript**, the host's Lua function hands a `vec3` back as a Luau `vector` or any table with `x`, `y` and `z`, a rotation as a table with `x`, `y`, `z` and `w`, and an array as a plain list. Lust rounds the numbers to f32 and counts the list, so they're the same as Lust's own. In Luau, the `vec3`s Lust hands over are Luau vectors. A `vec2` is a table with `x` and `y`, and a `colour` a table with `r`, `g`, `b` and `a` (an `a` left out is 1), both ways, in plain Lua and Luau alike.

`lust test` tests libraries too: a `game.host.rs` next to `game.lst` is a Rust program that uses it, and its output is compared with `game.out`.

### The compiler inside an engine

An engine can use Lust's compiler itself, as a Rust library: to check a game's Lust and show its mistakes, to turn it into Lua while the game runs, and into Rust for a release. In its `Cargo.toml`, at a version's tag (or at a commit, with `rev = "…"` in place of `tag`):

```
[dependencies]
lust = { git = "https://github.com/Rhyslos/Lust", tag = "v2.0.0-alpha.68", default-features = false }
```

Without its default features it needs nothing but Rust's own library (the default ones are the `lust` command). Lust doesn't open any files: the engine reads them, and hands over each one's path and text, with `/` between folders.

```
let files = vec![
    ("main.lst".to_string(), main_text),
    ("parts/gun.lst".to_string(), gun_text),
];
match lust::to_lua("main.lst", &files) {
    Ok(chunk) => { /* load chunk.code */ }
    Err(problems) => { /* show them */ }
}
```

- **`lust::check(entry, &files)`**, **`lust::to_lua(entry, &files)`** and **`lust::to_rust_library(entry, &files)`**: `entry` is the program's first file. `fetch` and `get` find the others among `files`, from the project's folder (the nearest one up with a `main.lst`).
- **`lust::check_with(&engine, entry, &files)`**, **`to_lua_with`** and **`to_rust_library_with`** do the same inside an engine: `lust::Engine { folders: vec!["solis".into()], records: true }` names its host folders, whose files come with every program and may give `entity` methods, and says what it can carry beyond the rest: `records`, records in components' fields and events (ENGINES.md, alpha 96, says what that takes). In mistakes the engine is called by its first folder, with a capital (`Solis`). The calls without `_with` are these with no host folders, and no records. On the disk, a `lust.host` line `supports records` says the same.
- **Mistakes:** every one found, in order, as a `Problem` with `file`, `line`, `column` and `message`. Shown with `{}`, it's what `lust check` prints: `parts/gun.lst:12:5  ammo is an int, but this is text.`
- **Deep code is a mistake, not a crash.** Code nested more than 150 levels deep (brackets, blocks, or operators in a row like `a + b + c …`) is reported as a `Problem` like any other, so Lua and Rust can always take what Lust accepts. The compiler runs on a thread of its own with room to spare (64 MB of stack, reserved but not used), so it never runs the engine out of stack, whatever it's given.
- **`lust::VERSION`** is the compiler's version, so an engine can show it and refuse one it doesn't expect.
- **`to_rust_library`** gives what `lust build --library` puts in `src/lib.rs`.

**The Lua** (`chunk.code`) runs in Lua 5.1, LuaJIT or Luau. Load it with the name `lust::LuaChunk::NAME` (in mlua: `.set_name(lust::LuaChunk::NAME)`), so a failure says where it came from. The engine gives native methods' bodies, and `send_event(to, name, values)`, in the table `LUST_NATIVES` first. Running the chunk runs the program's own start, and gives back a table:

- **`version`**, **`components`** (by name) and **`events`** (by name: what each carries, as `{ key, name, kind }` lists, kinds being `bool`, `int`, `float`, `string`, `state`, `vec3`, `vec2`, `rotation`, `colour`, `entity` and, once the engine keeps them, `record`, whose entry has its values' entries fourth and its name fifth: `{ "kit", "kit", "record", { { "ammo", "ammo", "int" } }, "Kit" }`). A record's value is a table of its values by name, made fresh each time it crosses, either way.
- **`new(name, entity)`** makes a component with its starting values, on an entity. The engine calls its `update(dt)` each tick, and `handle(event, values)` to deliver an event (it gives back whether the component handles it; the handler is a run, so it may still be waiting).
- **`removed(component)`** tells Lust the engine has removed a component, so its waiting runs are cancelled and its tweens stop (Runs in an engine, below).
- **`swap(component)`**, on the table from new code, gives a component made by an earlier chunk the new code, in place, so the engine's hold on it stays good. It follows the same rule as `from_data`: a field kept by name keeps its value; a field whose kind changed, or a new one, takes its starting value; a removed field is dropped; the entity it's on stays. It gives back the component and notes on what started over (`Gun's ammo is float now, not int, so it starts at 10.`), or `nil` and a note if the component isn't in the code any more.
- **`changes(old)`**, given the earlier chunk's table, lists the components whose fields `changed`, and those `removed` and `added`, so an inspector knows what to show again.
- The code outside classes and methods runs again with each new chunk, and objects start over.

**Vectors and rotations:** in Luau, a `vec3` is a Luau `vector` (`vector.create(x, y, z)`; mlua's `Vector`): in components' fields, in what events carry, and in what native methods are given and give back. Luau keeps a vector's numbers as f32s, as Lust does. A `rotation` is a table, `{ x, y, z, w }`. In plain Lua, which has no vectors, a `vec3` is a table, `{ x, y, z }`, and the chunk's `vectors` says which: `"luau"` or `"table"`. A native method's body may give back a table for a vec3 in Luau too, and Lust turns it into a vector. Lust never changes a rotation in place, so it shares them; an engine setting one gives a new table, and never changes one it got from Lust. A `vec2` is a table `{ x, y }` and a `colour` a table `{ r, g, b, a }`, in Luau as in plain Lua (Luau's vectors have three numbers, and a table keeps a vec2 apart from a vec3); their kinds in a schema are `"vec2"` and `"colour"`. They're shared the same way, so an engine never changes one it got from Lust either.

**Speed:** in Luau, Lust's vector maths is Luau's own, and it rounds numbers to f32 with Luau's vectors, which is many times faster than plain Lua can; it gives exactly the numbers Rust gives. Luau's native code (mlua's `luau-jit` feature) makes it faster again. `bench/` in Lust's own folder ticks 2,000 components both ways, and checks Rust and Luau end with exactly the same values.

**Fields the engine fills in:** in a component's schema their kind is `object`, followed by the class's name (`{ "anim", "anim", "object", "FarmerAnimator" }`). The chunk's **`make(name, values)`** makes a thing of one of the engine's classes with these fields (`api.make("FarmerAnimator", { id = 3 })`), and the engine sets the component's field to it (`farmer.anim = …`). Built as a library, a component's `FILLED` lists them with their classes, and each has its setter (`farmer.set_anim(Some(animator))`).

**Notes:** `chunk.notes` lists what's worth showing but isn't a mistake (a component still writing `me`), each a `Problem` with its file, line and column; the chunk runs either way.

**Views drawn by the engine:** an engine can show an assembly's `view` itself (Solis's screens). Before the chunk loads, it sets `LUST_HOST.ui_open` to a function; `ClassPick:open()` calls it with the view's settings and a function that draws the view, and the engine may keep that function and call it whenever it likes, after `ui_open` has given back. Each call gives back the view's tree. Pieces of the game's own inside it (`ClassCard { role = … }` in `ClassPick`'s view, and pieces inside those) keep their fields from one drawing to the next, as in a window. The view is drawn only through that function; a piece made any other way stops with a message saying so.

**A failure** in the Lua comes to the engine as a Lua error. `chunk.locate(&error.to_string())` turns it into a `RunError`, with `file`, `line`, `column` and `message`, pointing at the Lust.

`engine-test` in Lust's own folder does all of this in Luau, the way Solis does.

### Data files

An engine can keep what its editor makes (tile maps, areas, animators) as Lust too: a **data file** is an ordinary `.lst` whose top level is only `lock` values of fixed values. Lust checks it like any file, and the engine reads it **without running anything**:

```
-- maps/farm.lst: written by the editor.
fetch "solis/tiles":TileMap;
lock int version = 2;
lock TileMap farm = new TileMap {
    tileset = Tileset.Farm,
    size = [40, 36],
    rows = ["....tttt", "....tttt"],
};
end
```

- **Fixed values** are numbers (negative ones too), text without `{ }`, bools, `nothing`, a state's option (`Tileset.Farm`), plain lists of fixed values (`[40, 36]`, a vec3 too), colours written as a colour word (`orange5`) or `Colour:hex("#3a8f2a")` with the text right there, and things made with `new` from fixed values (records too: they come as a `Thing`, like a class's). Anything that would need running (a call, a sum, a variable, a class or a method, `Rotation:toward(…)` too) is a mistake, with its line.
- **`lust::read_values(entry, files)`** gives back a `Values`: each one's name, its type as written, its line and its value as a `Fixed` (`Int`, `Float`, `Text`, `Bool`, `Nothing`, `Option("Tileset.Farm")`, `List`, `Colour([r, g, b, a])`, or `Thing { class, fields }`, with `field`, `as_int` and friends to read them). The files are handed over as for `lust::check`: the data file, and the files it fetches. `lust::read_values_with(engine, …)` adds an engine's own folders, as `check_with` does.
- **Vectors, rotations and colours:** a vec3, vec2 or rotation comes as a `List` of its numbers, a rotation's as its degrees around x, y and z, as written. A colour written as numbers comes as a `List` too; one written as a colour word or with `Colour:hex` comes as `Colour`, its four parts as 32-bit numbers from 0 to 1, exactly as the program would have them. `as_vec3()`, `as_vec2()` and `as_colour()` read either way (`as_colour` gives see-through 1 for three numbers).
- **Checked like any file:** a value of the wrong type, a class or field that doesn't exist, or an option a state doesn't have is a mistake with its line. A dotted name must be a state's option: `Tileset.Farm` is fixed, but `farm.scale` reads from something, so it's a mistake. Nothing runs.
- A field left out of a `new` isn't in its `Fixed`: the engine uses the class's starting value.
- **A version** for the engine's own upgrades is just a value: `lock int version = 2;`.
- **Big files are quick:** a 1,000 × 1,000 map, written as a million one-character codes in 1,024 chunks of 32 rows (about 1 MB), reads in about a twentieth of a second, and a 4,096 × 4,096 one (about 19 MB) in about 0.7 seconds.
- Game code shouldn't fetch a big data file: the engine reads it, and game code fetches only the names it uses, from a small file of their own.

### Components

In an engine like Solis, a game is made of entities, and what an entity does comes from its components. A `component` is a class whose fields the engine keeps and shows in its inspector, and whose `update` the engine runs once a tick:

```
state Mode = [Single, Burst];

component Gun {
    has {
        int ammo = 30;
        float cooldown = 0;
        Mode mode = Mode.Single;
        entity target = nothing;
    }

    method update(float dt) {
        cooldown = cooldown - dt;
        is target == nothing? then {
            target = nearest(this);         -- a native method the engine gives
        }
    }
}

pass Gun;
```

- **`this`** is the entity the component is on. Every component has it, and it can't be changed. Hand it over whole (`send Hit(this, 6) to target;`), or ask it for an ability (`this:jump()`). Its text is `this.text()`, as for any entity. It was written `me` before; `me` still works for now, with a note saying to write `this` (from `lust check`, `lust run` and the others, and in a Lua chunk's `notes`; a Rust library has nowhere to put notes), and becomes a mistake once Solis has moved over. So nothing can be called `me` (a variable, field, parameter or loop name), anywhere (section 2). Outside a component, `this` and `me` are mistakes.
- **Abilities are called bare.** Inside a component, the methods the engine gives `entity` (above) are the component's own abilities: `moveTo(position() + [1, 0, 0])` is `this:moveTo(this:position() + …)`. A variable can share an ability's name (`float velocity = velocity();`). The component's own methods come first, then methods brought in with `get`, then the abilities: so a component's own `jump()` wins over the engine's, and `this:jump()` still reaches the engine's.
- **Fields** hold what the engine can keep and show: `bool`, `int`, `float`, `string`, a state (shown as a choice of its options), `vec3`, `vec2`, `rotation`, `colour` and `entity`. A colour field can start as a colour word: `colour tint = blue1;`.
- **Records** (section 10) go in a component's fields too, and in the events the engine carries, once the engine says it keeps them (`records: true` in its `lust::Engine`, below; `supports records` in a `lust.host`). Until then, a record there is a mistake that names the engine: `Solis doesn't keep records in a component's fields yet, so loadout can't be a Loadout.` The engine keeps each one as a copy, and hands Lust copies, so a record is never shared between the engine and the game.
- **Things the engine made for it.** A component can hold a thing of one of the engine's own classes (from its host folders), such as the animator its editor made: `FarmerAnimator anim;`, with **no starting value**. The engine fills it in before the first `update`; reading it before then stops on its line, naming the field: "anim is filled in by the engine before the component's first update, and hasn't been yet." A class of the game's own can't be a component's field (keep those in a class or an object), and one of the engine's can't be given a starting value.
- **`update(float dt)`** runs once a tick, with how long a tick is. The engine runs the entities in order of their IDs, and an entity's components in order of their names, so a game runs the same way every time.
- **`entity`** is a value the engine hands over: an entity, or `nothing`. Entities can be compared with `==`, and `.text()` gives the 13 letters and digits Solis writes them as. Lust can't make one; it gets them from the engine's native methods, from `this`, and from `new` of a class the engine makes (below).
- A component can't be `like` another one yet.
- A component's methods can't be called `handle`, `schema`, `toData`, `setData`, `fromData`, `setMe` or `removed`: the engine uses those on every component.
- **`update` can't wait:** it runs every tick, so waiting goes in a method of its own, started from `update` (`start open();`). Section 11, In an engine.

Built as a library, each component also has what the engine needs to keep, show and run it, in the same kinds as Solis's own (`reflect`):

- `Gun::NAME`, and `Gun::schema()`: its fields, their kinds and their starting values, in the order they're written. `game::components()` lists every component's.
- `gun.to_data()` gives its fields' values by name, as a `Data` (a `BTreeMap<String, Value>`), and `Gun::from_data(&data)` makes one from values. A value that's missing, or of the wrong kind, keeps the starting value.
- `gun.set_data(&data)` changes only the fields `data` has, so an edit made in the editor while the game runs keeps the rest, like ammo left and cooldowns.
- `gun.set_me(Some(id))` puts it on an entity, and `gun.update(dt)` runs a tick. `gun.removed()` says the engine has removed it (Runs in an engine, below). An entity is its 64-bit ID, `Option<u64>`, with `None` for nothing.
- `Value` is `Bool`, `Int`, `Float`, `Text`, `List` (a vec3's three numbers, a vec2's two, a rotation's four, a colour's four from 0 to 1), `Entity` and `Record` (a record's values by name, as a `Data`), and `FieldKind` is `Bool`, `Int`, `Float`, `Text`, `Choice` (with the options), `Vec3`, `Vec2`, `Rotation`, `Colour`, `Entity` and `Record` (its name, and its values' names and kinds in order). A colour from `set_data` has its parts kept between 0 and 1. A `Value::Record` coming in needs every value, each of the right kind, or it's the wrong kind as a whole: `set_data` leaves the field as it is, and `handle` gives an error.

### Things the engine makes

A class in the engine's own files (its host folders) can be one the **engine** makes: it has a native method called `made`, with nothing in its ( ). Then `new` hands the engine the new thing, fields filled in, and gives back what `made` gives back, instead of the thing:

```
-- solis/world.lst, written by the engine
class Fighter {
    has {
        vec3 position = [0, 0, 0];
        float speed = 9;
    }

    method entity made() native;
}

class Horn {
    has {
        vec3 at = [0, 0, 0];
    }

    method made() native;
}
```

```
entity ship = new Fighter { position = gate, speed = 12 };
new Horn { at = gate };
```

- **What `new` gives back** is what `made` does: `entity ship = new Fighter { … };` holds the entity the engine made, and a `made` that gives back nothing makes `new Horn { … }` a line of its own, like a call.
- **On its own line,** a `new` of a class the engine makes does something, so what it gives back can be left unused: `new Fighter { position = gate };`. A `new` of any other class on its own line is a mistake, since nothing keeps what it makes.
- **It doesn't end with a state:** what the engine makes is the engine's, so one made in a method of a class with a state of its own stays when the state changes.
- **Only the engine's own files** make such a class: a class of the game's own with a method called `made` is an ordinary class.
- **The body:** in Rust, the program's `fighter_made(&self, this: &Fighter)`, reading the fields with their getters; in Lua, the function `Fighter_made(thing)` in `LUST_NATIVES`.

### Objects in an engine

An `object` is one thing for the whole program (section 10). In an engine it's one for the whole **game**: a clock and calendar, money, the weather. Game code anywhere reads and changes it by name (`Clock.hour`), and the engine runs and keeps it:

```
object Clock {
    has {
        int day = 1;
        float hour = 6;
    }

    method update(float dt) {
        hour += dt / 60;
    }
}
pass Clock;
```

- **Passed objects only:** the engine ticks and saves an object its file passes (`pass Clock;`), from any file of the game. An object that isn't passed is that file's own helper: the engine leaves it alone.
- **Ticked:** a passed object's `update(float dt)` runs once a tick, objects in order of their names, before the entities' components. An `update` the engine can't call that way (`update(int n)`, one that gives something back) is a mistake when the game is built for the engine, as it is for a component.
- **Saved:** the engine keeps its fields of the kinds a component's are (`bool`, `int`, `float`, `string`, a state, `vec3`, `vec2`, `rotation`, `colour`, `entity`), and gives them back when a save loads. Other fields (arrays, things) start as written. A saved value that's missing, or of the wrong kind, leaves its field as it is, in Lua and in Rust.
- **In Lua:** the chunk's `objects` (by name) and `objectNames` (in order); `tick(dt)` runs the updates; `save(name)` gives an object's kept fields by name, and `load(name, values)` gives them back. A name that isn't a passed object stops with a message saying so.
- **Built as a library:** each object has `NAME`, `schema()`, `to_data()` and `set_data(&data)`; `objects()` lists their schemas and `tick_objects(dt)` runs the updates.

### Tweens in an engine

Tweens (section 17) move when the engine says: once a tick, **after the objects' tick and before the components' updates**, it calls the chunk's **`tickTweens(dt)`** in Lua, or **`tick_tweens(dt)`** built as a library. So an `update` sees this tick's values, and a done block's changes are there for the same tick's updates. A tween started in an `update` first moves on the next tick.

- A tween on a component's field belongs to that component: it moves it, and keeps it, until it ends.
- **New code:** tweens run inside the chunk that started them, so when the engine moves to a new chunk (and calls its `tickTweens`), tweens the old code started stop where they are. Their done blocks don't run.
- **Removing an entity:** the engine says so with `removed` (below), which stops its components' tweens where they are, without their done blocks. A tween on something else (an object's field, say) that a removed component started keeps going.
- `tickTweens` and `tick_tweens` with 0 move nothing. A failure in a done block comes back as from `update`.
- The same call moves Lust's waiting runs on too (below).

### Runs in an engine

Waiting works in a game as it does in a window (section 11, In an engine): each event handler, `start` and tween's done block is a run, and a `wait` pauses only its run, across ticks. The engine drives it with the calls it already makes, and one more:

- **`tickTweens(dt)`** (Lua) or **`tick_tweens(dt)`** (Rust), once a tick after the objects and before the components' updates, moves Lust's clock on by `dt`, moves the tweens, then goes on with every run whose wait is over, in the order they're due. `dt` is the game's own time: 0 while it's paused, smaller while it's slowed. With 0 the clock stays put, but `wait { }`s still look again. An engine that never calls it never freezes: its runs just stay where they wait.
- **`removed(component)`** (Lua: `api.removed(gun)`; Rust: `gun.removed()`), once, when the engine stops running a component for good (its entity is destroyed, the component taken off, or stopped after a failure): its handlers still waiting and the work started in its methods are cancelled (none of their code runs again), what it made in its states ends, and its tweens stop.
- **Failures:** a run that fails before its first wait fails the call that started it (`update`, `handle`, …); one that fails after a wait fails `tickTweens` / `tick_tweens`, with the file and line of the run's own code. Runs due after it that tick go on in the next.
- **What can't wait:** a component's or object's `update`, a method the engine calls, and the program's own start: code the engine calls straight into. A wait there stops that call with a message saying to start a method that waits instead.
- **New code:** runs belong to the chunk that started them, like tweens: once the engine ticks a new chunk instead, the old code's waiting runs never go on.
- **In Lua** each run is a coroutine: a native method's body shouldn't call back into Lust code that waits, since Lua can't pause across it. In Rust each run has a stack of its own, so only a library whose code waits needs them (and the corosensei library).

```
-- once a tick, on each core
api.tick(dt)                          -- the objects
api.tickTweens(dt)                    -- tweens, then runs whose wait is over
gun:update(dt)                        -- each component
gun:handle("Hit", values)             -- events: each handler is a run
api.removed(gun)                      -- its entity is gone
```

ENGINES.md (alpha 89) lists what an engine does to move over.

### Events

Components tell each other things with events (section 11). A component handles them in its `on { }` block, and anything sends one to an entity with `send … to`:

```
event Hit(entity by, float damage);
event Died();

component Health {
    has {
        float hp = 10;
    }

    on {
        Hit(entity by, float damage) {
            hp = hp - damage;
            is hp <= 0? then {
                send Died() to this;
            }
        }
    }
}

component Gun {
    has {
        entity target = nothing;
    }

    method update(float dt) {
        send Hit(this, 6) to target;
    }
}
```

- **`event Name(…)`** says what an event carries, in order. An event that reaches the engine (one a component handles, or one sent to an entity) carries the same kinds a component's fields hold: `bool`, `int`, `float`, `string`, a state, `vec3`, `vec2`, `rotation`, `colour` and `entity`, and records once the engine keeps them. Events heard only by Lust's own things can carry anything.
- **A component's `on { }` block** runs a handler when its event reaches the entity the component is on. Each handler is a run, so it can wait (section 11, In an engine). It takes what the event carries, in the same order (the names can be its own). A component with a state of its own can have entries for some of its options (`Hit(…) in Down { … }`, section 11, Events in some options): `handle` runs the one for the option it's in then. The old way, `on Hit(…) { … }` on its own, is a mistake now that shows the `on { }` form.
- **`send Name(…) to target;`** with an entity hands the event to the engine, which queues it. Nothing runs straight away: the engine delivers the queue in order, to each component on the entity in order of their names. Events sent while delivering (Hit sends Damaged, which sends Died) join the end of the same tick's queue, so a game runs the same way every time.
- **Components hear only the engine.** `send Name(…);` with no `to`, or `to` a thing of a class, stays in Lust: a game's objects and classes can have `on` blocks too, and hear those (section 11). Those events are delivered after each `update` and `handle` the engine runs, after `tick_objects` and `tick_tweens` (`tick` and `tickTweens` in Lua), and after every other call into the library.
- The engine knows events by their names, so two files can't each have an event with the same name. A program that sends to an entity needs the engine, so like native methods it's built with `lust build --library`; `lust run` on its own stops at the first such `send`. Events that stay in Lust need nothing: a plain program with them runs with `lust run` and builds with `lust build`.

Built as a library:

- The engine's `Host` has `send_event(&self, to: Option<u64>, event: &str, values: Vec<Value>)`, which `send` calls.
- Each component has `HANDLES`, the events it handles, and `handle(&self, event, &values)`, which runs its handler: `Ok(false)` if it has none for that event, and an error if the values are the wrong count or kind.
- `game::events()` lists every event the engine can carry (those that keep to its kinds) and what it carries, as `EventSchema { name, values }`.

In LustScript, `send … to entity` calls the host's Lua function `send_event(to, name, values)`, with the values in a list (its count in `n`), and a component's `handle(event, values)` runs its handler. The chunk's `events` lists the events the engine can carry, and its `settle()` delivers the events waiting for Lust's own things, for an engine that runs Lust code some other way.

## 22. Reserved words

These can't be used as names:

```
class      object     node       assembly   create     has
method     hook       return     like       new        nothing
schema     route      view       style      keep       pass
fetch      get        lua        is         then       or
either     not        true       false      loop       stop
skip       in         try        error      state      save
load       saved      end        database   int        float
string     bool       array      cMap       map        native
component  check      when       lock
```

The built-ins can't be used as names either:

```
print      wait       ask
```

Neither can the built-in objects: `Maths`, `Files`, `Time`, `Json`, `Web`, `Random`, `Rotation`, `Colour` and `Ease` (which is a state of easing curves too), and the built-in states `Icon`, Lust's icons, `Problem`, the kinds of failure (section 15), `Day`, the days of the week (section 19), `Request` and `Answer`, the kinds of request and the answers a server sends (section 18), and `Run`, how started work is going (section 11). (`Rotation` and `Colour` with a capital are the objects; `rotation` and `colour` are the types.)

`tween`, `to` and `over` are words only in a tween (and `to` after `send`), so they can still be names. So are `event`, `send` and `record`, words only at the start of a line (`record` only before a name and `{`), and `on`, a word only in the body of a class, object, node, component or assembly. So are `toast`, a word only at the start of a statement, `start`, a word only before a call, `cancel`, a word only before a name, `after`, a word only after a wait's `or`, and `App`, which a program's own assembly can be called.

The pieces of a view (`text`, `button`, `radio`, `switch`, `number`, `icon`, `table`, `list` and the rest) are words only at the start of a line inside a view, so they can still be names: `int number = 3;` works anywhere.

---

## 23. Still to design

- **LustQL (`.lql`):** designed (2026-11-19) in [LUSTQL.md](LUSTQL.md), and built one step at a time. Until then, databases and their questions are written in Lust code (section 14).
- **Ready-made host files:** the files describing Garry's Mod's or Aventail's functions as native methods (section 21), so LustScript there is checked like the rest of Lust.
- **More for engines** (Solis): vectors as real value types, typed events, leaner Rust for code that runs every frame, and rebuilding a game's Lust in a couple of seconds.
- **The standard library:** which Rust crates Lust wraps for files, networking, JSON, dates and sound, and how they look from Lust.
- **From v1:** whether `with x { }`, `{ }` inside text and the word `when` come along.
