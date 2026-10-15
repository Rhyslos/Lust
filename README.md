<img src="assets/lust.svg" width="96" alt="Lust logo">

# Lust

A small, strict, slightly silly programming language for things that have a state, carry stuff, and change over time. Think text adventures, RPG systems and little simulations.

```
state = [idle, fighting, knocked_out];

class Player {
    has {
        string[name "Hero"];
        int[hp 20];
        state is idle;
    }

    method hurt(int damage) {
        hp = hp - damage;

        is hp <= 0? then {
            state is knocked_out;
            print(name + " is knocked out!");
        } or {
            print(name + " has " + hp.text() + " hp left");
        }
    }
}

Player ada = new Player { name "Ada" };
ada:hurt(7);

end
```

## What makes it Lust

- **Checks are questions.** No `if`, no `==`: `is hp 0? then { } or { }`, plus the one-liner `either alive? "ready" or "napping"`.
- **`state` is built in.** Every file can have one, and things just *are* in a state: `state is fighting;`.
- **cArrays.** A connected array is a tiny table with ids: perfect for inventories and shops.
- **One loop.** `loop` does forever, repeat, while and for-each.
- **Strict and friendly.** Types first, `;` everywhere, every file ends with `end`, and mistakes are caught before the program runs, with errors that say what to do.
- **`true` and `not true`.** There is no `false`.

The full rules are in **[LUST.md](LUST.md)**. Examples are in [`examples/`](examples).

## Try it

You need the [.NET SDK](https://dotnet.microsoft.com/download), version 8 or newer. On Windows, double-click **`build.cmd`**. It makes `bin\lust.exe`. Then play the example game:

```
bin\lust run examples\valley\main.lust
```

Or, without building first:

```
dotnet run --project compiler -- run examples/valley/main.lust
```

Every push also builds a ready-made `lust.exe`: open the **Actions** tab, click the latest **Test** run, and download **lust-windows**.

| Command | Does |
| --- | --- |
| `lust run file.lust` | checks the file (and every file it fetches), then runs it. `lust file.lust` works too |
| `lust check file.lust` | only checks, and lists any mistakes |
| `lust build file.lust` | makes a program you can start without Lust, in a `build` folder |
| `lust cs file.lust` | shows the C# Lust writes |
| `lust test tests` | runs the test programs |

Mistakes come out as `file:line:column  message`, all at once, before anything runs:

```
oops.lust:2:15  can't add an int to a string. Use age.text() to turn age into text.
oops.lust:3:16  missing ; at the end of this line.
oops.lust:8:1  Lust asks questions instead of if: is hp 0? then { }
```

Mistakes that only show up while running stop the program with the line they happened on:

```
shop.lust:21  there's no row with id 7. The ids are 1, 2, 3.
```

## Examples

| File | Shows |
| --- | --- |
| [`examples/hello.lust`](examples/hello.lust) | variables, questions, loops, `random`, `wait` |
| [`examples/player.lust`](examples/player.lust) | classes, an object, a cArray inventory, states, `get` from [`dice.lust`](examples/dice.lust) |
| [`examples/valley/`](examples/valley) | **Lust Valley**, a small farming game in three files: buy seeds, water, sleep, harvest, sell. Uses `like`, states, cArrays, arrays, `ask` and `fetch`. |
| [`examples/ui/counter.lust`](examples/ui/counter.lust) | the smallest window: a label and a button (Windows) |
| [`examples/ui/pet.lust`](examples/ui/pet.lust) | **a virtual pet** in a window: name it, feed it, play, and turn off the lights to sleep. Screens switch with a state, and its mood uses states inside states (Windows) |

## Status

Everything in [LUST.md](LUST.md) works: types, questions, `either`, `loop`, classes, objects, `like`, `do` and `give`, arrays, cArrays, states (including states inside states), `send` / `fetch` / `get`, `ask` for typed input, and windows with buttons through `fetch ui;`. The test programs run on Linux and Windows on every push, and on Windows the UI examples open their windows and click every button by themselves.

What's next:

1. Running `.lust` files and seeing their mistakes inside [Ledger](https://github.com/Rhyslos/Ledger).
2. The ideas under "Still open" in the rulebook, like functions as values and calling C# libraries.
3. More for windows: pictures, colours, and a timer for things that move.
4. A small 2D game engine, written in Lust.

## The name

Lust started as "Rust + Lua", then turned out to be mostly C# with a Lua accent. The name stuck because it's funny. It may get a new one before anyone else sees it.
