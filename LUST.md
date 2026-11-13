# The Lust v2 Rulebook

Lust is a small, strict language for apps, games and tools. It reads like a calm mix of C# structure and Lua's lightness. Checks are written as questions, states are built in and own their memory, and every file ends with `end`.

This is the single source of truth for Lust v2. Things still being designed are marked **(to design)** and gathered in the last section.

> **What works today (alpha 72):** both outputs. `.lsc` files are checked and turned into Lua. `.lst` files are checked and built into real programs through Rust (`lust build`), and `lust run` runs either kind straight away through Lua. Built so far: types (with vec3, vec2, rotation and colour), questions, `either`, maths (with bitwise operators) and text, arrays, maps, cMaps (with key-makers), states (with options inside options), classes, objects, `like`, methods, hooks with `new hook`, every kind of `loop`, `try`/`error`, `lua { }` blocks, the built-ins and the standard library (text and array tools, `Maths`, `Random`, `Files`, `Time` and `Json`, which holds vectors, rotations and colours too). Sharing between files (`pass`/`fetch`/`get`), things ending with their state (`keep`), nodes, assemblies, saves, databases and servers (`schema`/`route`) work too. Windows with `view` are built too: layout, styles, pieces of your own, inputs, text boxes, dropdowns, menus, dialogs, tips, dark mode, grids, splits, tabs, branches, progress bars, drawing, animation, tweens with easing, checkboxes, switches, radio buttons, number boxes, sliders, icons, tables and long lists, right-click menus, rows dragged to new places, dropped files, the mouse on a canvas, scrolling, pictures (SVG too), timers, keyboard shortcuts, fonts of your own, date and colour pickers, second windows, full screen, toasts, notifications and a tray icon, all drawn by Lust itself. Lust can also be part of a Rust program, with `native` methods and `lust build --library`. Not yet: LustQL as a language of its own. Lust v1 lives on the `v1` branch.

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

Plus **LustQL** (`.lql`) for stored data **(to design)**.

**Why Rust:** Lust writes readable Rust, and Rust's compiler does the rest: real machine code through LLVM, every desktop platform, and WebAssembly if it's ever wanted. Rust's libraries (crates) give Lust its standard library: files, HTTP, JSON, SQLite, sound, and windows and graphics through winit and wgpu. Lust checks everything itself, so nobody ever sees a Rust error. Programs need Rust installed to build, the way v1 needs .NET.

**The web:** Lust is for servers (`route`, `schema`). Pages in the browser stay HTML, CSS and JavaScript. Lust in the browser through WebAssembly is possible later, but not planned.

There is no .NET in v2, so no `fetch cs:`.

---

## 2. Files

- **Extensions:** `.lst` for programs, `.lsc` for LustScript, `.lql` for LustQL.
- **Every project has a `main.lst`** at its root, where the program starts.
- **Every statement ends with `;`.**
- **Every file ends with `end`** on its last line. Only blank lines and comments may follow.
- **Order inside a file:** imports (`fetch`, `get`), then states and nodes, then classes, objects, assemblies, schemas and methods, then `pass`, then the code that runs.
- **No dashes in names**, since `-` takes away: `int max-hp` is a mistake that suggests `maxHp`. Files and folders that other files `fetch` can't have dashes either (`fetch "my-parts/x"` is a mistake), so use `myParts` or `my_parts`. A project folder, or a file only ever run on its own, can be called anything.

### Comments

```
-- a comment, the Lua way
// also a comment
--[[ a block comment,
     over several lines ]]
```

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
| `T array` | a list of `T` | `int array` |
| `cMap(...)` | a table of rows, found by their key (the first column) | `cMap(int id, string itemName, int amount)` |
| `T map` | values found by a text key | `int map` |
| a state's name | one of that state's options | `GameState` |
| a schema's name | a data shape for the web | `UserPayload` |
| `hook ...` | a method kept as a value | `hook int(int n)` |

- A variable keeps its type forever.
- **`lock`** makes a variable keep the value it starts with: `lock int maxLives = 3;`. Giving it a new value (`=`, `+=`, `is`) is a mistake, caught before anything runs. A field can be locked too, in `has { lock int maxHp = 20; }`. It gets its value when the thing is made (its starting value, or one given in `new Player { maxHp = 30 }`) and keeps it.
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

- **Every option is covered, always.** A check has no "anything else": leave an option out and Lust says which one is missing. So when a state gets a new option, Lust points at every check that needs a `when` for it.
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
- A `{` straight before a name, a bracket, `#` or text starts a value. Any other `{` is just a brace, so JSON (`"{\"name\": \"Ada\"}"`) and patterns (`"[0-9]{4}"`) need nothing special.
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

- A variable holding a state option changes with `is`: `now is GameState.ActiveGame.Combat;` (or with `=`, like any variable).
- A class or object can have a state of its own, written `state is GameState.MainMenu;` in its `has`. Inside its methods, `state is GameState.ActiveGame.Combat;` changes it, and `state` reads it. Outside a class or object there's no `state` to change; use a variable.
- A question about an option includes everything inside it: during `Combat`, `is state == GameState.ActiveGame?` is yes, whichever side the option is written on. `===` asks for that very option (section 5).

### Things made in a state end with it

An object with a state of its own (`state is …;` in its `has`) owns what its methods make. Anything made with `new` in one of its methods belongs to the option the object is in at that moment, and **ends when the object leaves that option**:

```
class Hero {
    has {
        state is Phase.Calm;
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
- Options inside options count separately: something made during `Battle.Melee` ends when the object moves on to `Battle.Ranged`.
- Starting values in `has`, and anything made by code outside a class's methods, belong to no state.
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
class Dog {
    has {
        string name = "Rex";
        int hp = 10;
        state is Happy;
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
- Vectors, rotations and colours are saved as they're written in JSON (section 19, Json), so loading gives back exactly the same value, to the last bit.
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
- Programs with a database are built with Cargo, Rust's builder, which gets SQLite. The first build on a computer takes a minute or two. After that it's quick.
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
- **Icons:** `icon "trash"`, from a built-in set of about 150, on their own or in buttons (see below).
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
| `choose cat.png`, `choose cancel` | the answer the next `Files:chooseOpen`, `chooseSave` or `chooseFolder` box gives |
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
| `wait 2.5` | lets 2.5 seconds go by on the test's own clock, running the timers that come due |
| `size 800 600` | resizes the window |
| `snapshot shot.png` | draws the window into a picture |
| `in Settings click Save`, `in "My settings" dump` | sends a command to the second window with that title (in quotes when it has spaces) instead of the main one |
| `close`, `in Settings close` | clicks the window's X: the main window runs its `close { }` (or ends), and a second window's bool becomes false |
| `tray click`, `tray item Quit` | clicks the tray icon, or chooses one of its items |

`lust test` sets it for every test, so a view's tests compare what `dump` prints. A table or list dumps only the rows on the screen, after a line saying how many it has and which show: `table 100000 rows, showing 41–51`. `dump` shows each open window under a heading of its own, `window "Settings" 300x200`, with `(fullscreen)` or `(hidden)` after the main window's when it is; the toasts showing, as `toast "Saved"` in a `layer "toast"`; and the tray, as `tray "Notes" (2 items)`. `App:notify("Title", "Body")` prints `notify: Title — Body` instead of showing anything. `wait` moves the toasts along too, and every window's timers.

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

`icon "trash";` draws a small picture from Lust's own set, in the text's colour, sharp at any size. `size` sets how big it is (20 unless told), and a colour word colours it:

```
row [gap 8] {
    icon "star" [size 18, yellow6];
    text "Favourites";
}
button "Delete" [icon "trash", red] { remove(); }
button [icon "settings", tip "Settings"] { showSettings = true; }
```

On a button, `icon` in its `[ ]` puts the icon before the words, or on its own when the button has no words. An icon's name is usually written in place, and then a name Lust doesn't have is a mistake before running, with the nearest name it does have; a name worked out while running that isn't one shows as an empty grey box.

The icons are a subset of [Lucide](https://lucide.dev) (ISC licence, see `LICENSES/lucide.txt`), drawn from their shapes by Lust itself. These are the names:

| | |
| --- | --- |
| arrows | `arrow-up`, `arrow-down`, `arrow-left`, `arrow-right`, `arrow-up-right`, `arrow-up-down`, `arrow-left-right`, `undo`, `redo`, `corner-down-left`, `move`, `maximize`, `minimize`, `external-link`, `log-in`, `log-out` |
| chevrons | `chevron-up`, `chevron-down`, `chevron-left`, `chevron-right`, `chevrons-up`, `chevrons-down`, `chevrons-left`, `chevrons-right` |
| marks | `check`, `x`, `plus`, `minus`, `circle-check`, `circle-x`, `circle-plus`, `circle-minus`, `circle-alert`, `circle-help`, `info`, `triangle-alert`, `ban`, `circle`, `square` |
| editing | `trash`, `trash-2`, `pencil`, `square-pen`, `scissors`, `copy`, `clipboard`, `paperclip`, `bold`, `italic`, `type`, `align-left` |
| finding your way | `house`, `menu`, `ellipsis`, `ellipsis-vertical`, `search`, `zoom-in`, `zoom-out`, `settings`, `funnel`, `list-filter`, `list`, `layout-grid`, `panel-left`, `grip-vertical` |
| people and files | `user`, `users`, `folder`, `folder-open`, `file`, `file-text`, `save`, `download`, `upload`, `cloud`, `archive`, `inbox`, `printer`, `share`, `link` |
| media | `play`, `pause`, `skip-back`, `skip-forward`, `volume-2`, `volume-x`, `mic`, `camera`, `image`, `video`, `music` |
| state | `refresh-cw`, `refresh-ccw`, `rotate-cw`, `rotate-ccw`, `loader`, `eye`, `eye-off`, `lock`, `lock-open`, `key`, `shield`, `shield-check`, `bell`, `star`, `heart`, `thumbs-up`, `bookmark`, `flag`, `tag`, `map-pin`, `globe`, `wifi`, `zap`, `power` |
| time and talking | `calendar`, `clock`, `timer`, `history`, `mail`, `send`, `message-circle`, `phone` |
| things | `sun`, `moon`, `gift`, `shopping-cart`, `chart-column`, `chart-line`, `activity`, `code`, `terminal`, `database`, `monitor`, `gamepad-2`, `trophy`, `sword`, `coins`, `rocket`, `palette`, `book`, `bug`, `wrench`, `lightbulb`, `layers`, `box`, `puzzle` |
| other names | `edit` (`square-pen`), `home` (`house`), `unlock` (`lock-open`), `more-horizontal` (`ellipsis`), `more-vertical` (`ellipsis-vertical`), `alert-triangle` (`triangle-alert`), `alert-circle` (`circle-alert`), `help-circle` and `help` (`circle-help`), `check-circle`, `x-circle`, `plus-circle` and `minus-circle` (`circle-…`), `filter` (`funnel`), `refresh` (`refresh-cw`), `sort` (`arrow-up-down`), `stop` (`square`), `close` (`x`), `delete` (`trash-2`) |

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
            button [icon "settings", tip "Settings"] { showingSettings = true; }
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
    toast "Saved" [green, icon "check"];
}
button "Delete" { toast "Can't delete a note that's open" [red, icon "triangle-alert", seconds 8]; }
```

- **How it looks:** white words on dark grey, or on its colour with a colour word, like a button: `[green]`, `[red]`. `icon "check"` puts one of Lust's icons before the words, and `seconds 8` keeps it for 8 seconds (more than 0).
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
    tray [icon "bell", tip "Notes"] {
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

- **The icon** is one of Lust's icons, drawn white on a blue square: `icon "bell"`. `picture "tray.png"` uses a picture file instead, found the way `image` finds files. `tip` is the words shown when the mouse rests on it.
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

`key "ctrl+s" { save(); }` is a keyboard shortcut: its block runs when those keys are pressed anywhere in the window. Like `every`, it takes no room, and it works only while it's in the view, so a shortcut inside a question is there only some of the time:

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
        button [icon "trash", subtle] { forget(p); }
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

`image "cat.png";` shows a PNG, JPEG or SVG file, found the way `Files` finds files: from the folder the program runs in, or anywhere with its whole path. Without a size it's as big as its file, shrinking to fit if there isn't room. Give it a `width` or a `height` and the other follows, keeping its shape. Give both and it fits inside that box; add `cover` and it fills the box instead, cut to fit. `round` rounds its corners, so `[width 48, height 48, cover, round 24]` makes a round picture. A file that isn't there shows as a grey box.

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
- **In a plain program**, `wait seconds;` lets the clock run: tweens move on during it, in steps of 1/60 s, so `tween x to 10 over 1; wait 1;` leaves `x` at 10 with its done block run. Without a `wait`, nothing moves. When the program ends, tweens still running just stop. A `wait` inside a done block, or in a button while the window is open, only pauses.
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
| in dark mode | `dark:[ … ]` |
| windows | `title`, `width`, `height`, `minWidth`, `minHeight`, `background`, `font`, `dark`, `fullscreen`, `visible` (the main window), `escape` (a second window) |
| toasts | a colour word, `icon "check"`, `seconds 8` |
| the tray | `icon "bell"`, `picture "tray.png"`, `tip "Notes"` |
| inputs and pictures | `placeholder "…"`, `from`, `to`, `step` (sliders), `min`, `max`, `step` (number boxes), `min`, `max` (date pickers, as dates), `alpha`, `swatches [ … ]` (colour pickers), `across` (radios), `cover` |
| icons | `icon "trash"` on a button |
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

Lust draws everything itself, from the pixels up: layout, every piece, text editing, focus, scrolling, styles and animation. Small permissive libraries only open the window (winit), hand over the pixels (softbuffer), read font files (ab_glyph), show the computer's own boxes for choosing files (rfd), reach the system clipboard (arboard), read PNG and JPEG files (png, jpeg-decoder) and draw SVG files (resvg). On Windows, tray-icon puts the tray icon in the notification area, and tauri-winrt-notification shows notifications; on Linux, zbus sends notifications over D-Bus. Icons are Lucide's shapes, drawn by Lust. Views are for `.lst` programs on Windows, macOS and Linux. Web pages stay HTML, CSS and JavaScript.

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
- **`Web:fail(404, "there's no user 9.");`** stops and answers with that status, as `{"error": "…"}`. A path value that isn't a number, or a body that doesn't fit, answers 400 by itself. Any other error answers 500 with its message.
- A path nobody answers gives 404. One that's answered, but not for that kind of request, gives 405.
- **`Web:serve(8080);`** answers requests on that port until the program is stopped. Every file's routes are served. `pass route;` is fine to write but not needed.
- **`Web:request("GET", "/api/users/2", "")`** tries a route without a network, and gives back the status and the answer as text: `200 {"id":2,"name":"Bo"}`. Good for testing.
- Requests are read and answered several at once, so a slow visitor doesn't hold up the others. The methods that answer run one at a time, so they never get in each other's way.

### Asking other websites

```
string answer = Web:get("https://api.example.com/weather?city=Oslo");
Weather w = Json:read(answer);
string reply = Web:post("https://example.com/scores", Json:write(score));
```

- Both give back the answer's text. Turn JSON into Lust values with `Json:read`.
- If the site can't be reached, or answers with an error (400 and up), the program stops with what happened, like `https://example.com/x answered 404: …`. Use `try` to carry on.
- A program that asks websites is built with Cargo, like ones with databases. In LustScript, it needs `lust run`.

---

## 19. Built-ins and the standard library

| Built-in | Does |
| --- | --- |
| `print(x);` | shows `x` on its own line. Any type: numbers, text, bools, lists, vectors, colours, states and `nothing` |
| `print(a, b, c);` | shows several values on one line, with a space between each: `print("Score:", score, "lives:", lives);` |
| `wait seconds;` | pauses, while tweens move on (section 17, Tweens) |
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
| `scores:has(10)`, `scores:find(10)` | whether it's there, and where (or -1). For arrays of numbers, text and bools |
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
| `Time:weekday(date)` | `"Monday"` to `"Sunday"` |
| `Time:isDate(text)` | true if it's a real date, written like `"2026-10-21"` |

**Json** turns values into JSON text and back:

```
string text = Json:write(player);     -- {"name":"Ada","hp":20}
string nice = Json:pretty(player);    -- the same, over several lines
Player back = Json:read(text);
```

- JSON holds numbers, text, bools, state options (as their names), vectors, rotations and colours (below), arrays, maps (as JSON objects), cMaps (as arrays of objects) and things made from classes (as objects of their fields). Hooks stay out.
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

The `choose` boxes are the ones the computer itself has (Windows', macOS', or the desktop's on Linux), and the program waits while one is open. In a view's tests no box opens: `choose pictures/cat.png` in the test's input gives the answer the next box gives, and `choose cancel` cancels it.

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
- **Failures come back as a `LustError`.** Everything gives back a `Result`, and a Lust failure says where it happened: `file`, `line` and `column` (of the statement that failed), and `message`. Shown with `{}`, it's the message Lust prints: `game.lst:24  this is nothing, so it has no fields or methods.` The program decides what to do with it.
- **`game::LUST_VERSION`** is the version of Lust that wrote the library.
- **No unsafe code.** A library starts with `#![forbid(unsafe_code)]`, so Rust itself checks it has none. The one thing that needed it, the computer's time zone for `Time:date()` and `Time:time()`, comes from the program instead: when the Lust uses either, `Host` has `local_time_offset(&self, unix_seconds: i64) -> i64`, the seconds the local time is ahead of UTC (0 for UTC). A program of its own (`lust build`) still asks the computer.
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
- **`lust::check_with(&engine, entry, &files)`**, **`to_lua_with`** and **`to_rust_library_with`** do the same inside an engine: `lust::Engine { folders: vec!["solis".into()] }` names its host folders, whose files come with every program and may give `entity` methods. The calls without `_with` are these with no host folders.
- **Mistakes:** every one found, in order, as a `Problem` with `file`, `line`, `column` and `message`. Shown with `{}`, it's what `lust check` prints: `parts/gun.lst:12:5  ammo is an int, but this is text.`
- **Deep code is a mistake, not a crash.** Code nested more than 150 levels deep (brackets, blocks, or operators in a row like `a + b + c …`) is reported as a `Problem` like any other, so Lua and Rust can always take what Lust accepts. The compiler runs on a thread of its own with room to spare (64 MB of stack, reserved but not used), so it never runs the engine out of stack, whatever it's given.
- **`lust::VERSION`** is the compiler's version, so an engine can show it and refuse one it doesn't expect.
- **`to_rust_library`** gives what `lust build --library` puts in `src/lib.rs`.

**The Lua** (`chunk.code`) runs in Lua 5.1, LuaJIT or Luau. Load it with the name `lust::LuaChunk::NAME` (in mlua: `.set_name(lust::LuaChunk::NAME)`), so a failure says where it came from. The engine gives native methods' bodies, and `send_event(to, name, values)`, in the table `LUST_NATIVES` first. Running the chunk runs the program's own start, and gives back a table:

- **`version`**, **`components`** (by name) and **`events`** (by name: what each carries, as `{ key, name, kind }` lists, kinds being `bool`, `int`, `float`, `string`, `state`, `vec3`, `rotation` and `entity`).
- **`new(name, entity)`** makes a component with its starting values, on an entity. The engine calls its `update(dt)` each tick, and `handle(event, values)` to deliver an event (it gives back whether the component handles it).
- **`swap(component)`**, on the table from new code, gives a component made by an earlier chunk the new code, in place, so the engine's hold on it stays good. It follows the same rule as `from_data`: a field kept by name keeps its value; a field whose kind changed, or a new one, takes its starting value; a removed field is dropped; the entity it's on stays. It gives back the component and notes on what started over (`Gun's ammo is float now, not int, so it starts at 10.`), or `nil` and a note if the component isn't in the code any more.
- **`changes(old)`**, given the earlier chunk's table, lists the components whose fields `changed`, and those `removed` and `added`, so an inspector knows what to show again.
- The code outside classes and methods runs again with each new chunk, and objects start over.

**Vectors and rotations:** in Luau, a `vec3` is a Luau `vector` (`vector.create(x, y, z)`; mlua's `Vector`): in components' fields, in what events carry, and in what native methods are given and give back. Luau keeps a vector's numbers as f32s, as Lust does. A `rotation` is a table, `{ x, y, z, w }`. In plain Lua, which has no vectors, a `vec3` is a table, `{ x, y, z }`, and the chunk's `vectors` says which: `"luau"` or `"table"`. A native method's body may give back a table for a vec3 in Luau too, and Lust turns it into a vector. Lust never changes a rotation in place, so it shares them; an engine setting one gives a new table, and never changes one it got from Lust. A `vec2` is a table `{ x, y }` and a `colour` a table `{ r, g, b, a }`, in Luau as in plain Lua (Luau's vectors have three numbers, and a table keeps a vec2 apart from a vec3); their kinds in a schema are `"vec2"` and `"colour"`. They're shared the same way, so an engine never changes one it got from Lust either.

**Speed:** in Luau, Lust's vector maths is Luau's own, and it rounds numbers to f32 with Luau's vectors, which is many times faster than plain Lua can; it gives exactly the numbers Rust gives. Luau's native code (mlua's `luau-jit` feature) makes it faster again. `bench/` in Lust's own folder ticks 2,000 components both ways, and checks Rust and Luau end with exactly the same values.

**Fields the engine fills in:** in a component's schema their kind is `object`, followed by the class's name (`{ "anim", "anim", "object", "FarmerAnimator" }`). The chunk's **`make(name, values)`** makes a thing of one of the engine's classes with these fields (`api.make("FarmerAnimator", { id = 3 })`), and the engine sets the component's field to it (`farmer.anim = …`). Built as a library, a component's `FILLED` lists them with their classes, and each has its setter (`farmer.set_anim(Some(animator))`).

**Notes:** `chunk.notes` lists what's worth showing but isn't a mistake (a component still writing `me`), each a `Problem` with its file, line and column; the chunk runs either way.

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

- **Fixed values** are numbers (negative ones too), text without `{ }`, bools, `nothing`, a state's option (`Tileset.Farm`), plain lists of fixed values (`[40, 36]`, a vec3 too), colours written as a colour word (`orange5`) or `Colour:hex("#3a8f2a")` with the text right there, and things made with `new` from fixed values. Anything that would need running (a call, a sum, a variable, a class or a method, `Rotation:toward(…)` too) is a mistake, with its line.
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

- **`this`** is the entity the component is on. Every component has it, and it can't be changed. Hand it over whole (`send Hit(this, 6) to target;`), or ask it for an ability (`this:jump()`). It was written `me` before; `me` still works for now, with a note saying to write `this` (from `lust check`, `lust run` and the others, and in a Lua chunk's `notes`; a Rust library has nowhere to put notes), and becomes a mistake once Solis has moved over. Outside a component, `this` is a mistake.
- **Abilities are called bare.** Inside a component, the methods the engine gives `entity` (above) are the component's own abilities: `moveTo(position() + [1, 0, 0])` is `this:moveTo(this:position() + …)`. A variable can share an ability's name (`float velocity = velocity();`). The component's own methods come first, then methods brought in with `get`, then the abilities: so a component's own `jump()` wins over the engine's, and `this:jump()` still reaches the engine's.
- **Fields** hold what the engine can keep and show: `bool`, `int`, `float`, `string`, a state (shown as a choice of its options), `vec3`, `vec2`, `rotation`, `colour` and `entity`. A colour field can start as a colour word: `colour tint = blue1;`.
- **Things the engine made for it.** A component can hold a thing of one of the engine's own classes (from its host folders), such as the animator its editor made: `FarmerAnimator anim;`, with **no starting value**. The engine fills it in before the first `update`; reading it before then stops on its line, naming the field: "anim is filled in by the engine before the component's first update, and hasn't been yet." A class of the game's own can't be a component's field (keep those in a class or an object), and one of the engine's can't be given a starting value.
- **`update(float dt)`** runs once a tick, with how long a tick is. The engine runs the entities in order of their IDs, and an entity's components in order of their names, so a game runs the same way every time.
- **`entity`** is a value the engine hands over: an entity, or `nothing`. Entities can be compared with `==`, and `.text()` gives the 13 letters and digits Solis writes them as. Lust can't make one; it gets them from the engine's native methods and from `this`.
- A component can't be `like` another one yet.
- A component's methods can't be called `handle`, `schema`, `toData`, `setData`, `fromData` or `setMe`: the engine uses those on every component.

Built as a library, each component also has what the engine needs to keep, show and run it, in the same kinds as Solis's own (`reflect`):

- `Gun::NAME`, and `Gun::schema()`: its fields, their kinds and their starting values, in the order they're written. `game::components()` lists every component's.
- `gun.to_data()` gives its fields' values by name, as a `Data` (a `BTreeMap<String, Value>`), and `Gun::from_data(&data)` makes one from values. A value that's missing, or of the wrong kind, keeps the starting value.
- `gun.set_data(&data)` changes only the fields `data` has, so an edit made in the editor while the game runs keeps the rest, like ammo left and cooldowns.
- `gun.set_me(Some(id))` puts it on an entity, and `gun.update(dt)` runs a tick. An entity is its 64-bit ID, `Option<u64>`, with `None` for nothing.
- `Value` is `Bool`, `Int`, `Float`, `Text`, `List` (a vec3's three numbers, a vec2's two, a rotation's four, a colour's four from 0 to 1) and `Entity`, and `FieldKind` is `Bool`, `Int`, `Float`, `Text`, `Choice` (with the options), `Vec3`, `Vec2`, `Rotation`, `Colour` and `Entity`. A colour from `set_data` has its parts kept between 0 and 1.

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
- **Removing an entity** doesn't stop its tweens: Lust has no way to hear that a component has gone yet. They keep moving the component nobody ticks until they end, and their done blocks still run (a `send` from one goes to an entity the engine no longer has, which it should ignore).
- `tickTweens` and `tick_tweens` with 0 move nothing. A failure in a done block comes back as from `update`.

### Events

Components tell each other things with events. An event says what it carries; a component handles it with `on`, and anything sends it to an entity with `send`:

```
event Hit(entity by, float damage);
event Died();

component Health {
    has {
        float hp = 10;
    }

    on Hit(entity by, float damage) {
        hp = hp - damage;
        is hp <= 0? then {
            send Died() to this;
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

- **`event Name(…)`** says what an event carries, in order. It carries the same kinds a component's fields hold: `bool`, `int`, `float`, `string`, a state, `vec3`, `vec2`, `rotation`, `colour` and `entity`.
- **`on Name(…) { … }`** in a component runs when the event reaches the entity it's on. It takes what the event carries, in the same order (the names can be its own). Only components handle events, and each one handles an event once.
- **`send Name(…) to target;`** hands the event to the engine, which queues it. Nothing runs straight away: the engine delivers the queue in order, to each component on the entity in order of their names. Events sent while delivering (Hit sends Damaged, which sends Died) join the end of the same tick's queue, so a game runs the same way every time.
- The engine knows events by their names, so two files can't each have an event with the same name. A program with `send` needs the engine, so like native methods it's built with `lust build --library`; `lust run` on its own stops at the first `send`.

Built as a library:

- The engine's `Host` has `send_event(&self, to: Option<u64>, event: &str, values: Vec<Value>)`, which `send` calls.
- Each component has `HANDLES`, the events it handles, and `handle(&self, event, &values)`, which runs its handler: `Ok(false)` if it has none for that event, and an error if the values are the wrong count or kind.
- `game::events()` lists every event and what it carries, as `EventSchema { name, values }`.

In LustScript, `send` calls the host's Lua function `send_event(to, name, values)`, with the values in a list (its count in `n`), and a component's `handle(event, values)` runs its handler.

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

Neither can the built-in objects: `Maths`, `Files`, `Time`, `Json`, `Web`, `Random`, `Rotation`, `Colour` and `Ease` (which is a state of easing curves too). (`Rotation` and `Colour` with a capital are the objects; `rotation` and `colour` are the types.)

`tween`, `to` and `over` are words only in a tween (and `to` after `send`), so they can still be names. So is `toast`, a word only at the start of a statement, and `App`, which a program's own assembly can be called.

The pieces of a view (`text`, `button`, `radio`, `switch`, `number`, `icon`, `table`, `list` and the rest) are words only at the start of a line inside a view, so they can still be names: `int number = 3;` works anywhere.

---

## 23. Still to design

- **LustQL (`.lql`):** a query language of its own, for later. Queries are written in Lust code for now (section 14).
- **Ready-made host files:** the files describing Garry's Mod's or Aventail's functions as native methods (section 21), so LustScript there is checked like the rest of Lust.
- **More for engines** (Solis): vectors as real value types, typed events, leaner Rust for code that runs every frame, and rebuilding a game's Lust in a couple of seconds.
- **The standard library:** which Rust crates Lust wraps for files, networking, JSON, dates and sound, and how they look from Lust.
- **From v1:** whether `with x { }`, `{ }` inside text and the word `when` come along.
