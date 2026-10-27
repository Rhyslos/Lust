# Roadmap

The order Lust gets built in. Each step makes Lust more useful on its own, and nothing waits on something built later. [LUST.md](LUST.md) has the rules as they are today; [IDEAS.md](IDEAS.md) has the older, longer list of ideas.

| | Step | Status |
| --- | --- | --- |
| ✅ | **LustScript:** `.lsc` checked and turned into Lua, `lust run` with Lua built in | Done (alpha 1) |
| ✅ | **Real programs:** `.lst` built through Rust into a program of its own | Done (alpha 2) |
| ✅ | **1. cMaps and `.lst`:** the key is the first column, key-makers; programs end in `.lst` | Done (alpha 3) |
| ✅ | **2. Sharing between files:** `pass`, `fetch`, `get` | Done (alpha 4) |
| ✅ | **3. A basic standard library:** files and folders, time, maths, text tools, JSON | Done (alpha 5) |
| 🏁 | *Lust is a complete tool for console programs and Garry's Mod scripts.* | Reached |
| ✅ | **4. `keep`:** things made in a state end with it, unless they're kept | Done (alpha 6) |
| ✅ | **5. `node` and `assembly`:** objects that open and close, and factories with one recipe per class | Done (alpha 7) |
| ✅ | **6. Databases (on SQLite), saves, and `lust new`:** tables used like cMaps, queries written in Lust, saves as readable JSON | Done (alpha 8) |
| 🏁 | *Programs keep their data.* | Reached |
| ✅ | **7. Servers:** `schema` and `route`, answered by methods, with `Web:serve` and `Web:request` | Done (alpha 9) |
| ✅ | **8. Ledger support and polish:** Ledger colours, checks, runs and builds `.lst` and `.lsc` files from their project's folder | Done |
| ✅ | **Filling the gaps:** `Random[min, max]`, text patterns, dates, sorting, a web client, servers that read and answer several requests at once | Done (alpha 10) |
| ✅ | **9.1 A window:** `view`, `App:open()`, `column`, `row`, `stack`, `space`, `text`, `button` and styles, drawn by Lust itself, with a test mode | Done (alpha 11) |
| ✅ | **9.2 Layout and styles:** sizes that fill and share, alignment, `spread`, stacks, wrapping text, borders, opacity, nine shades of each colour | Done (alpha 12) |
| ✅ | **9.3 Pieces:** assemblies used inside views (`TodoRow { label = task };`), from files in `ui/` folders, each row's piece found by its key | Done (alpha 13) |
| ✅ | **9.4 Inputs:** `input`, `checkbox` and `slider` bound to fields, with typing, selecting, the clipboard, focus and the keyboard | Done (alpha 14) |
| ✅ | **9.5 Scrolling, images and timers:** `scroll` with the wheel and a knob, `image` for PNG and JPEG files, `every` for timers | Done (alpha 15) |
| ✅ | **9.6 Polish:** painting several times faster, hover fades and pressed buttons, `disabled`, undo and redo in inputs, titles that follow the data, `lust new name app` | Done (alpha 16) |
| ✅ | **10. Lust inside other programs:** `native` methods whose bodies are in the program using Lust, and `lust build --library` for a Rust library (the first step towards Solis) | Done (alpha 20) |
| ✅ | **11.1 Keys and fonts:** `enter { }` on inputs, `key "ctrl+s" { }` shortcuts, `font "file.ttf"`, and fonts that cost a quarter of the memory | Done (alpha 21) |
| ✅ | **11.2 Text boxes:** `textarea`, an input with several lines that wrap and scroll | Done (alpha 22) |
| ✅ | **11.3 Things over the view:** `dropdown`, `menu` with `item` and `divider`, `dialog`, and `tip` | Done (alpha 23) |
| ✅ | **11.4 Dark mode:** `view [dark]`, and `dark:[ … ]` for a piece's own dark look | Done (alpha 24) |
| ✅ | **11.5 Bigger layouts:** `grid`, `split`, `tabs`, `branch` and `progress` | Done (alpha 25) |
| ✅ | **11.6 Drawing and motion:** `canvas` with `box`, `circle` and `line`, and `animate` | Done (alpha 26) |
| ✅ | **11.7 Open and save boxes:** `Files:chooseOpen`, `chooseSave` and `chooseFolder` show the computer's own boxes | Done (alpha 27) |
| ✅ | **12.1 Vectors and rotations:** `vec3` and `rotation`, kept as f32 like Solis keeps them, and the same in both outputs | Done (alpha 28) |
| ✅ | **12.2 Values across native methods:** vectors, rotations and arrays, both in a Rust library and in LustScript | Done (alpha 29) |
| ✅ | **12.3 Components:** `component` with `me` and `update`, the `entity` type, and schemas and values for the engine | Done (alpha 30) |
| ✅ | **12.4 Events:** `event`, `on` in components, and `send … to`, queued and delivered by the engine | Done (alpha 31) |
| ✅ | **12.5 The compiler inside an engine:** the compiler as a library taking files' text, mistakes and failures with file, line and column, Lua for Luau with components swapped in while the game runs | Done (alpha 32) |
| ✅ | **12.6 Faster code for every tick:** Lua in Luau over twice as fast (f32 rounding in Luau's vectors, vectors of numbers made once, turning around z alone), with a benchmark that also checks Rust and Luau agree | Done (alpha 33) |
| ✅ | **12.7 Luau vectors:** in Luau a `vec3` is Luau's own `vector`, in fields, events and native methods, with its maths done by Luau and still exactly the same as Rust's | Done (alpha 34) |
| ✅ | **12.8 Deep code:** code nested more than 150 levels deep is a mistake, and the compiler runs on its own stack, so nothing given to it can crash the engine | Done (alpha 35) |
| ✅ | **12.9 No unsafe code in libraries:** a library forbids unsafe code, and gets the time zone from the engine | Done (alpha 36) |
| ✅ | **13.1 match:** `match x { when A { … } when B, C { … } }` for a state's options and bools, with every option covered | Done (alpha 37) |
| ✅ | **13.2 Install and update:** a Windows installer, releases made by themselves, and `lust update` | Done (alpha 38) |
| ✅ | **13.3 print and text:** `print(a, b, c)` with any values, and values in text with `"hp: {hp}"` | Done (alpha 41) |
| ✅ | **13.4 `+=` `-=` `*=` `/=`:** change a value using itself, on numbers, text, vectors and rotations | Done (alpha 42) |
| ✅ | **13.5 Sized numbers:** `int8`, `int16`, `int32`, `uint8`, `uint16` and `uint32`, checked to fit, alongside `int` | Done (alpha 43) |
| ✅ | **13.6 lock:** variables and fields that keep the value they start with | Done (alpha 44) |
| | 🏁 *Lust's first release that isn't a test version.* | |

## Decisions along the way

- **Databases run on SQLite.** For now, queries are written in Lust code (`Shop.items:where(price < 3.0)`); a `.lql` language of its own is designed later. SQLite does the hard part of a database (keeping data safe through crashes, indexes, one file per database), and it's free to use in anything. Lust checks queries and turns them into SQL. A cMap's key matches a table's primary key. Garry's Mod has SQLite built in, so LustScript there can get databases later.
- **Saves are readable JSON files** (`name.save`), as the rulebook asked, not databases.
- **`view` is our own, all the way down,** and it comes last, once everything it builds on has settled. Most of the remaining work is here.
- **Servers read and answer requests several at once,** while the methods that answer run one at a time.
- **An engine describes itself in ordinary Lust files,** with `native` methods, instead of Lust having a special `fetch solis;`. The engine writes the files (one per topic, like `solis/entity.lst`), games fetch them like any other file, and the same works for any program Lust is part of.
