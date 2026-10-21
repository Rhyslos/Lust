# The Lust v2 Rulebook

Lust is a small, strict language for apps, games and tools. It reads like a calm mix of C# structure and Lua's lightness. Checks are written as questions, states are built in and own their memory, and every file ends with `end`.

This is the single source of truth for Lust v2. Things still being designed are marked **(to design)** and gathered in the last section.

> **What works today (2.0 alpha 3):** both outputs. `.lsc` files are checked and turned into Lua. `.lst` files are checked and built into real programs through Rust (`lust build`), and `lust run` runs either kind straight away through Lua. Built so far: types, questions, `either`, maths and text, arrays, maps, cMaps (with key-makers), states (with options inside options), classes, objects, `like`, methods, hooks with `new hook`, every kind of `loop`, `try`/`error`, `lua { }` blocks and the built-ins. Not yet: sharing between files (`pass`/`fetch`/`get`), `node`, `assembly`, `keep`, saves, `view`, `schema`/`route`, and LustQL. Lust v1 lives on the `v1` branch.

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
- **Dashes are not allowed** in names, file names or folder names: `my_file.lst`, `myFile.lst`, never `my-file.lst`.

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

- **PascalCase for kinds of things:** classes, objects, states, schemas, assemblies, nodes. `Player`, `GameState`, `LootSpawner`.
- **camelCase for everything else:** variables, fields, methods, hooks, files and folders. `currentHp`, `takeDamage()`, `main.lst`, `entities/player.lst`.
- So `Player player = new Player;` reads clearly: the capital is the kind, the small letter is the thing.
- Map keys are text, so any text is fine: `"max_hp"`, `"item-id"`.
- ALL_CAPS names are not allowed.
- **Every word Lust uses is reserved** and can't be a name (see section 20).

---

## 4. Types and variables

A variable is declared with its type, its name and a value:

```
int lives = 3;
float speed = 1.5;
string name = "Ada";
truth alive = true;
```

| Type | Holds | Examples |
| --- | --- | --- |
| `int` | whole numbers | `0`, `42`, `-7` |
| `float` | numbers with a point | `1.5`, `0.25` |
| `string` | text, in double quotes | `"Ada"`, `"line\n"` |
| `truth` | yes or no | `true`, `false`, `not true` |
| a class name | one thing made from a class | `Player`, `Weapon` |
| `T array` | a list of `T` | `int array` |
| `cMap(...)` | a table of rows, found by their key (the first column) | `cMap(int id, string itemName, int amount)` |
| `T map` | values found by a text key | `int map` |
| a state's name | one of that state's options | `GameState` |
| a schema's name | a data shape for the web | `UserPayload` |
| `hook ...` | a method kept as a value | `hook int(int n)` |

- A variable keeps its type forever.
- An `int` fits where a `float` goes. The other way needs `.int()`.
- A variable declared inside `{ }` only exists inside those braces.
- A name can't be declared twice while it still exists.
- **`nothing`** means "no thing here yet". Only class-typed values can be `nothing`. Using a field or method on `nothing` stops the program with an error naming the line.

---

## 5. Truth and questions

- Truth values are `true` and `false`. `not` flips one: `not alive`, `not true`.
- The Lust way to write a negative check is with `not`: `is not alive?`.

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
| `a == b` | same value |
| `a != b` | different values |
| `a === b` | the very same thing (the same object, not just an equal copy) |
| `a !== b` | not the very same thing |
| `<  >  <=  >=` | comparisons for numbers |

- For numbers, text and truth values, `===` means the same as `==`. It matters for things made from classes: two players with the same name are `==` only if you compare their fields, but `===` asks whether they are one and the same player.
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
- `loop (string name, int price, int stock in shop) { … }` goes through the rows in the order they were added.

**Keys Lust makes.** A new row can leave its key out, and Lust makes one:

- **An `int` key counts up** from the biggest key so far: `bag:inject(["Map", 1]);` gets 3.
- **A key-maker** makes it instead. It's a method or hook that takes nothing and gives back a key, written after the key column like a starting value:

```
method int rollId() {
    return random(1000, 9999);
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

- Change it with `is`: `state is GameState.ActiveGame.Combat;`
- A question about an option includes everything inside it: during `Combat`, `is state == GameState.ActiveGame?` is yes.

### States own memory

Things made while a state is active belong to that state, and are freed the moment the state ends. There's no garbage collector, and nothing to free by hand.

**The safety rule:** something made inside a state can't be stored anywhere that lives longer than the state. Lust catches this before the program runs:

```
// during Combat
Item sword = new Item { name = "Sword" };
player.bag:inject(sword);      -- mistake: the sword is freed when Combat ends, but the bag lives on
```

**To keep it, say so.** `keep` moves it out of the state's memory, into the place that will hold it:

```
keep sword in player.bag;      -- the sword now lives as long as the bag
```

This rule may be reworked as the design matures **(to design)**.

---

## 9. node and assembly

### node: a short-lived group

A node bundles things that live and die together, like a room in a game. Leaving or closing the node frees everything inside it:

```
node RoomOne {
    state is Active;
    Player guard = new Player { name = "Guard" };
}
```

How nodes and state memory work together is **(to design)**.

### assembly: a factory

An assembly makes ready-filled things without constructors:

```
assembly LootSpawner {
    create Item {
        name = "Potion";
        value = 50;
    }
}

Item drop = LootSpawner:create(Item);
```

The exact shape of `create` (several recipes per assembly, choosing between them, collections) is **(to design)**.

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

Shop.onSell = new hook {
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
pass GameState;
```

Another file brings things in with `fetch` (classes, objects, states) and `get` (single methods). The path is in quotes, from the project's root, without `.lst`:

```
fetch "system/gameState":GameState;
fetch "entities/player":[Player, Party];
get "helpers/mathUtils":clamp;
```

---

## 14. Saving and loading

```
save gameSession = [Farm, player, currentDay];

is saved gameSession? then {
    load gameSession;
}

save gameSession;
```

- A save lists what it holds once, then `save` and `load` do the rest.
- Save files are readable text. Loading is strict: if the save doesn't match the program, nothing changes and Lust says why.

---

## 15. Errors

```
try {
    writeText("config.json", data);
} error (string message) {
    print("That didn't work: " + message);
}
```

- `try` runs the block. If something goes wrong inside it, the program carries on in `error` instead of stopping.
- Without `try`, an error stops the program with the file, line and a plain message.

---

## 16. LustScript (.lsc)

LustScript is Lust turned into Lua, for running inside programs that already speak Lua. It's the same language with the same rules and checks. Only the output differs.

- The host program offers its own library: `fetch gmod;`, `fetch aventail;`.
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

Screens are described Compose-style, with Tailwind-style styles in `[ ]`. Change a field or a state and the screen redraws itself. Lust draws everything itself, with no toolkit underneath.

```
fetch ui;

state Clock = [Stopped, Running, Paused];

object Timer like App {
    has {
        int secondsLeft = 1500;
        state is Clock.Stopped;
    }

    view {
        column [center, gap 12, pad 24] {
            text time() [size 28, weight 500];
            row [gap 8] {
                button "Start" [blue, round] { state is Clock.Running; }
                button "Pause" [subtle] { state is Clock.Paused; }
            }
            is state == Clock.Paused? then { text "Paused" [muted]; }
        }
    }
}

Timer:open();
```

- Buttons say what they do in their own block.
- Styles are short words: `gap 12`, `pad 24`, `blue`, `round`. Name one to reuse it: `style card = [pad 16, round, shadow];`.
- Questions and loops work inside a view.
- Views are for desktop apps. Web pages stay HTML, CSS and JavaScript.

---

## 18. Web: schema and route

### schema: the shape of data

A schema describes data coming in and going out. A field with no value must be filled in; a field with a value is optional:

```
schema UserPayload {
    has {
        string username;
        int age = 0;
    }
}
```

### route: answering requests

```
route "/api/v1" {
    get "/user/:id" => getUser;
    post "/user" => createUser;
}

pass route;
```

How handlers receive a schema and send one back is **(to design)**.

---

## 19. Built-ins and testing

| Built-in | Does |
| --- | --- |
| `print(x);` | shows `x` on its own line |
| `wait seconds;` | pauses |
| `random(low, high)` | a whole number from `low` to `high` |
| `ask(question)` | shows a question and gives back what's typed |
| `readText(file)` | a whole text file |
| `writeText(file, text);` | writes a text file |

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

## 21. Reserved words

These can't be used as names:

```
class      object     node       assembly   create     has
method     hook       return     like       new        nothing
schema     route      view       style      keep       pass
fetch      get        lua        is         then       or
either     not        true       false      loop       stop
skip       in         try        error      state      save
load       saved      end        int        float      string
truth      array      cMap       map
```

The built-ins can't be used as names either:

```
print      wait       random     ask        readText   writeText
```

---

## 22. Still to design

- **State memory:** whether the `keep` rule is enough, and how nodes and states share memory.
- **assembly:** several recipes, choosing between them, and making collections.
- **LustQL (`.lql`):** what queries look like. It runs on SQLite underneath (see [ROADMAP.md](ROADMAP.md)).
- **Web:** how route handlers take and give schemas.
- **The standard library:** which Rust crates Lust wraps for files, networking, JSON, dates and sound, and how they look from Lust.
- **Memory in Rust:** how state arenas and `keep` turn into Rust that satisfies Rust's ownership rules (likely arenas with handles).
- **From v1:** whether `with x { }`, `{ }` inside text and the word `when` come along.
