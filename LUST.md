# The Lust rulebook

Lust is a small, strict language for describing things that have a state, carry stuff, and change over time. It has C#'s skeleton, a Lua accent, and checks written as questions:

```
is hp <= 0? then {
    state is knocked_out;
}
```

This file is the source of truth. The compiler follows it, and when the two disagree, one of them gets fixed.

**Contents:** [Files](#files) · [Types and variables](#types-and-variables) · [Truth and checks](#truth-and-checks) · [Math and text](#math-and-text) · [Arrays](#arrays) · [cArrays](#carrays) · [state](#state) · [Classes and objects](#classes-and-objects) · [Functions](#functions-do) · [Loops](#loops) · [Sharing between files](#sharing-between-files) · [Built-ins](#built-ins) · [Errors](#errors) · [How Lust runs](#how-lust-runs) · [Reserved words](#reserved-words) · [Still open](#still-open)

---

## Files

- A Lust file ends in `.lust`.
- Every statement ends with `;`.
- Every file ends with the word `end`. Nothing may come after it except blank lines and comments.
- Comments start with `//` and run to the end of the line. There are no block comments.
- Inside a file, the order is: `fetch` and `get` lines first, then everything else.

```
// hello.lust
print("Hello!");

end
```

## Types and variables

A variable is declared with its type first, and must be given a value:

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
| `truth` | yes or no | `true`, `not true` |
| a class name | one thing made from a class | `Player`, `Dog` |
| `T array` | a list of `T` | `int array` |
| `cArray(...)` | a connected array | see [cArrays](#carrays) |
| `state` | this file's state | see [state](#state) |

Changing a variable uses `=`:

```
lives = lives - 1;
```

**Strict rules:**
- A variable keeps its type forever. `lives = "many";` is an error.
- A variable must be declared before it's used.
- A variable declared inside `{ }` only exists inside those braces. Once the block ends, the name is free again.
- A name can't be declared again while it still exists, not even inside an inner block. `int hp = 1;` followed by `loop 2 { int hp = 2; }` is a mistake, because it's never clear which `hp` is meant.

### nothing

`nothing` means "no thing here yet". Only class-typed values can be `nothing`:

```
Player friend = nothing;    // fine
int score = nothing;        // error: an int always has a number
```

Using a field or method on `nothing` (`friend.name` when `friend` is `nothing`) stops the program with an error that names the line.

## Truth and checks

There is no `false`. The two truth values are `true` and `not true`. `not` flips any truth: `not alive`.

### is … ? then … or

Lust has no `if`. Decisions are questions that start with `is` and end with `?`:

```
is hp 0? then {
    print("out cold");
} or is hp < 5? then {
    print("barely standing");
} or {
    print("fine");
}
```

Inside a question, a **check** is one of:

| Check | Means | Example |
| --- | --- | --- |
| `a b` | a equals b | `is name "Bo"?` |
| `a not b` | a does not equal b | `is alive not true?` |
| `a < b`, `a > b`, `a <= b`, `a >= b` | comparing numbers | `is hp <= 0?` |
| a truth on its own | that truth is true | `is alive?` |

Checks join with `&&` (both) and `||` (either one):

```
is hungry true && bo.state sleepy? then {
    print("snack, then nap");
}
```

- `then { }` runs when the answer is yes. `or is …? then { }` asks another question. A last `or { }` catches everything else. Both `or` parts are optional.
- Comparing two different types (`is hp "zero"?`) is an error.
- A negative number right after the first value reads as subtraction, so write `is x (-1)?`.

### either

`either` is the one-line version that gives back a value:

```
string mood = either hp > 10? "happy" or "grumpy";
int bonus = either bo.state angry? 10 or 0;
```

Both sides must be the same type.

### Truth values outside questions

`<`, `>`, `<=` and `>=` give a truth anywhere, so `truth low = hp < 5;` works. "Equals" only exists inside a question: `truth same = either a b? true or not true;`.

## Math and text

- `+ - * / %` work on `int` and `float`. Parentheses `( )` group: `(a + b) * 2`.
- An `int` and a `float` together give a `float`. A `float` never quietly becomes an `int`; use `.int()`, which drops the part after the point.
- `int / int` gives an `int` (`7 / 2` is `3`). Use `7.float() / 2` for `3.5`.
- An `int` that gets too big, or dividing an `int` by zero, stops the program with an error naming the line.
- `+` also joins two strings. Joining a string with anything else is an error: turn it into text first with `.text()`.

```
print("Lives: " + lives.text());   // fine
print("Lives: " + lives);          // error: can't add int to string
```

## Arrays

```
int array scores = [10, 20, 30];
string array names = [];
```

- Arrays start at **0**: `scores[0]` is `10`.
- `#scores` is how many items there are.
- `scores:inject(40);` adds to the end. `scores:remove(0);` removes the item at that position.
- `scores[1] = 25;` replaces an item.
- Every item must match the type. Reading past the end stops the program with an error naming the line.

## cArrays

A connected array is a small table. You name the columns once, then add rows. Every row also gets an `id`, an `int` that is the key you look rows up by.

```
cArray(string itemName, int amount) bag = [
    [1, "sword", 1],
    [2, "apple", 3]
];
```

- `id` is never written in the column list. It's always there, always an `int`, and always unique inside one cArray. Two cArrays can both have an id 1.
- `bag[2]` is the row whose id is 2 (not the third row). `bag[2].itemName` reads a column and `bag[2].amount = 5;` changes one.
- `bag:inject(["map", 1]);` adds a row with the next free id (one more than the biggest so far, or 1). `bag:inject([10, "map", 1]);` picks the id yourself.
- `bag:remove(2);` removes the row with id 2. `bag:has(2)` is a truth.
- `#bag` is how many rows there are.
- An id that's already used, or looking up one that isn't there, is an error.
- Looping visits rows in the order they were added. Name the id first, then every column:

```
loop (int id, string itemName, int amount in bag) {
    print(id.text() + ": " + itemName);
}
```

## state

Each file can have one `state`: a list of options something can be in. It goes at the top level of the file.

```
state = [idle, walking, fighting, knocked_out];
```

A class or object that has the state says which option it starts in, in its `has` block:

```
class Player {
    has {
        state is idle;
    }
}
```

- Change it with `is`: `state is fighting;` inside a method, or `bo.state is fighting;` from outside.
- Ask about it in a question: `is bo.state fighting?`.
- `bo.state.text()` gives the option's name as text, like `"fighting"`.
- `loop (state s in state) { }` goes through every option, in the order written.
- From another file, after `fetch player:state;`, the type is called `player.state`. The option names work as they are, because Lust knows which state is being compared.

## Classes and objects

A **class** is a blueprint: you make as many things from it as you like. An **object** is one ready-made thing, used directly by its name.

```
class Dog {
    has {
        string[name "Rex"];
        int[hp 10, age 3,];
        truth[hungry not true];
        Dog[friend nothing];
        state is happy;
    }

    method bark() {
        print(name + " says woof");
    }
}

object Game {
    has {
        int[score 0];
    }

    method addPoints(int amount) {
        score = score + amount;
    }
}
```

### The has block

Fields live in one `has { }` block at the top of the class or object. Fields of the same type are grouped in brackets, each name followed by its starting value, with no `=`. A trailing comma is fine.

```
has {
    int[hp 20, gold 0, level 1,];
    string array bag [];
    cArray(string itemName, int amount) loot [];
    state is idle;
}
```

Starting values are plain values only: a number, text, `true`, `not true`, `nothing`, `[]`, or rows for a cArray. Any calculating happens in methods.

### Making things

```
Dog bo = new Dog { name "Bo", hp 12 };
Dog rex = new Dog;
```

The braces fill in fields, in the same style as `has`. Anything not mentioned keeps its starting value. There are no constructors. `new` on an object (`new Game`) is an error.

### Using them

- Fields use `.`: `bo.hp`, `bo.hp = 5;`, `Game.score`.
- Methods use `:`: `bo:bark();`, `Game:addPoints(10);`.
- Inside a method, the thing's own fields and methods are used by name alone: `hp`, `bark()`. There is no `self`.

## Functions (do)

`do` makes a function that doesn't belong to anything. `method` makes one inside a class or object. Both work the same way:

```
do greet(string who) {
    print("Hello " + who);
}

do int double(int n) {
    give n * 2;
}
```

- The return type goes before the name, and is left out when nothing comes back. There is no `void`.
- `give` hands a value back and ends the function. A function with a return type must `give` on every path; one without may only use `give;` on its own to leave early.
- Parameters are typed: `(int a, string b)`.

## Loops

`loop` is the only loop:

```
loop { }                          // forever, until stop
loop 3 { }                        // three times
loop (hp > 0) { }                 // while that's true
loop (int n in scores) { }        // each item of an array
loop (int id, string itemName, int amount in bag) { }   // each row of a cArray
loop (state s in state) { }       // each option of a state
```

`stop;` leaves the loop. `skip;` jumps to the next round.

## Sharing between files

Everything in a file is private until it's sent:

```
// player.lust
send Player;
send heal;
send state;
```

Another file in the same folder brings things in with `fetch` (big things: classes, objects, state) and `get` (single functions). `:` means "belongs to": file name on the left, thing on the right.

```
fetch player:Player;
fetch player:[Player, Party];
fetch player;                  // everything player.lust sends
get dice:roll;
get dice:[roll, flip];
```

- `fetch` and `get` lines come before anything else in the file.
- Fetching something the other file doesn't send is an error.
- Only the file you run has its top-level code run. A fetched file only lends its classes, objects, functions and state, so test code at the bottom of `player.lust` doesn't run when `main.lust` fetches it.

## Built-ins

| Built-in | Does |
| --- | --- |
| `print(x);` | shows `x` on its own line. Takes any type. |
| `wait seconds;` | pauses, then carries on (see below). Takes an `int` or `float`. |
| `random(low, high)` | a whole number from `low` to `high`, both included |
| `x.text()` | `x` as text. Works on `int`, `float`, `truth` and state options. A float always shows its point (`3.0`), and a truth shows as `true` or `not true`. |
| `x.int()` | a `float` with the part after the point dropped |
| `x.float()` | an `int` as a `float` |

### wait

`wait` pauses only the code that's running it. In a plain program only one thing runs at a time, so it pauses the whole program. Once the game engine runs many objects side by side, a `wait` inside one object's code pauses just that object while the rest carry on.

## Errors

Lust finds as many mistakes as it can before the program starts, and says where and why:

```
player.lust:14:22  can't add int to string. Use hp.text() to turn hp into text.
player.lust:31:5   missing ; at the end of this line.
main.lust:3:1      player.lust doesn't send Party.
```

Mistakes only visible while running (a missing cArray id, reading past the end of an array, using `nothing`) stop the program with a message in the same style.

## How Lust runs

Lust is compiled. `lust run main.lust` reads `main.lust` and everything it fetches, checks the rules above, turns the result into C#, and runs it with .NET. `lust build main.lust` makes an `.exe` instead, in a `build` folder next to the file.

You need the .NET SDK (version 8 or newer) installed. The compiler lives in [`compiler/`](compiler), and `lust cs main.lust` shows the C# it writes.

## Reserved words

These can't be used as names:

```
class object has method do give new nothing
is then or either not true
loop stop skip in
state send fetch get end
int float string truth array cArray
```

`print`, `wait` and `random` are built-ins and can't be reused as function names.

## Still open

Decisions for later, so nobody guesses:

- `+=` and friends (`hp += 5;`). Left out for now.
- Fetching a single method, as in `fetch dog:Dog[bark]`.
- Reading typed input, such as `ask("Name?")`, and turning text into numbers.
- A method and a `do` function with the same name: which one a bare call means.
- Whether `nothing` is allowed for arrays and cArrays.
