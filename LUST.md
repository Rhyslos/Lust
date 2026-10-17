# The Lust v2 Rulebook

Lust is a small, strict, multi-backend language designed for native applications, web deployment via WebAssembly, and Lua-targeted modding (LustScript). It features C#'s structural backbone, a clean Lua-inspired aesthetic, checks written as questions, state-bound memory ownership, node blocks, assembly factories, and files that always close with end.

is hp <= 0? then {
    state is knocked_out;
}

This file is the source of truth. The compiler follows it, and when the two disagree, one of them gets fixed.

Contents: Files · Types and variables · Truth and checks · Math and text · Arrays · cArrays · Maps · state · node and assembly · Classes and objects · Methods · Loops · Sharing between files · Saving and loading · C# and Lua Interop · Windows and UI · Built-ins · Errors · How Lust runs · Reserved words

---

## Files

- Lust project file extensions:
  - .lust — Standard core logic (Native / WebAssembly).
  - .lql — Lust Query Language for embedded databases and schemas.
  - .lsc — LustScript for Lua-targeted modding and runtime scripting.
- Every statement ends with ;.
- Every file ends with the word end. Nothing may come after it except blank lines and comments.
- Comments start with // or Lua-style -- and run to the end of the line. There are no block comments.
- Inside a file, the order is: fetch and get lines first, then everything else.
- Every project requires a main.lust master file at its root, serving as the core execution anchor.

-- hello.lust
print("Hello!");

end

## Types and variables

A variable is declared with its type first, followed by its name, and a value:

int lives = 3;
float speed = 1.5;
string name = "Ada";
truth alive = true;

| Type | Holds | Examples |
| --- | --- | --- |
| int | whole numbers | 0, 42, -7 |
| float | numbers with a point | 1.5, 0.25 |
| string | text, in double quotes | "Ada", "line\n" |
| truth | yes or no | true, not true |
| a class name | one thing made from a class | Player, Dog |
| T array | a list of T | int array |
| cArray(...) | a connected array | see cArrays |
| T map | values found by a text key | int map, see Maps |
| state | this file's state or enum list | see state |
| method int(int) | a callable method kept as a value | see Methods |
| interop type | a value from a C# or Lua library | see Interop |

Changing a variable uses =:

lives = lives - 1;

Strict rules:
- A variable keeps its type forever. lives = "many"; is an error.
- An int fits where a float goes: float speed = 2; makes speed 2.0. The other way round needs .int().
- A variable must be declared before it's used.
- A variable declared inside { } only exists inside those braces. Once the block ends, the name is free again.
- A name can't be declared again while it still exists.

### nothing

nothing represents a missing or empty reference (null). Only class-typed values or references can be nothing:

Player friend = nothing;    // fine
int score = nothing;        // error: an int always has a number

Using a field or method on nothing stops the program with an error naming the line.

## Truth and checks

There is no false. The two truth values are true and not true. not flips any truth: not alive.

### is … ? then … or

Lust uses questions starting with is and ending with ? instead of traditional if statements:

is hp 0? then {
    print("out cold");
} or is hp < 5? then {
    print("barely standing");
} or {
    print("fine");
}

- then { } runs when yes. or is …? then { } asks another question. A final or { } catches everything else.
- Checks combine using && and ||.

### either

either is the inline expression version that returns a value:

string mood = either hp > 10? "happy" or "grumpy";

## Math and text

- Standard operators + - * / % work on int and float.
- int / int yields an int. Use .float() for precise division.
- + joins strings. Non-string types must be converted explicitly using .text().

print("Lives: " + lives.text());    // fine

## Arrays

int array scores = [10, 20, 30];
string array names = [];

- Arrays are 0-indexed (scores[0]).
- #scores counts elements; #"hello" counts text length.
- Methods include :inject(val) and :remove(index).

## cArrays

Connected arrays manage tabular/heterogeneous data with automatic local IDs:

cArray(string itemName, int amount) bag = [
    [1, "sword", 1],
    [2, "apple", 3]
];

- bag[2] looks up by row ID.
- bag:inject(["map", 1]); auto-assigns a unique tracking ID.
- bag:remove(2); and bag:has(2); manage rows.

## Maps

Maps store values looked up by string keys:

int map prices = ["apple" 3, "seed" 1];

- Access via prices["apple"], modify with prices["pear"] = 5;, check with prices:has("pear").

## state and Memory Ownership

A state lists options an entity or file can adopt, doubling as Lust's enum system. States double as memory and processing arenas: data and entities instantiated within a state are automatically kept alive and active only while that state is engaged. Transitioning out drops the context, instantly reclaiming memory and halting background processing without traditional garbage collection stalls.

state GameState = [
    MainMenu,
    ActiveGame [
        Exploration,
        Combat,
    ],
];

- Change states using is: state is GameState.ActiveGame.Combat;.
- States can be nested inside options using [ ].

## node and assembly

### node (Scoped Grouping Node)
A node block creates a scoped structural block that bundles entities, local variables, and sub-states together under a single container (such as a level segment, UI panel, or AI subsystem).

node RoomOne {
    state is Active;
    Player hero = new Player {};
}

When a node block is closed or exited, all entities and allocations defined inside it are automatically freed from memory.

### assembly (Factory Blueprint)
An assembly is a dedicated factory blueprint designed for spawning instances cleanly without class constructor boilerplate.

assembly LootSpawner {
    create Item {
        string name = "Health Potion";
        int value = 50;
    }

    create Weapon {
        string name = "Iron Sword";
        int damage = 15;
    }
}

Item drop = LootSpawner:create(Item);
Weapon sword = LootSpawner:create(Weapon);

## Classes and objects

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

- Fields are declared inside a single has { } block using bracketed grouping.
- Instantiate with Dog bo = new Dog { name "Bo" };.
- Access fields with . and methods with :.
- Use like for class inheritance.

## Methods

All executable subroutines in Lust are declared using the method keyword (whether defined inside classes/objects or standalone at top-level file scope). return handles value exportation and early exits.

method int double(int n) {
    return n * 2;
}

- Methods can be passed as values (method int(int) f = double;).

## Loops

loop handles all iteration loops: forever, count-based, conditional, and collection traversal.

loop 3 { }
loop (hp > 0) { }
loop (int n in scores) { }
loop (int id, string itemName, int amount in bag) { }

Use stop; to break and skip; to continue.

## Sharing between files

- send Target; exports definitions from a module.
- fetch path/file:Target; or get path/file:func; imports symbols.
- main.lust serves as the master entry point for the application workspace.

## Saving and loading

Saves persist state data cleanly to disk under user directories:

save game = [Farm, Player, day];

is saved game? then {
    load game;
}
save game;

## C# and Lua Interop

Lust supports robust multi-backend compilation targets (Native machine code, WebAssembly, and Lua via LustScript using .lsc files):

- C# Interop: fetch cs:System.Math; brings in external .NET components.
- Lua Interop (fetch lua;): When targeting LustScript, adding fetch lua; unlocks native Lua code execution. External Lua runtimes can manipulate Lust application states seamlessly via exposed hooks:
  
  fetch lua;
  
  method bindModListener() {
      GetEngine():Listen(function(ev)
          Lust.GameState:hook("Paused");
      end);
  };

## Windows and UI (ui)

fetch ui; unlocks built-in UI layout capabilities utilizing component functional templates.

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

## Built-ins, Errors, and Testing

- Built-ins: print(x);, wait seconds;, random(low, high), ask(q), readText(f), writeText(f, w), and type cast extensions (.text(), .int(), .float(), .isNumber()).
- Error Handling: Clean syntax isolation via try { ... } error (string err) { ... } blocks.
- Testing: Comprehensive test suite capability via lust test <folder> mapping .out, .err, and .in files.

## Reserved words

class object node assembly create has method give new nothing
is then or either not true like
loop stop skip in
state send fetch get end
int float string truth array cArray map
save load saved try error

print, wait, random, ask, readText, and writeText are reserved built-ins.

end
