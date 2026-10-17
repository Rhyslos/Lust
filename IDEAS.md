# Ideas for later

Lust's long-term plan: everything we've talked about but haven't built yet. Some ideas are decided in direction (marked as such), but each one still gets designed together in detail before it's built, the way saves were. The rules that are real today live in [LUST.md](LUST.md).

## What Lust is aiming for

- **Easy to learn.** A beginner only needs `print`, `is … ?`, `loop` and variables. Everything else is found later, never needed on day one. Every feature should read out loud and be guessable, and come with friendly error messages.
- **State-driven.** States are Lust's big idea. Things are described by what state they're in and what happens on the way in and out.
- **Powerful through libraries.** The core stays small. Power comes from C# (`fetch cs:`) and from libraries written to feel like Lust, the way `ui` wraps WinForms.
- **Watch for feature creep.** Features that make Lust more *itself* (reacting states) come first. Features that only make it more like C# (generics) come last, or never if C# can do the job.

## The long-term plan

The order we'd build things in, so Lust grows into something real without biting off too much at once:

1. **Native programs:** `lust build --native`. Small, and useful straight away.
2. **`fetch web;`:** servers with routes and JSON. Useful straight away, and it's familiar ground.
3. **LustScript:** Lust turned into Lua, to run inside other programs: Garry's Mod and Aventail first.
4. **`view { }`:** describing screens Compose-style, with Tailwind-style `[ ]` styles, drawn by Lust itself in a window.
5. **The same views on the web,** over a live connection.
6. **The 2D engine,** on the same drawing layer as the views, then a sprite-placing editor written in Lust.
7. **Our own machine code (LLVM),** only if .NET is ever what's holding Lust back.

`view { }` is the heart of it: one way of describing screens serves desktop windows, web pages and the engine's editor. It's worth designing carefully before any of steps 4 to 6 are built.

The smaller language additions below (hooks, `with`, `{ }` in text, `when`, LustQL, `+=`) can slot in between any of these steps.

## How big it is

Rough estimates, measured against today's Lust: about 5,500 lines of compiler plus 2,000 lines of runtime, so 7,500 in all.

| Plan | Rough size | How hard |
| --- | --- | --- |
| Native builds | 100–300 lines | Small: one pull request |
| Language additions (hooks, `with`, `{ }` in text, `when`, `+=`, `assemble`) | 1,500–2,500 lines together | Medium: each is small, `assemble` needs the most design |
| LustQL | 800–1,500 lines | Medium: SQLite does the hard part |
| `fetch web;` | 1,000–2,000 lines | Medium: ASP.NET does the hard part |
| LustScript | 1,500–2,000 lines, plus 500–1,500 per host program | Medium to large |
| `view { }` and Lust drawing its own UI | 2,000 for the language, 10,000–20,000+ for the renderer | The big one: text editing, scrolling, fonts and high-DPI take the time |
| Views on the web | 3,000–5,000 lines | Large |
| 2D engine and sprite editor | 3,000–6,000, plus 2,000–5,000 for the editor (in Lust) | Large, and reuses the renderer |
| Ledger tooling (autocomplete, hover, stepping through Lust) | 2,000–4,000 lines | Medium |
| Our own machine code (LLVM) | 20,000+ lines | Huge: a second compiler. Only if ever needed |

Without LLVM that's roughly 30,000–50,000 lines, 4 to 7 times today's Lust. The pace is set less by writing code than by designing each feature, testing it on Windows and reviewing it. As a guide: native builds and the small additions take a few sessions each, LustQL, `fetch web;` and LustScript a week or two of sessions each, and the renderer months of on-and-off work.

## The language

### Decided directions

Talked through and liked, but the exact syntax still needs working out:

- **`assemble`:** groups things together the way `has` groups fields, and reads like a class. It builds one named thing, or a collection of named things that Lust remembers, so you can make a whole scene or a set of shop items in one place.
- **`with x { … }`:** "here's the thing, and here's what I attach to it". Sets several fields without repeating the name:

  ```
  with player {
      hp 20;
      gold 5;
      state is idle;
  }
  ```

- **`{ }` inside text:** `"Ada has {hp} hp"` instead of `"Ada has " + hp.text() + " hp"`.
- **`when` as a simple switch:**

  ```
  when weather {
      sunny then { print("Water the crops."); }
      rainy then { print("Stay in."); }
      or { print("Just another day."); }
  }
  ```

  This and "States that react" below both want the word `when`, so one of them needs to give way.
- **LustQL: a cArray that lives in SQLite.** Practical, not a second language to learn:

  ```
  cArray(string title, truth done) notes in "notes.db";

  loop (int id, string title, truth done in notes where done not true) {
      print(title);
  }
  ```

  Adding and removing rows works like any cArray, and the file is kept up to date by itself.

### Hooks: functions kept as values

Decided. `hook` replaces `do` as the type of a function kept in a variable, field, array or map, so `do` only ever means "declare a function". `new hook { }` makes one right where it's needed.

```
hook int(int n) triple = new hook {
    give n * 3;
};

class Shop {
    has {
        hook(string item, int price) [onSell nothing];
        hook() [onClose nothing];
    }
}

Shop.onSell = new hook {
    print("Sold " + item + " for " + price.text());
};

hook int roll = new hook { give random(1, 6); };   // takes nothing: no ( )
hook int(int) f = double;                           // an existing function works as before
```

- **The type is written once, where the hook is declared**, names included. `new hook { }` is just the body: its values and what it gives back come from where it's going, since once declared it can't be anything else.
- **Names in a hook's type now matter.** Today they're allowed and ignored (`do(string who)` is the same as `do(string)`). With hooks, they're the names the body uses. A type without names (`hook(string)`) still works for existing functions, but a `new hook` for it asks for them: "name what the hook takes, like hook(string item)".
- **A `new hook` can see the variables around it**, unlike a `do` function. So `loop (Loaf l in shelf) { l.onBurn = new hook { print(l.name + " is burning!"); }; }` works.
- **No two things with one name:** a name from the hook's type can't be a variable that already exists where the `new hook` is written.
- **`hook` becomes a reserved word.**
- This also settles "functions written in place" from LUST.md's still-open list, and lets a button say what it does on the spot: `button.onClick = new hook { … };`.

### States that react

The biggest idea: states that do things, not just hold a value. Sketches only:

```
when fighting { print("Shields up!"); }       // runs on entering fighting
when leaving fighting { hp = hp + 5; }        // runs on leaving it
state is stunned for 2;                       // goes back to before after 2 seconds
when hp <= 0 { state is dead; }               // a rule that watches by itself
```

To work out: how `when` fits with `tick()`, what happens with states inside states (entering `fighting.blocking` also enters `fighting`?), and the order rules run in.

### Every object runs on its own

Each object gets its own `tick()`, and `wait` pauses just that object while the rest carry on. LUST.md already promises this for the engine.

### Smaller additions

- **Vector maths with operators:** `pos + vel * dt` instead of method calls. Needed for games.
- **C# events:** hooking a Lust function to "the song finished" or "the button was clicked".
- **Catching errors**, so one broken enemy or file doesn't end the whole program.
- **Interfaces and generics:** last, if ever.
- **Still open from LUST.md:** `+=`, fetching single methods, `nothing` for arrays, calling the replaced method from a `like` class.

## Screens: view { } and styles

Inspired by Jetpack Compose and Tailwind. **The screen is a description of your state.** Change a field or a state and Lust redraws what changed. There's no `rebuild()`, and no need to keep widgets in fields to change them later.

```
fetch ui;

state clock = [stopped, running, paused];

object Timer like App {
    has {
        int[secondsLeft 1500];
        clock is stopped;
    }

    view {
        column [center, gap 12, pad 24] {
            text time() [size 28, weight 500];
            row [gap 8] {
                button "Start" [blue, round] { clock is running; }
                button "Pause" [subtle]      { clock is paused; }
            }
            is clock paused? then { text "Paused" [muted]; }
        }
    }
}

Timer:open();
end
```

- Buttons say what they do in a block, so there's no class per button.
- Styles go in `[ ]`, the same brackets as `int[hp 20]`: short words like `gap 12`, `pad 24`, `blue`, `round`. A style can be named once and reused: `style card = [pad 16, round, shadow];`.
- Questions and `loop` work inside a view. Switching screens is just a question, and a list is a `loop` inside a `column`.
- Lust draws the views itself (with SDL or Skia underneath) instead of using WinForms. WinForms is boxy, Windows-only and can't be built natively. The same drawing layer becomes the 2D engine's.
- **To decide before building:** draw everything ourselves, or build `view { }` on an existing toolkit like **Avalonia** (a mature .NET UI library for Windows, macOS and Linux). You'd write exactly the same `view { }` and `[ ]` styles either way. Avalonia would handle text boxes, fonts and scrolling, cutting the biggest item in the plan from 10,000–20,000 lines to perhaps 3,000–5,000. The cost is less control over the look, and the 2D engine would need a drawing layer of its own.
- The current `ui` keeps working until views can do everything it does.

## The web: fetch web;

**Servers first.** A Lust program answers requests, with ASP.NET Core underneath. Things go out as JSON and come back in as Lust classes:

```
fetch web;

class Todo {
    has {
        string[title ""];
        truth[done not true];
    }
}

object Api like Server {
    has {
        int[port 8080];
        Todo array[todos []];
    }

    route get "/todos" {
        give todos;
    }

    route post "/todos" (Todo item) {
        todos:inject(item);
        give "added";
    }
}

Api:start();
end
```

With LustQL, `todos` can live in a SQLite file, and that's a real backend.

**Then views in the browser.** A `view { }` can become HTML just as well as a window, and the `[ ]` styles map almost word for word to CSS. The Lust code stays on the server, and the browser shows the page over a live connection (the way Blazor Server and Phoenix LiveView work). A click changes state on the server, and only what changed is sent back. No JavaScript, and no separate API to keep in step. Making a window app into a web app would be one line: `like WebApp` instead of `like App`.

**Maybe, later:** Lust running inside the browser through WebAssembly. It works with .NET, but costs several megabytes of download before anything shows, so it's only worth it for apps that must work offline.

## LustScript: Lust inside other programs

**One language, two outputs.** Lust compiles through C# into programs of its own. LustScript is the same Lust turned into Lua, so it can run inside programs that already speak Lua. Same parser, same checker, same rules and friendly errors: only the last step, writing the output, differs. The syntax must never split into two languages.

- **What's left out:** the parts that need .NET: `fetch cs:`, `ui` and saves to AppData. Lust says so when a LustScript file uses them.
- **What comes in:** whatever the host program offers, as a library: `fetch gmod;`, `fetch aventail;`.
- **Choosing the output:** the same `.lust` files, with `lust build --lua`. No new file extension.
- **The Lua it writes:** plain Lua 5.1, which Garry's Mod (LuaJIT) runs and Luau mostly accepts. Classes become tables, states become names, cArrays become lists, and a small Lua library handles `print`, `.text()`, cArray ids and the like. Lua 5.1 has no separate whole numbers, so ints are checked strictly by Lust and are plain numbers in Lua, which is exact up to about 9 quadrillion.
- **The compiler ships as one native file** (step 1 of the plan), so a host program can run it without .NET.

**First homes:**

- **Garry's Mod:** addons written in Lust, Rhy's Library systems included.

  ```
  fetch gmod;

  object MedKit like Hook {
      has {
          int[healAmount 25];
      }

      method playerUse(Player p) {
          is p.health < p.maxHealth? then {
              p.health = p.health + healAmount;
              p:chat("Healed for " + healAmount.text());
          }
      }
  }
  ```

- **Aventail:** scripts for its Phase 9 scripting, next to plain Lua. Aventail stays Lua-first, so nobody has to learn Lust to script it, but LustScript fits naturally: a command's `has` block is exactly its list of settings, so each field shows up in the Attributes panel with its starting value. The scripts run in Aventail's Luau sandbox, with its limits and undo, like any Lua script.

  ```
  fetch aventail;

  object RoundEverything like Command {
      has {
          string[title "Round every edge"];
          float[radius 0.05];
      }

      method run() {
          loop (Mesh m in Scene:selected()) {
              m:bevel(m:edges(), radius);
          }
      }
  }
  ```

- **Later:** Halberd's Lua plugins, or any other program with Lua inside.

## Machine code

**Native programs, soon.** .NET's Native AOT turns the C# Lust writes into real machine code: one `.exe`, no .NET needed to run it, starting instantly. That's `lust build --native`, a small change. Limits: it needs Visual Studio's C++ build tools to build, WinForms programs can't be built this way (another reason for views), and some NuGet packages that lean on reflection won't work.

**Our own backend, maybe someday.** Compiling Lust straight to machine code without C# means a lot of new work:

- handling memory ourselves (a garbage collector, or ownership rules)
- our own standard library
- talking to Windows directly
- debugger info so Ledger can still step through code
- losing `fetch cs:`

The realistic route is LLVM, the way Rust, Swift and Zig do it. Only worth it if Lust outgrows .NET.

## Tools

- **Faster runs:** compile inside Lust with Roslyn instead of calling `dotnet build`. Runs in well under a second, maybe with hot reload.
- **Stepping through Lust in Ledger's debugger.** The C# Lust writes already knows which Lust line each part came from.
- **Autocomplete and hover info** in Ledger.
- **Lust's folder in Ledger's terminal:** Ledger could add the Lust path from Settings to its terminal, so `lust` works there without editing PATH.
- **Installing and updating:** GitHub Releases, a one-line installer, and `lust update`.

## Libraries

- **`fetch sound;`**: `Sound:play("boom.mp3");`, with NAudio underneath.
- **`fetch game;`**: a very light 2D engine (sprites, a game loop, collisions) on the same drawing layer as views, where each game object is a class with states. Plus a small editor for placing sprites in a scene, written in Lust.
- **`fetch web;`**: see [The web](#the-web-fetch-web) above. Getting things from the internet (`Web:get(url)`) belongs here too.
- **ui 2**, the widgets real apps need (these become view parts once views exist):
  - `TextArea`: a multi-line text box. The most important one.
  - Panels with layout: a side panel, a main area, a status bar, and dividers that can be dragged.
  - `List` and `Tree`, for songs, files and folders.
  - Tabs and menus.
  - Key combos like Ctrl+S (`keyPressed` doesn't know about Ctrl yet).
  - Open and save file dialogs.
  - Sliders and progress bars.
- **WinForms controls from C#:** Lust can't `fetch cs:` WinForms types yet, because the compiler doesn't look in Windows' desktop libraries. Fixing that would allow wrapping things like ScintillaNET, a code editor with colouring.

## Projects to build in Lust

Real projects show what's missing, the way Lust Valley did.

- **The orc fight, made visual:** the same fight as a terminal game and as a window game, with pictures for each state, health bars, and attacks you can see. Shows how logic, input and drawing split apart.
- **Lust Tunes, a little MP3 player:**
  - Your Music folder, a play queue, search, and playlists kept with `save`.
  - A seek bar and volume slider drawn on a canvas.
  - NAudio from NuGet does the sound.
- **Ledger, in Lust:** a notes app with a file tree, tabs, themes and settings. Mostly needs ui 2. A real code editor with colouring is the stretch goal.
