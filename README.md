<img src="assets/lust.svg" width="96" alt="Lust logo">

# Lust

A small, strict language for apps, games, tools and scripting. Checks are written as questions, states are built in, and every file ends with `end`.

<img src="assets/icons/lst.svg" width="28" alt="Lt"> programs (`.lst`) &nbsp; <img src="assets/icons/lsc.svg" width="28" alt="Ls"> LustScript (`.lsc`) &nbsp; <img src="assets/icons/lql.svg" width="28" alt="Lq"> LustQL (`.lql`)

```
state Mood = [Cheerful, Grumpy];

class Player {
    has {
        string name = "Hero";
        int hp = 20;
        Mood mood = Mood.Cheerful;
    }

    method hurt(int damage) {
        hp = hp - damage;

        is hp <= 0? then {
            print(name + " is knocked out!");
        } or is hp < 10? then {
            mood = Mood.Grumpy;
            print(name + " has " + hp.text() + " hp left.");
        }
    }
}

Player ada = new Player { name = "Ada" };
ada:hurt(12);

end
```

## Lust 2

This is **Lust 2**, a fresh start written in Rust. One language, two outputs:

| Output | Files | For |
| --- | --- | --- |
| Rust, compiled to real programs | `.lst` | apps, tools and servers on Windows, macOS and Linux |
| LustScript, turned into Lua | `.lsc` | scripting and modding inside other programs, like Garry's Mod, Aventail and Halberd |

**What works today (alpha 30):** both outputs. Every file is checked before anything runs, with mistakes explained in plain words.

- `.lsc` files turn into plain Lua 5.1 that Garry's Mod (LuaJIT) and Luau can run.
- `.lst` files turn into Rust, which builds into a real program (an `.exe` on Windows) that runs without Lust.
- Programs can be split into many files that share with `pass`, `fetch` and `get`.
- A standard library: text and array tools, plus `Maths`, `Files`, `Time` and `Json`.
- Saves, and databases on SQLite whose tables work like cMaps.
- Servers: `route` blocks answered by methods, with `schema`s for the data coming in, and `Web:get`/`Web:post` for asking other sites.
- Windows: an assembly with a `view` opens a window drawn entirely by Lust, with text, buttons, layout, styles, pieces of your own made from other assemblies, and inputs, checkboxes and sliders that change fields as you use them, scrolling, pictures and timers.
- Lust inside other programs: `lust build --library` makes a Rust library for a Rust program (like a game engine) to use, and `native` methods have their bodies in that program.
- `lust run` runs either kind straight away, with Lua built into `lust` itself, so nothing else needs installing. Building a real program needs Rust installed.

Every test runs both ways, through Lua and as a real program, and both have to print exactly the same.

[Ledger](https://github.com/Rhyslos/ledger), our editor, colours, checks, runs and builds Lust files.

The full rules are in **[LUST.md](LUST.md)**, and what gets built next is in **[ROADMAP.md](ROADMAP.md)**. Lust 1, the version that compiled to C#, lives on the [`v1`](https://github.com/Rhyslos/Lust/tree/v1) branch.

## Try it

You need [Rust](https://rustup.rs). Then:

```
cargo build --release
target/release/lust run tests/run/player.lst
target/release/lust build tests/run/player.lst
```

The second line makes `tests/run/player` (`player.exe` on Windows), a real program you can run on its own.

| Command | Does |
| --- | --- |
| `lust run file` | checks the file, then runs it straight away. `lust file` works too |
| `lust new name` | makes a project folder with a `main.lst`, and gets built programs ready |
| `lust new name app` | the same, starting with a window and a piece of it in `ui/` |
| `lust check file` | only checks, and lists any mistakes |
| `lust build file.lst` | builds a real program next to it, through Rust |
| `lust build file.lst --library` | makes a Rust library from it, for a Rust program (like a game engine) to use. See LUST.md section 21 |
| `lust build file.lsc` | writes the Lua next to it, as `file.lua`, ready for Garry's Mod or any Lua host |
| `lust lua file` | shows the Lua it turns into |
| `lust rust file.lst` | shows the Rust it turns into |
| `lust test tests` | runs the test programs, both ways. `lust test tests/player.lst` runs one |

Mistakes come out all at once, before anything runs:

```
basics.lsc:3:1  Lust asks questions instead of if: is hp == 0? then { }
basics.lsc:6:23  can't add an int to a string. Use lives.text() to turn it into text.
classes.lsc:18:19  Dog has no field called nam. Did you mean name?
```

Mistakes that only show up while running point at the Lust line, in Lua and in real programs alike:

```
missing-row.lst:2  there's no row with id 7. The ids are 1, 2.
```

Every push builds a ready-made `lust.exe`: open the **Actions** tab, click the latest **Test** run, and download **lust-windows**.

## Writing Lua inside LustScript

Anything Lust doesn't cover yet can be written in Lua directly:

```
fetch lua;

method shout(string words) {
    lua {
        print(string.upper(words) .. "!")
    };
}

shout("hello");

end
```

## The name

Lust started as "Rust + Lua". Then it spent a year mostly being C#. Now it really is Rust and Lua.
