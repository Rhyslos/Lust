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
| | **4. `keep`:** the memory rule for states | Next |
| | **5. `node` and `assembly`** | |
| | **6. LustQL, then saves on top of it** | |
| | 🏁 *Programs keep their data.* | |
| | **7. `view` and our own renderer,** starting with a small test run (a window, text, a button) | |
| | **8. Servers:** `schema` and `route`, if they're still wanted by then | |
| | **9. Ledger support, polish, and the Lust 2.0 release** | |

## Decisions along the way

- **LustQL runs on SQLite.** SQLite does the hard part of a database (keeping data safe through crashes, indexes, one file per database), and it's free to use in anything. LustQL is a friendly language on top that Lust checks and turns into SQL. Garry's Mod has SQLite built in, so LustQL can work from LustScript too. Saves use it as well, so there's no save format of our own to design. A cMap's key matches a table's primary key.
- **`view` is our own, all the way down.** Most of the remaining work is here.
- **Servers come last.** They're on the list, but whether web servers are worth it gets decided when we get there.
