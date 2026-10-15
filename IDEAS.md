# Ideas for later

Things we've talked about but aren't building yet. Nothing here is decided: each idea gets designed together before it's built, the way saves were. The rules that are real live in [LUST.md](LUST.md).

## What Lust is aiming for

- **Easy to learn.** A beginner only needs `print`, `is … ?`, `loop` and variables. Everything else is found later, never needed on day one. Every feature should read out loud and be guessable, and come with friendly error messages.
- **State-driven.** States are Lust's big idea. Things are described by what state they're in and what happens on the way in and out.
- **Powerful through libraries.** The core stays small. Power comes from C# (`fetch cs:`) and from libraries written to feel like Lust, the way `ui` wraps WinForms.
- **Watch for feature creep.** Features that make Lust more *itself* (reacting states) come first. Features that only make it more like C# (generics) come last, or never if C# can do the job.

## The long-term plan

The order we'd build things in, so Lust grows into something real without biting off too much at once:

1. **Native programs:** `lust build --native`. Small, and useful straight away.
2. **`fetch web;`:** servers with routes and JSON. Useful straight away, and it's familiar ground.
3. **`view { }`:** describing screens Compose-style, with Tailwind-style `[ ]` styles, drawn by Lust itself in a window.
4. **The same views on the web,** over a live connection.
5. **The 2D engine,** on the same drawing layer as the views, then a sprite-placing editor written in Lust.
6. **Our own machine code (LLVM),** only if .NET is ever what's holding Lust back.

`view { }` is the heart of it: one way of describing screens serves desktop windows, web pages and the engine's editor. It's worth designing carefully before any of steps 3 to 5 are built.

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
- **Functions written in place** (lambdas): `do(int n) { print(n); }` as a value, for events and callbacks.
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
