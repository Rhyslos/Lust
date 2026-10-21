# The Lust v2 Rulebook

Lust is a small, strict language for apps, games and tools. It reads like a calm mix of C# structure and Lua's lightness. Checks are written as questions, states are built in and own their memory, and every file ends with `end`.

This is the single source of truth for Lust v2. Things still being designed are marked **(to design)** and gathered in the last section.

> **What works today (2.0 alpha 9):** both outputs. `.lsc` files are checked and turned into Lua. `.lst` files are checked and built into real programs through Rust (`lust build`), and `lust run` runs either kind straight away through Lua. Built so far: types, questions, `either`, maths and text, arrays, maps, cMaps (with key-makers), states (with options inside options), classes, objects, `like`, methods, hooks with `new hook`, every kind of `loop`, `try`/`error`, `lua { }` blocks, the built-ins and the standard library (text and array tools, `Maths`, `Files`, `Time` and `Json`). Sharing between files (`pass`/`fetch`/`get`), things ending with their state (`keep`), nodes, assemblies, saves, databases and servers (`schema`/`route`) work too. Not yet: `view`, and LustQL as a language of its own. Lust v1 lives on the `v1` branch.

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
        sharpness = random(1, 5);
    }
}

Item drop = Loot:create(Item);
Sword blade = Loot:create(Sword);
```

- A recipe sets fields, and its values are worked out each time something is made, so `random` gives each sword its own sharpness.
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
- **Only the file being run runs its code.** The code at the bottom of a fetched file is left out, so any file can be run on its own to try it out.
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
- The file is `gameSession.save`, next to the program, as readable JSON.
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

- The first column is the key, as in a cMap. An `int` key counts up when a row leaves it out. A `string` key has to be given.
- Columns hold numbers, text, truths and state options.
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
- For now, a server answers one request at a time.

---

## 19. Built-ins and the standard library

| Built-in | Does |
| --- | --- |
| `print(x);` | shows `x` on its own line |
| `wait seconds;` | pauses |
| `random(low, high)` | a whole number from `low` to `high` |
| `ask(question)` | shows a question and gives back what's typed |

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

### Arrays

| | Does |
| --- | --- |
| `words:join(", ")` | one text from a string array, with `", "` between the items |
| `scores:has(10)`, `scores:find(10)` | whether it's there, and where (or -1). For arrays of numbers, text and truths |
| `scores:sort();` | puts numbers or text in order, smallest first |
| `scores:reverse();` | turns the array around |

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

**Json** turns values into JSON text and back:

```
string text = Json:write(player);     -- {"name":"Ada","hp":20}
string nice = Json:pretty(player);    -- the same, over several lines
Player back = Json:read(text);
```

- JSON holds numbers, text, truths, state options (as their names), arrays, maps (as JSON objects), cMaps (as arrays of objects) and things made from classes (as objects of their fields). Hooks stay out.
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
print      wait       random     ask
```

Neither can the built-in objects: `Maths`, `Files`, `Time`, `Json` and `Web`.

---

## 22. Still to design

- **LustQL (`.lql`):** a query language of its own, for later. Queries are written in Lust code for now (section 14).
- **The standard library:** which Rust crates Lust wraps for files, networking, JSON, dates and sound, and how they look from Lust.
- **From v1:** whether `with x { }`, `{ }` inside text and the word `when` come along.
