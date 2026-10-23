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
| | **11.3 to 11.7 More for windows:** tooltips, menus and dialogs, dark mode, grids, panels, tabs and trees, animation and drawing, and the system's open and save boxes | Next |
| | 🏁 *The Lust 2.0 release.* | |

## Decisions along the way

- **Databases run on SQLite.** For now, queries are written in Lust code (`Shop.items:where(price < 3.0)`); a `.lql` language of its own is designed later. SQLite does the hard part of a database (keeping data safe through crashes, indexes, one file per database), and it's free to use in anything. Lust checks queries and turns them into SQL. A cMap's key matches a table's primary key. Garry's Mod has SQLite built in, so LustScript there can get databases later.
- **Saves are readable JSON files** (`name.save`), as the rulebook asked, not databases.
- **`view` is our own, all the way down,** and it comes last, once everything it builds on has settled. Most of the remaining work is here.
- **Servers read and answer requests several at once,** while the methods that answer run one at a time.
- **An engine describes itself in ordinary Lust files,** with `native` methods, instead of Lust having a special `fetch solis;`. The engine writes the files (one per topic, like `solis/entity.lst`), games fetch them like any other file, and the same works for any program Lust is part of.
