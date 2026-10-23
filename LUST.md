# The Lust v2 Rulebook

Lust is a small, strict language for apps, games and tools. It reads like a calm mix of C# structure and Lua's lightness. Checks are written as questions, states are built in and own their memory, and every file ends with `end`.

This is the single source of truth for Lust v2. Things still being designed are marked **(to design)** and gathered in the last section.

> **What works today (2.0 alpha 23):** both outputs. `.lsc` files are checked and turned into Lua. `.lst` files are checked and built into real programs through Rust (`lust build`), and `lust run` runs either kind straight away through Lua. Built so far: types, questions, `either`, maths and text, arrays, maps, cMaps (with key-makers), states (with options inside options), classes, objects, `like`, methods, hooks with `new hook`, every kind of `loop`, `try`/`error`, `lua { }` blocks, the built-ins and the standard library (text and array tools, `Maths`, `Files`, `Time` and `Json`). Sharing between files (`pass`/`fetch`/`get`), things ending with their state (`keep`), nodes, assemblies, saves, databases and servers (`schema`/`route`) work too. Windows with `view` are built too: layout, styles, pieces of your own, inputs, text boxes, dropdowns, menus, dialogs, tips, checkboxes, sliders, scrolling, pictures, timers, keyboard shortcuts and fonts of your own, all drawn by Lust itself. Lust can also be part of a Rust program, with `native` methods and `lust build --library`. Not yet: LustQL as a language of its own. Lust v1 lives on the `v1` branch.

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
| `float` | numbers with a point | `1.5`, `0.25` |
| `string` | text, in double quotes | `"Ada"`, `"line\n"` |
| `bool` | yes or no | `true`, `false`, `not true` |
| a class name | one thing made from a class | `Player`, `Weapon` |
| `T array` | a list of `T` | `int array` |
| `cMap(...)` | a table of rows, found by their key (the first column) | `cMap(int id, string itemName, int amount)` |
| `T map` | values found by a text key | `int map` |
| a state's name | one of that state's options | `GameState` |
| a schema's name | a data shape for the web | `UserPayload` |
| `hook ...` | a method kept as a value | `hook int(int n)` |

- A variable keeps its type forever.
- An `int` fits where a `float` goes. The other way needs `.int()`.
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
- Join checks with `&&` and `||`. Group with `( )`.

### either

A question that gives back one of two values:

```
string mood = either hp > 10? "happy" or "grumpy";
```

---

## 6. Maths and text

- `+ - * / %` work on `int` and `float`. `( )` groups.
- `int / int` gives an `int`: `7 / 2` is `3`. Use `7.float() / 2` for `3.5`.
- `+` joins text. Anything else must be turned into text first: `"Score: " + score.text()`.

| Method | Does |
| --- | --- |
| `.text()` | anything as text |
| `.int()` | a float or text as a whole number |
| `.float()` | an int or text as a float |
| `.isNumber()` | `true` when text is a number |
| `.lines()` | text cut into its lines |

More tools for text, and `Maths` for square roots, rounding and the rest, are in section 19.

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
```

- `stop;` leaves the loop. `skip;` jumps to the next round.

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
- Columns hold numbers, text, bools and state options.
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

- **Pieces:** `column`, `row`, `stack`, `space`, `text` and `button`. A button's `{ }` runs when it's clicked, and the window redraws afterwards.
- **Scrolling, pictures and timers:** `scroll`, `image` and `every` (see below).
- **Inputs:** `input`, `textarea` (several lines), `checkbox` and `slider`, each bound to a field (see below), with the mouse, the keyboard and the clipboard.
- **Your own pieces:** any assembly with a view, used inside another view as `TodoRow { label = task };`, from the same file or fetched from files in `ui/` folders (see below).
- **Styles and layout:** every word in the list below, `hover:[ … ]` included, plus named styles (`style chip = [pad 6, round];`). Text wraps onto more lines when it doesn't fit.
- **Questions and loops** inside a view.
- **The window's settings:** `title`, `width`, `height`, `minWidth`, `minHeight` and `background`. They're worked out each time the view is drawn, so a title can show the data: `view [title "To do (" + (#todos).text() + ")"]`.
- **`disabled`** greys out a piece and stops it being clicked, typed in or reached with Tab. It can depend on the data: `button "Save" [disabled name == ""] { … }`.
- **Undo and redo** in inputs: Ctrl+Z, and Ctrl+Shift+Z or Ctrl+Y (Cmd on a Mac). A run of typing undoes in one go.
- **Enter and shortcuts:** `input newTodo enter { add(); }` runs a block when Enter is pressed in the input, and `key "ctrl+s" { save(); }` is a keyboard shortcut (see below).
- **Fonts:** `font "fonts/Lora.ttf"` draws a piece's words in a font file of your own, and on the view's `[ ]` it's the whole window's font.
- **Feedback:** hover looks fade in and out, and a button looks pressed in while it's held down.
- **Things that show over the view:** `dropdown` lists to choose from, `menu`s of `item`s, `dialog`s that pop up while a bool is true, and `tip "…"` for a note when the mouse rests on a piece (see below).

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
| `click Add one` | clicks the button showing that text, or a row of an open list (`click Cupboard`, `click Save`) |
| `open menu File`, `open dropdown 1` | clicks a menu or the first dropdown, opening its list |
| `hover Delete` | puts the mouse on the button showing that text, and shows its tip straight away |
| `focus input 1` | gives the first input focus (also `textarea 1`, `checkbox 2`, `slider 1`, `button 3`) |
| `type Hello` | types into the input that has focus |
| `key ctrl+left` | presses a key: `backspace`, `delete`, `left`, `right`, `home`, `end`, `up`, `down`, `tab`, `enter`, `space`, `escape`, with `shift+` or `ctrl+` in front, and `ctrl+a`, `ctrl+c`, `ctrl+x`, `ctrl+v`; any key a shortcut uses works too (`key ctrl+s`, `key f1`) |
| `check 1` | clicks the first checkbox |
| `slide 1 70` | drags the first slider to 70 |
| `press 40 20`, `move 90 20`, `release 90 20`, `dblclick 40 20` | the mouse, at a place in the window |
| `scroll 1 100` | scrolls the first scroll down 100 (up with a minus) |
| `wait 2.5` | lets 2.5 seconds go by on the test's own clock, running the timers that come due |
| `size 800 600` | resizes the window |
| `snapshot shot.png` | draws the window into a picture |

`lust test` sets it for every test, so a view's tests compare what `dump` prints.

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

- **What they change.** An `input` changes a string, a `checkbox` a bool, and a `slider` an int or a float. It's bound to a field (or a field of something, like `todos[id].task`), not a variable of the view, because the view's variables are made again each time it's drawn.
- **Styles of their own:** `placeholder "…"` is what an empty input shows. A slider goes `from` one number `to` another, one `step` at a time (`step 0` slides smoothly). A colour word colours a checkbox's tick and a slider's bar: `slider volume [purple]`. Inputs are 220 wide and sliders 200 unless they're given a `width`.
- **The mouse:** click to put the caret somewhere, drag to select, double-click for a word, three clicks for everything, Shift+click to select up to there. Drag a slider's knob, or click anywhere along it.
- **The keyboard:** Tab and Shift+Tab move between inputs, checkboxes, sliders and buttons, and what has focus gets a ring. In an input: the arrows, Home and End (with Shift to select, and with Ctrl, or Option on a Mac, to go by words), Backspace and Delete, and Ctrl+A, C, X and V (Cmd on a Mac) to select all, copy, cut and paste. Space ticks a checkbox and presses a button, and so does Enter on a button. The arrows move a slider by one step. Escape takes focus away.

### The pieces

| | |
| --- | --- |
| Layout | `column`, `row`, `stack` (layers), `scroll`, `space` |
| Content | `text`, `button`, `input`, `textarea`, `checkbox`, `slider`, `image` |
| Choosing | `dropdown`, `menu` with `item` and `divider` |
| Over the view | `dialog` |
| Time and keys | `every`, `key` |

Questions and loops work inside a view: showing something only sometimes is an `is … ? then`, and a list is a `loop`.

### Text boxes

`textarea notes;` is an input with several lines, for notes, descriptions and messages. It's bound to a string field like an input, and the string holds the lines with `\n` between them:

```
textarea notes [placeholder "Write something…", width fill, height 160] {
    saved = false;
}
```

- **Lines:** Enter starts a new line, and lines too long for the box wrap between words. The text box is 300 wide and four lines tall unless it's given a `width` and `height`; when there's more, it scrolls (with the wheel, or by itself to keep the caret in view).
- **Keys:** everything an input has, and Up and Down go a line up or down, Home and End to the start or end of the line, and Ctrl+Home and Ctrl+End (Cmd+Up and Cmd+Down on a Mac) to the start or end of the whole text. Pasting keeps the copied lines.
- **`enter { }`** on a text box runs on Ctrl+Enter (Cmd+Enter on a Mac), since Enter starts a new line: `textarea message enter { send(); }`.

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

### Menus

`menu "File" { … }` is a word that opens a list of `item`s, each running its block when it's chosen. `divider;` draws a line between items, and an item can be `disabled`:

```
row [gap 4] {
    menu "File" {
        item "New" { clear(); }
        item "Save" [disabled saved] { save(); }
        divider;
        item "Quit" { confirming = true; }
    }
    menu "Help" {
        item "About" { showingAbout = true; }
    }
}
```

A row of menus at the top of the window is a menu bar, but a menu can go anywhere, like a "⋯" menu on a row of a list. Items can come from a `loop`. The keyboard works as in a dropdown, and Down opens a menu too.

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

### Pictures

`image "cat.png";` shows a PNG or JPEG file, found the way `Files` finds files: from the folder the program runs in, or anywhere with its whole path. Without a size it's as big as its file, shrinking to fit if there isn't room. Give it a `width` or a `height` and the other follows, keeping its shape. Give both and it fits inside that box; add `cover` and it fills the box instead, cut to fit. `round` rounds its corners, so `[width 48, height 48, cover, round 24]` makes a round picture. A file that isn't there shows as a grey box.

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
| inputs and pictures | `placeholder "…"`, `from`, `to`, `step`, `cover` |
| words | `font "file.ttf"` |
| off | `disabled`, or `disabled` with a bool: `disabled not ready` |

**Colours** are `white`, `black`, `gray1` to `gray9`, and `red`, `orange`, `yellow`, `green`, `teal`, `blue`, `indigo`, `purple` and `pink`. Each of those has nine shades from light to dark: `blue1` is a pale blue for backgrounds, `blue9` a deep one. Any other colour is written `"#2f9e44"`. The shades are the Open Color palette.

**Sharing the room.** Pieces are as big as what's in them. `width fill` in a row (or `height fill` in a column) takes the room that's left over; several fills share it equally, and `grow 2` takes twice the share of `grow 1`. A `space` is an empty fill, so it pushes what comes after it to the end. `minWidth` and `maxWidth` keep a piece between two sizes.

**Placing.** On a column, `left`, `center` and `right` place its children side to side, and `top`, `middle` and `bottom` up and down in any room to spare. On a row it's the same, and its children sit in the middle up and down unless told otherwise. `spread` puts the spare room evenly between the children: `row [spread] { text "Back"; text "Next"; }` puts one at each end. A row or column told `center`, `right` or `spread` stretches across the room it's in, so there's somewhere to place its children. On a text, `center` and `right` place its lines.

A `stack` lays its children on top of each other, the later ones in front, placed by these same words: a picture with a label on it, or a badge in a corner.

**Text wraps** onto more lines when it's wider than the room it has, breaking between words. `\n` in the text starts a new line, and `nowrap` keeps it on one line.

**Values can come from the data.** After a style word, a value can be any number, text or bool: `[grow done, background shade, size 12 + extra, disabled not ready]`, with fields and variables. Only colour words (`red`, `gray3`…) and `fill` are read as words.

Name a style to reuse it: `style card = [pad 16, round, shadow];`, then `[card]`.

### Underneath

Lust draws everything itself, from the pixels up: layout, every piece, text editing, focus, scrolling, styles and animation. Small permissive libraries only open the window (winit), hand over the pixels (softbuffer), read font files (ab_glyph), reach the system clipboard (arboard) and read PNG and JPEG files (png, jpeg-decoder). Views are for `.lst` programs on Windows, macOS and Linux. Web pages stay HTML, CSS and JavaScript.

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
| `print(x);` | shows `x` on its own line |
| `wait seconds;` | pauses |
| `ask(question)` | shows a question and gives back what's typed |

### Random numbers

```
int dice = Random[1, 6];          -- a whole number from 1 to 6
float chance = Random[0.0, 1.0];  -- a decimal from 0 up to (not quite) 1
```

- With two ints, and going into an int, it's a whole number, and both ends can come up.
- With a float, or going into a float, it's a decimal from the lowest up to just below the highest.
- Each run rolls different numbers, even two runs started at the same moment. To get the same numbers every time (for tests), set `LUST_SEED` to a number before running: `lust run` and the built program then roll exactly the same numbers. `lust test` sets it to 1.

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

### Built-in objects

Lust comes with five objects. Their methods are called like any object's: `Maths:sqrt(2.0)`. The fifth, `Web`, is in section 18.

**Maths**

| | Gives |
| --- | --- |
| `Maths:abs(x)`, `Maths:min(a, b)`, `Maths:max(a, b)`, `Maths:clamp(x, low, high)` | an int for ints, a float when there's a float |
| `Maths:round(x)`, `Maths:floor(x)`, `Maths:ceil(x)` | the nearest whole number, the one below, the one above (an int) |
| `Maths:sqrt(x)`, `Maths:power(x, p)`, `Maths:sin(x)`, `Maths:cos(x)`, `Maths:tan(x)` | a float. Angles are in radians |
| `Maths.pi` | 3.14159… |

**Files.** Paths are from the folder the program runs in.

| | Does |
| --- | --- |
| `Files:read(path)` | the whole file as text |
| `Files:write(path, text);`, `Files:append(path, text);` | writes the file, or adds to its end |
| `Files:exists(path)` | true if there's a file or folder there |
| `Files:remove(path);` | removes a file, or an empty folder |
| `Files:list(folder)` | a string array of what's in the folder, in order |
| `Files:makeFolder(path);` | makes the folder, and any folders it's inside |

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

- JSON holds numbers, text, bools, state options (as their names), arrays, maps (as JSON objects), cMaps (as arrays of objects) and things made from classes (as objects of their fields). Hooks stay out.
- `Json:read` needs a place to go, so Lust knows what it's reading: `Player back = Json:read(text);`.
- A class's fields that are missing from the JSON keep their starting values. Anything extra is left out.
- If the JSON doesn't fit, the error says where: `the JSON at bag[0] is missing item.`

In LustScript, `Files:list`, `Files:makeFolder` and the precise clock need `lust run`. Inside other programs, like Garry's Mod, they say they aren't available.

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

- A native method takes and gives back ints, floats, text, bools and things made from classes.
- Where the body comes from:
  - **built as a Rust library** (below): from the Rust program using it;
  - **LustScript** (`.lsc`): from the host's own Lua function with the same name, or `Class_method` for a class's method, which is given the thing first;
  - **on its own** (`lust run`, or a program built without `--library`): nowhere, so calling it stops with a message saying where its body lives.

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
- **Mistakes come back as text.** Everything gives back a `Result`, and a Lust failure is the same message Lust prints, with its file and line: `Err("game.lst:24  this is nothing, so it has no fields or methods.")`. The program decides what to do with it.
- Ints, floats, text and bools are Rust's `i64`, `f64`, `String` and `bool`. A thing made from a class is `Option<Weapon>`, since it can be `nothing`.

`lust test` tests libraries too: a `game.host.rs` next to `game.lst` is a Rust program that uses it, and its output is compared with `game.out`.

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
```

The built-ins can't be used as names either:

```
print      wait       ask
```

Neither can the built-in objects: `Maths`, `Files`, `Time`, `Json`, `Web` and `Random`.

---

## 23. Still to design

- **LustQL (`.lql`):** a query language of its own, for later. Queries are written in Lust code for now (section 14).
- **Ready-made host files:** the files describing Garry's Mod's or Aventail's functions as native methods (section 21), so LustScript there is checked like the rest of Lust.
- **More for engines** (Solis): vectors as real value types, typed events, leaner Rust for code that runs every frame, and rebuilding a game's Lust in a couple of seconds.
- **The standard library:** which Rust crates Lust wraps for files, networking, JSON, dates and sound, and how they look from Lust.
- **From v1:** whether `with x { }`, `{ }` inside text and the word `when` come along.
