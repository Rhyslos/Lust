# Ideas for later

Things we've talked about but aren't building yet. Nothing here is decided: each idea gets designed together before it's built, the way saves were. The rules that are real live in [LUST.md](LUST.md).

## What Lust is aiming for

- **Easy to learn.** A beginner only needs `print`, `is … ?`, `loop` and variables. Everything else is found later, never needed on day one. Every feature should read out loud and be guessable, and come with friendly error messages.
- **State-driven.** States are Lust's big idea. Things are described by what state they're in and what happens on the way in and out.
- **Powerful through libraries.** The core stays small. Power comes from C# (`fetch cs:`) and from libraries written to feel like Lust, the way `ui` wraps WinForms.
- **Watch for feature creep.** Features that make Lust more *itself* (reacting states) come first. Features that only make it more like C# (generics) come last, or never if C# can do the job.

## The language

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

## Tools

- **Faster runs:** compile inside Lust with Roslyn instead of calling `dotnet build`. Runs in well under a second, maybe with hot reload.
- **Stepping through Lust in Ledger's debugger.** The C# Lust writes already knows which Lust line each part came from.
- **Autocomplete and hover info** in Ledger.
- **Lust's folder in Ledger's terminal:** Ledger could add the Lust path from Settings to its terminal, so `lust` works there without editing PATH.
- **Installing and updating:** GitHub Releases, a one-line installer, and `lust update`.

## Libraries

- **`fetch sound;`**: `Sound:play("boom.mp3");`, with NAudio underneath.
- **`fetch game;`**: a small 2D engine (sprites, a game loop, collisions) on Raylib or MonoGame, where each game object is a class with states.
- **`fetch web;`**: getting things from the internet.
- **ui 2**, the widgets real apps need:
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
