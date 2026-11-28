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

## Lust

Lust is written in Rust, and has one language with two outputs:

| Output | Files | For |
| --- | --- | --- |
| Rust, compiled to real programs | `.lst` | apps, tools and servers on Windows, macOS and Linux |
| LustScript, turned into Lua | `.lsc` | scripting and modding inside other programs, like Garry's Mod, Aventail and Halberd |

**What works today (alpha 101):** both outputs. Every file is checked before anything runs, with mistakes explained in plain words.

- `.lsc` files turn into plain Lua 5.1 that Garry's Mod (LuaJIT) and Luau can run.
- `.lst` files turn into Rust, which builds into a real program (an `.exe` on Windows) that runs without Lust.
- Programs can be split into many files that share with `pass`, `fetch` and `get`.
- A standard library: text and array tools (sums, averages, picking and shuffling too), plus `Maths` (with smooth noise), `Random`, `Files`, `Time` and `Json`.
- States whose options carry values: `state Pose = [Idle { speed = 0.0 }, Run { speed = 6.5 }];`, then `pose.speed` or `Pose.Run.speed`, with options inside options taking their outer option's values (LUST.md, section 8).
- Records: `record Loadout { Gun primary; int grenades = 2; }` groups a few simple values that are copied wherever they go, so changing one copy never changes another, and an engine can keep them in components and events (LUST.md, section 10).
- Events that count in a state: an `on` block's entry can be for some of a thing's options, as `Hit(entity by, float damage) in Battle { … }`, so the same event does different things in different options, and nothing at all in an option no entry covers (LUST.md, section 11).
- Methods that exist in some states: a method can have bodies for some of a thing's options, as `method fire() in Battle { … }`, and calling it in an option no body covers stops with `Problem.WrongState` (LUST.md, section 11).
- Allowed moves between options: `state Door = [Locked -> Shut, Shut -> [Open, Locked], Open -> Shut];` says which option may follow which, and a move the arrows don't allow stops with `Problem.WrongState` (LUST.md, section 8).
- Starting in a state: `new Hero in Phase.Battle { hp = 5 }` makes a thing in one of its options, the same as `new Hero { state = Phase.Battle, hp = 5 }` with the state up front (LUST.md, section 8).
- Watching states: run from [Ledger](https://github.com/Rhyslos/ledger), a program's things with a state of their own show in Ledger's States panel, named after their variables, with the option each is in and the last few changes, timed. `lust run` sends them only when Ledger asks (`LUST_STATES`), so programs run at full speed otherwise (LUST.md, section 21).
- Tweens: `tween hp to 0 over 0.5 [ease OutCubic];` glides a number, vector, colour or rotation to a new value, with the standard easing curves.
- Waiting: `wait 2;` pauses only the piece of work that waits, so windows never freeze, `start openGate();` runs a call alongside, `wait { is door.state == Door.Open? } or after 5 { … }` waits until things are true, with a time limit, and a `check` inside a `wait { }` waits until a state is one of its options, then runs that one's block. `start gate = openGate();` names the work, `cancel gate;` stops it, `gate.state` says how it's going (`Run.Waiting`, `Going`, `Done` or `Cancelled`), work started during a state is cancelled when the state ends, and a program whose runs could only wake each other stops with a message naming them. Asking a website (`Web:get`) and the computer's file boxes pause only their run too, and in a game, event handlers and started work wait across the engine's ticks, on the game's own time (LUST.md, section 11).
- Saves, and databases on SQLite whose tables work like cMaps.
- Servers: `route` blocks answered by methods, with `schema`s for the data coming in, and `Web:get`/`Web:post` for asking other sites.
- Windows: an assembly with a `view` opens a window drawn entirely by Lust, with text, buttons, layout, styles, pieces of your own made from other assemblies, and inputs, checkboxes, switches, radio buttons, number boxes, sliders, date pickers with a calendar and colour pickers that change fields as you use them, icons, tables and long lists that stay quick with 100,000 rows, right-click menus, rows dragged to new places, files dropped on the window, drawing with the mouse on a canvas, scrolling, pictures (SVG too) and timers. Programs can open second windows, go full screen, show toasts and the computer's own notifications, and live in the tray on Windows. Windows stay sharp at any screen scaling, take typing through input methods (Chinese, Japanese, Korean and more), and describe themselves to screen readers like Narrator and NVDA.
- Lust inside other programs: `lust build --library` makes a Rust library for a Rust program (like a game engine) to use, and `native` methods have their bodies in that program.
- Lust inside a game engine: `component`s with `update`, `event`s handled with `on`, vectors, rotations and entities, and the compiler itself as a Rust library, so an engine can check a game's Lust, run it as Lua (or Luau) and swap in edits while the game runs (LUST.md, section 21).
- `lust run` runs either kind straight away, with Lua built into `lust` itself, so nothing else needs installing. Building a real program needs Rust installed.

Every test runs both ways, through Lua and as a real program, and both have to print exactly the same.

[Ledger](https://github.com/Rhyslos/ledger), our editor, colours, checks, runs and builds Lust files.

The full rules are in **[LUST.md](LUST.md)**, and what gets built next is in **[ROADMAP.md](ROADMAP.md)**. An engine using Lust (Solis, say) reads **[ENGINES.md](ENGINES.md)** when it moves to a newer Lust: what changed for engines, version by version, and what to do. Lust 1, the version that compiled to C#, lives on the [`v1`](https://github.com/Rhyslos/Lust/tree/v1) branch.

## Install

On Windows, download **lust-setup.exe** from the newest [release](https://github.com/Rhyslos/Lust/releases) and run it. It needs no administrator: it puts Lust in your own programs folder and adds it to your PATH, so `lust` works in any new terminal. Lust is still a test version (alpha), so things can change between releases.

- **Updates:** `lust update` gets the newest Lust, after checking the download against the release's checksums. `lust update check` only looks. When you run, build or check a file, Lust says at most once a day when a newer one is out; `lust update off` stops that.
- **Building real programs** (`lust build`) also needs [Rust](https://rustup.rs). `lust run` doesn't. The first `lust build` without Rust says how to get it. The installer also gets Lust's libraries for built programs ready, downloading them ready-made (about 150 MB), so even the first build takes seconds, not minutes; after that a build takes about a second. Lust builds with Rust 1.97.0, which rustup gets by itself the first time.
- **Uninstalling:** Windows' Settings → Apps → Lust. It takes Lust off your PATH too.

A new release comes out by itself whenever a new version of Lust reaches `main`.

## Try it from the source

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
| `lust new name` | makes a project folder with a `main.lst`, straight away (the first time on a computer, built programs get ready in the background) |
| `lust new name app` | the same, starting with a window and a piece of it in `ui/` |
| `lust check file` | only checks, and lists any mistakes |
| `lust build file.lst` | builds a real program next to it, through Rust |
| `lust build file.lst --release` | the same, taking longer (about 20 seconds) to make the fastest program, for sharing |
| `lust build file.lst --library` | makes a Rust library from it, for a Rust program (like a game engine) to use. See LUST.md section 21 |
| `lust build file.lsc` | writes the Lua next to it, as `file.lua`, ready for Garry's Mod or any Lua host |
| `lust lua file` | shows the Lua it turns into |
| `lust rust file.lst` | shows the Rust it turns into |
| `lust test tests` | runs the test programs, both ways. `lust test tests/player.lst` runs one |
| `lust update` | gets the newest Lust. `lust update check` only looks, `lust update off` stops the notes about new ones |

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

Every push also builds a ready-made `lust.exe`: open the **Actions** tab, click the latest **Test** run, and download **lust-windows**. Releases are the easier way to get it.

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

## Licence and privacy

Lust is free and open source, for ever, under the [MIT licence](LICENSE): use it, change it and share it, for anything, as long as the licence comes along. It comes with no warranty.

Views' icons are a subset of [Lucide](https://lucide.dev)'s, under the ISC licence (some from Feather, under the MIT licence); both are in [LICENSES/lucide.txt](LICENSES/lucide.txt).

Lust collects nothing about you. The only thing it does online is check GitHub for a newer Lust, at most once a day, and you can turn that off. [PRIVACY.md](PRIVACY.md) says exactly what it does.

## The name

Lust started as "Rust + Lua". Then it spent a year mostly being C#. Now it really is Rust and Lua.
