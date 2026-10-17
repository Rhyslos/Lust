# The Lust v2 Rulebook

Lust is a small, strict, multi-backend language designed for native applications, web deployment via WebAssembly, and Lua-targeted modding (**LustScript**). It features C#'s structural backbone, a clean Lua-inspired aesthetic, checks written as questions, state-bound memory ownership, and files that always close with `end`[cite: 1].

```
is hp <= 0? then {
    state is knocked_out;
}
```

This file is the source of truth. The compiler follows it, and when the two disagree, one of them gets fixed[cite: 1].

**Contents:** [Files](#files) · [Types and variables](#types-and-variables) · [Truth and checks](#truth-and-checks) · [Math and text](#math-and-text) · [Arrays](#arrays) · [cArrays](#carrays) · [Maps](#maps) · [state](#state) · [Classes and objects](#classes-and-objects) · [Functions and methods](#functions-and-methods) · [Loops](#loops) · [Sharing between files](#sharing-between-files) · [Saving and loading](#saving-and-loading) · [C# and Lua Interop](#c-and-lua-interop) · [Windows and UI](#windows-and-ui) · [Built-ins](#built-ins) · [Errors](#errors) · [How Lust runs](#how-lust-runs) · [Reserved words](#reserved-words)

---

## Files

- A Lust file ends in `.lust`[cite: 1].
- Every statement ends with `;`[cite: 1].
- Every file ends with the word `end`. Nothing may come after it except blank lines and comments[cite: 1].
- Comments start with `//` or Lua-style `--` and run to the end of the line. There are no block comments[cite: 1].
- Inside a file, the order is: `fetch` and `get` lines first, then everything else[cite: 1].
- Every project requires a **`main.lust`** master file at its root, serving as the core execution anchor (like an HTML index file)[cite: 1].

```
-- hello.lust
print("Hello!");

end
```

## Types and variables

A variable is declared with its type first, followed by its name, and a value[cite: 1]:

```
int lives = 3;
float speed = 1.5;
string name = "Ada";
truth alive = true;
```

| Type | Holds | Examples |
| --- | --- | --- |
| `int` | whole numbers[cite: 1] | `0`, `42`, `-7`[cite: 1] |
| `float` | numbers with a point[cite: 1] | `1.5`, `0.25`[cite: 1] |
| `string` | text, in double quotes[cite: 1] | `"Ada"`, `"line\n"`[cite: 1] |
| `truth` | yes or no[cite: 1] | `true`, `not true`[cite: 1] |
| a class name | one thing made from a class[cite: 1] | `Player`, `Dog`[cite: 1] |
| `T array` | a list of `T`[cite: 1] | `int array`[cite: 1] |
| `cArray(...)` | a connected array | see [cArrays](#carrays) |
| `T map` | values found by a text key[cite: 1] | `int map`, see [Maps](#maps) |
| `state` | this file's state[cite: 1] | see [state](#state) |
| `do int(int)` | a function, kept as a value[cite: 1] | see [Functions as values](#functions-as-values) |
| interop type | a value from a C# or Lua library | see [Interop](#c-and-lua-interop) |

Changing a variable uses `=`[cite: 1]:

```
lives = lives - 1;
```

**Strict rules:**
- A variable keeps its type forever. `lives = "many";` is an error[cite: 1].
- An `int` fits where a `float` goes: `float speed = 2;` makes `speed` `2.0`. The other way round needs `.int()`[cite: 1].
- A variable must be declared before it's used[cite: 1].
- A variable declared inside `{ }` only exists inside those braces. Once the block ends, the name is free again[cite: 1].
- A name can't be declared again while it still exists[cite: 1].

### nothing

`nothing` represents a missing or empty reference (null). Only class-typed values or references can be `nothing`[cite: 1]:

```
Player friend = nothing;    // fine
int score = nothing;        // error: an int always has a number
```

Using a field or method on `nothing` stops the program with an error naming the line[cite: 1].

## Truth and checks

There is no `false`. The two truth values are `true` and `not true`. `not` flips any truth: `not alive`[cite: 1].

### is … ? then … or

Lust uses questions starting with `is` and ending with `?` instead of traditional `if` statements[cite: 1]:

```
is hp 0? then {
    print("out cold");
} or is hp < 5? then {
    print("barely standing");
} or {
    print("fine");
}
```

- `then { }` runs when yes. `or is …? then { }` asks another question. A final `or { }` catches everything else[cite: 1].
- Checks combine using `&&` and `||`[cite: 1].

### either

`either` is the inline expression version that returns a value[cite: 1]:

```
string mood = either hp > 10? "happy" or "grumpy";
```

## Math and text

- Standard operators `+ - * / %` work on `int` and `float`[cite: 1].
- `int / int` yields an `int`. Use `.float()` for precise division[cite: 1].
- `+` joins strings. Non-string types must be converted explicitly using `.text()`[cite: 1].

```
print("Lives: " + lives.text());   // fine
```

## Arrays

```
int array scores = [10, 20, 30];
string array names = [];
```

- Arrays are 0-indexed (`scores[0]`)[cite: 1].
- `#scores` counts elements; `#"hello"` counts text length[cite: 1].
- Methods include `:inject(val)` and `:remove(index)`[cite: 1].

## cArrays

Connected arrays manage tabular/heterogeneous data with automatic local IDs:

```
cArray(string itemName, int amount) bag = [
    [1, "sword", 1],
    [2, "apple", 3]
];
```

- `bag[2]` looks up by row ID[cite: 1].
- `bag:inject(["map", 1]);` auto-assigns a unique tracking ID[cite: 1].
- `bag:remove(2);` and `bag:has(2);` manage rows[cite: 1].

## Maps

Maps store values looked up by string keys:

```
int map prices = ["apple" 3, "seed" 1];
```

- Access via `prices["apple"]`, modify with `prices["pear"] = 5;`, check with `prices:has("pear")`[cite: 1].

## state and Memory Ownership

A state lists options an entity or file can adopt[cite: 1]. **States double as memory and processing arenas**: data and entities instantiated within a state are automatically kept alive and active *only* while that state is engaged. Transitioning out drops the context, instantly reclaiming memory and halting background processing without traditional garbage collection stalls.

```
state GameState = [
    MainMenu,
    ActiveGame [
        Exploration,
        Combat,
    ],
];
```

- Change states using `is`: `state is GameState.ActiveGame.Combat;`[cite: 1].
- States can be nested inside options using `[ ]`[cite: 1].

## Classes and objects

```
class Dog {
    has {
        string[name "Rex"];
        int[hp 10];
        state is happy;
    }

    method bark() {
        print(name + " says woof");
    }
}
```

- Fields are declared inside a single `has { }` block using bracketed grouping[cite: 1].
- Instantiate with `Dog bo = new Dog { name "Bo" };`[cite: 1].
- Access fields with `.` and methods with `:`[cite: 1].
- Use `like` for class inheritance[cite: 1].

## Functions and methods

Functions use `do` (top-level) or `method` (inside classes/objects)[cite: 1]. **`return`** handles value exportation and early exits.

```
do int double(int n) {
    return n * 2;
}
```

- Functions can be passed as values (`do int(int) f = double;`)[cite: 1].

## Loops

`loop` handles all iteration loops: forever, count-based, conditional, and collection traversal[cite: 1].

```
loop 3 { }
loop (hp > 0) { }
loop (int n in scores) { }
loop (int id, string itemName, int amount in bag) { }
```

Use `stop;` to break and `skip;` to continue[cite: 1].

## Sharing between files

- `send Target;` exports definitions from a module[cite: 1].
- `fetch path/file:Target;` or `get path/file:func;` imports symbols[cite: 1].
- **`main.lust`** serves as the master entry point for the application workspace[cite: 1].

## Saving and loading

Saves persist state data cleanly to disk under user directories:

```
save game = [Farm, Player, day];

is saved game? then {
    load game;
}
save game;
```

## C# and Lua Interop

Lust supports robust multi-backend compilation targets (Native machine code, WebAssembly, and Lua via **LustScript**):

- **C# Interop:** `fetch cs:System.Math;` brings in external .NET components[cite: 1].
- **Lua Interop (`fetch lua;`):** When targeting **LustScript**, adding `fetch lua;` at the top of a file unlocks native Lua code execution anywhere. External Lua runtimes can manipulate Lust application states seamlessly via exposed hooks:
  ```csharp
  fetch lua;
  
  method bindModListener() {
      GetEngine():Listen(function(ev)
          Lust.GameState:hook("Paused");
      end);
  };
  ```

## Windows and UI (`ui`)

`fetch ui;` unlocks built-in UI layout capabilities (powered by native windows underneath) utilizing component functional templates[cite: 1].

```
fetch ui;

object App like Window {
    has {
        string[title "App"];
        int[width 400, height 300];
    }
    method build() {
        add(new Label { text "Welcome to Lust" });
    }
}

App:open();
end
```

## Built-ins, Errors, and Testing

- **Built-ins:** `print(x);`, `wait seconds;`, `random(low, high)`, `ask(q)`, `readText(f)`, `writeText(f, w)`, and type cast extensions (`.text()`, `.int()`, `.float()`, `.isNumber()`)[cite: 1].
- **Error Handling:** Clean syntax isolation via `try { ... } error (string err) { ... }` blocks.
- **Testing:** Comprehensive test suite capability via `lust test <folder>` mapping `.out`, `.err`, and `.in` files[cite: 1].

## Reserved words

```
class object has method do give new nothing
is then or either not true like
loop stop skip in
state send fetch get end
int float string truth array cArray map
save load saved try error
```

`print`, `wait`, `random`, `ask`, `readText`, and `writeText` are reserved built-ins[cite: 1].

end
