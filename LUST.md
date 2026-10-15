# The Lust rulebook

Lust is a small, strict language for describing things that have a state, carry stuff, and change over time. It has C#'s skeleton, a Lua accent, and checks written as questions:

```
is hp <= 0? then {
    state is knocked_out;
}
```

This file is the source of truth. The compiler follows it, and when the two disagree, one of them gets fixed.

**Contents:** [Files](#files) · [Types and variables](#types-and-variables) · [Truth and checks](#truth-and-checks) · [Math and text](#math-and-text) · [Arrays](#arrays) · [cArrays](#carrays) · [Maps](#maps) · [state](#state) · [Classes and objects](#classes-and-objects) · [Functions](#functions-do) · [Loops](#loops) · [Sharing between files](#sharing-between-files) · [Saving and loading](#saving-and-loading) · [C# libraries](#c-libraries) · [Windows and buttons](#windows-and-buttons-ui) · [Built-ins](#built-ins) · [Errors](#errors) · [Testing](#testing) · [How Lust runs](#how-lust-runs) · [Reserved words](#reserved-words) · [Still open](#still-open)

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
| `T map` | values found by a text key | `int map`, see [Maps](#maps) |
| `state` | this file's state | see [state](#state) |
| `do int(int)` | a function, kept as a value | see [Functions as values](#functions-as-values) |
| a C# type | a value from a C# library | `Random`, see [C# libraries](#c-libraries) |

Changing a variable uses `=`:

```
lives = lives - 1;
```

**Strict rules:**
- A variable keeps its type forever. `lives = "many";` is an error.
- An `int` fits where a `float` goes: `float speed = 2;` makes `speed` `2.0`. The other way round needs `.int()`.
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
- Things made from classes, arrays and cArrays are the same only when they're the very same thing, not when they hold the same values. To check if an array is empty, ask `is #scores 0?`.
- Parts of a check can be grouped with `( )`: `is ready && (hp 0 || state fighting)?`
- A negative number right after the first value reads as subtraction, so write `is x (-1)?`.

### either

`either` is the one-line version that gives back a value:

```
string mood = either hp > 10? "happy" or "grumpy";
int bonus = either bo.state angry? 10 or 0;
```

Both sides must be the same type, except that an `int` and a `float` mix into a `float`: `either hungry? 1 or 1.5` gives `1.0` or `1.5`.

### Truth values outside questions

`<`, `>`, `<=` and `>=` give a truth anywhere, so `truth low = hp < 5;` works. "Equals" only exists inside a question: `truth same = either a b? true or not true;`.

## Math and text

- `+ - * / %` work on `int` and `float`. Parentheses `( )` group: `(a + b) * 2`.
- An `int` and a `float` together give a `float`. A `float` never quietly becomes an `int`; use `.int()`, which drops the part after the point.
- `int / int` gives an `int` (`7 / 2` is `3`). Use `7.float() / 2` for `3.5`.
- An `int` that gets too big, or dividing an `int` by zero, stops the program with an error naming the line. Dividing a `float` by zero isn't a mistake: it gives `infinity` (or `-infinity`), and the program carries on.
- `+` also joins two strings. Joining a string with anything else is an error: turn it into text first with `.text()`.

```
print("Lives: " + lives.text());   // fine
print("Lives: " + lives);          // error: can't add an int to a string
```

## Arrays

```
int array scores = [10, 20, 30];
string array names = [];
```

- Arrays start at **0**: `scores[0]` is `10`.
- `#scores` is how many items there are. `#` on text counts its letters: `#"hello"` is `5`.
- `scores:inject(40);` adds to the end. `scores:remove(0);` removes the item at that position.
- `scores[1] = 25;` replaces an item.
- Every item must match the type. Reading past the end stops the program with an error naming the line.
- A loop goes through a copy of the array, so injecting or removing inside the loop is safe.
- `#` reads the whole thing after it, `.text()` included: `#scores.text()` means `#(scores.text())`. Write `(#scores).text()` for the count as text.

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
- `bag:inject(["map", 1]);` adds a row with a new id: one more than the biggest id the cArray has ever had, starting at 1. Ids are never reused, even after a row is removed. `bag:inject([10, "map", 1]);` picks the id yourself.
- `bag:remove(2);` removes the row with id 2. `bag:has(2)` is a truth.
- `#bag` is how many rows there are.
- An id that's already used, or looking up one that isn't there, is an error.
- Looping visits rows in the order they were added. Name the id first, then every column:

```
loop (int id, string itemName, int amount in bag) {
    print(id.text() + ": " + itemName);
}
```

## Maps

A map finds values by a text key. It's written like a has block: each key, then its value.

```
int map prices = ["apple" 3, "seed" 1];
string map names = [];
```

- `prices["apple"]` reads a value. Reading a key that isn't there stops the program with an error naming the line, so check first with `prices:has("pear")`.
- `prices["pear"] = 5;` changes a value, or adds the key when it's new.
- `prices:remove("seed");` removes a key. `#prices` is how many keys there are.
- Keys are always text. Values can be any one type: `Dog map`, `int array map`, a state.
- A negative value goes right after its key: `["cold" -5]`.
- Looping goes through the keys in the order they were first added. Name the key, then the value:

```
loop (string item, int price in prices) {
    print(item + " costs " + price.text());
}
```

## state

A state is a list of options something can be in. Each file can have one **main state**, and any number of **named states** for when one isn't enough. They go at the top level of the file.

```
state = [idle, walking, fighting, knocked_out];   // the main state
state season = [spring, summer, fall, winter];     // named states
state weather = [sunny, rainy, stormy];
```

A class or object that has a state says which option it starts in, in its `has` block. The main state is written `state is …`, and a named one uses its name:

```
class Farm {
    has {
        state is idle;
        season is spring;
        weather is sunny;
    }
}
```

- Change it with `is`: `season is summer;` inside a method, or `farm.season is summer;` from outside. To copy a state from somewhere else, use `=`: `farm.season = nextSeason(farm.season);`, or `state = other.state;` for the main state.
- Ask about it in a question: `is farm.season winter?`.
- `farm.season.text()` gives the option's name as text, like `"winter"`.
- `loop (season s in season) { }` goes through every option, in the order written. For the main state: `loop (state s in state) { }`.
- A state can also be a variable's type: `season today = spring;`. It's changed with `is` too: `today is fall;`, or with `=` from another state value.
- A class or object can have each state once at most.
- Two states in the same file may share an option name. Lust always knows which state is meant from what it's compared with.
- From another file, after `fetch farm:season;`, the type is called `farm.season`, or just `season` when this file has no state of its own with that name. Another file's main state is always `farm.state`. The option names work as they are.

### States inside states

An option can hold options of its own, in [ ] right after it. That's handy for anything with a broad mode and finer details, like an enemy's brain or an app's screens:

```
state = [idle, walking, fighting [attacking, blocking, fleeing [running, hiding]]];
```

- An option inside another is named with a dot: `state is fighting.blocking;`, `state is fighting.fleeing.hiding;`.
- A question about an option is also yes for everything inside it: when the state is `fighting.blocking`, both `is state fighting?` and `is state fighting.blocking?` are yes, and `is state fighting.attacking?` is no. In general, `is a b?` with states asks "is a the same as b, or inside it?"
- `fighting` on its own is an option too, for when the details don't matter yet.
- `.text()` and `print` show the full name: `fighting.blocking`.
- `loop (state s in state)` goes through every option, inside ones included, in the order written.
- An option inside another always needs its full name. Writing `blocking` alone is a mistake, because the same name could sit inside two options.

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

Starting values are plain values only: a number, text, `true`, `not true`, `nothing`, `[]`, rows for a cArray, or a C# value made from plain values (`new DateTime(2026, 1, 1)`). Any calculating happens in methods.

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

### like: building on another class

`like` makes a class that starts as a copy of another one and adds to it:

```
class Wizard like Player {
    has {
        int[mana 50];
        int[hp 15];          // Player has hp too: this changes its starting value
    }

    method int attack() {    // replaces Player's attack for wizards
        give mana / 10 + random(1, 6);
    }
}

Wizard merlin = new Wizard { name "Merlin" };
```

- A Wizard gets every field, state and method that Player has.
- Its `has` block adds new fields. Listing one Player already has, with the same type, only changes its starting value.
- A method with the same name as one of Player's replaces it for wizards. It must take the same types of parameters (their names can differ) and give back the same type.
- A Wizard can go anywhere a Player is expected: a `Player` variable, a `Player array`, a `Player` parameter. Methods called on it use the Wizard's version.
- Ask what something really is with `is`: `is hero like Wizard? then { … }`.
- A class is like one other class at most. An object can be like a class too: `object Boss like Wizard { … }`.

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
- `do` and `method` can't go inside another function or method. Every function sits at the top level of its file, and every method directly inside its class or object.
- Functions and methods can't see variables declared at the top of a file. They work with their parameters, their own fields, objects and other functions. To share a value between functions, keep it in an object.
- Inside a method, a bare call like `bark()` means the class's own method when there is one, and a `do` function otherwise.
- A parameter or variable inside a method can't use the name of one of its class's fields, since fields are used by name alone.

### Functions as values

A function can be kept in a variable, a field, an array or a map, and passed to another function, for events like "when the day ends, run this". The type says what the function takes, and what it gives back when it gives something:

```
do int(int) f = double;           // takes an int, gives back an int
do(string) say = greet;           // takes a string, gives back nothing
do() later = bo:bark;             // a method of one thing: later() makes bo bark

print(f(5));                      // 10
later();

do int twice(do int(int) step, int x) {
    give step(step(x));
}
print(twice(double, 3));          // 12
```

- A function's name without `()` is the function itself. A method is written `thing:method` without `()`, and stays tied to that thing. Inside a method, the class's own methods work by name alone: `bark`.
- Call what a variable or parameter holds like a function: `f(5)`. A field that holds one is called like a method: `bo:onBark("hi")`, or `onBark("hi")` inside the class. From an array: `jobs[0]()`, and from a cArray's column: `bag[1]:onUse()`.
- Parameter names may be written for clarity, and are ignored: `do(string who)` is the same type as `do(string)`.
- A function only fits where it takes and gives back exactly the same types.
- A function value can be `nothing`, which suits events nobody has filled in yet: `do() [onDayEnd nothing];` in a has block. Calling `nothing` stops the program with an error, so check first: `is onDayEnd nothing?`.
- `is a b?` asks whether two function values are the same function (for a method, of the same thing).
- Built-ins like `print` can't be kept as values. Wrap them: `do show(string s) { print(s); }`.

## Loops

`loop` is the only loop:

```
loop { }                          // forever, until stop
loop 3 { }                        // three times
loop (hp > 0) { }                 // while that's true
loop (cmd not "quit") { }         // the check works like a question's
loop (int n in scores) { }        // each item of an array
loop (int id, string itemName, int amount in bag) { }   // each row of a cArray
loop (string key, int value in prices) { }              // each key of a map
loop (state s in state) { }       // each option of a state
```

The check in `loop ( )` works exactly like a question's, so `a b`, `a not b`, `&&` and `||` all work there.

`stop;` leaves the loop. `skip;` jumps to the next round.

## Sharing between files

Everything in a file is private until it's sent:

```
// player.lust
send Player;
send heal;
send state;
```

Another file brings things in with `fetch` (big things: classes, objects, state) and `get` (single functions). `:` means "belongs to": file name on the left, thing on the right.

```
fetch player:Player;
fetch player:[Player, Party];
fetch player;                  // everything player.lust sends
get dice:roll;
get dice:[roll, flip];
```

Files can sit in folders. The path starts from the folder of the file doing the fetching, with `/` between folders and `../` to go up one:

```
// game/main.lust
fetch world/farm:[Farm, season];     // game/world/farm.lust
get shared/dice:roll;                // game/shared/dice.lust

// tests/farm_test.lust
fetch ../game/world/farm;            // up out of tests, into game/world
```

- `fetch` and `get` lines come before anything else in the file.
- `fetch farm;` looks for `farm.lust` right next to this file. Leave out `.lust`, and don't start with `./`.
- Things from a file are named by the file's own name, whatever folder it's in: after `fetch world/farm:season;` the state is `farm.season`. So one file can't fetch two different files that are both called `farm.lust`.
- Fetching something the other file doesn't send is an error.
- `fetch farm;` brings in everything `farm.lust` sends, functions included.
- Only the file you run has its top-level code run. A fetched file only lends its classes, objects, functions and state, so test code at the bottom of `player.lust` doesn't run when `main.lust` fetches it.

## Saving and loading

A save keeps things between runs of a program. Declare it at the top level of a file, listing what it holds, like declaring a state:

```
save game = [Farm, Player, day];      // objects, and variables from the top of the file

save game;                            // writes them all
load game;                            // puts them all back
is saved game? then { load game; }    // has it been saved before?
```

- Everything inside is saved: every field, array, map, cArray and state, and every thing they point to. Two fields that held the same thing still hold the same thing after loading, and the cArray ids that were used stay used.
- `load` puts the values back into those same objects and variables. Load after the variables are declared, or their declaration replaces what was loaded.
- A save can hold objects, and variables declared at the very top of the file you run. Things made from classes go in through them: in a variable, an array, an object's field.
- Functions and C# values can't be saved; Lust says so before running. Window parts (labels, buttons, canvases) are skipped, since `build()` makes them again.
- There can be several saves, saved and loaded separately: `save settings = [Options];` next to `save game = …`. A save can be sent and fetched like a class: `send game;`.
- Saves live in your user folder, in AppData\Roaming\Lust\(program)\(save).save on Windows. The program's name is its file's name, or its folder's name when the file is called main.lust.
- A save is text that looks like Lust, so you can open it and change things (the gold, say). Loading is strict: when the save doesn't match the program exactly, for example after adding a field to a class, `load` stops the program with an error that says what's different, and changes nothing. Loading a save that doesn't exist is an error too, so ask `is saved game?` first.

## C# libraries

`fetch cs:` brings in a type from C#, by its full name. After that, it's used by its short name:

```
fetch cs:System.Math;
fetch cs:System.Random;
fetch cs:System.Text.StringBuilder;
fetch cs:Humanizer.NumberToWordsExtension from "Humanizer.Core" 2.14.1;   // from a NuGet package

float root = Math:Sqrt(2.0);                // static methods use :, like Lust's
print(Math.PI);                             // properties and fields use .
Random dice = new Random(42);               // C# types can take values in ( ) when they're made
int roll = dice:Next(1, 7);
StringBuilder sb = new StringBuilder { Capacity 100 };
sb:Append("Hello");
print(NumberToWordsExtension:ToWords(42));  // forty-two
```

- Any public type from .NET works: `System.IO.File`, `System.DateTime`, `System.Text.RegularExpressions.Regex` and so on. A type from a NuGet package names the package and version after `from`; it's downloaded the first time, so that needs the internet.
- Values become Lust's own as they come and go. C# whole numbers are `int`, `double`, `float` and `decimal` are `float`, `string` and `char` are `string`, `bool` is `truth`, and lists and arrays become Lust arrays (a copy). Anything else is a C# value, which Lust keeps and passes back as it is. Its type is written by its short name: `Random dice`, `Random array dice`.
- When a method has several versions, Lust picks the one that fits the values best. Methods that take any number of values (`String:Format("{0} and {1}", a, b)`) take them one by one.
- Lust's own values have C# methods too: `"a,b,c":Split(",")` gives a string array, `"lust":ToUpper()`, `3.14159:ToString("F2")`, and `name.Length` reads a C# property.
- A C# type's name on its own isn't a value: it's always followed by `:` or `.`.
- What a C# method gives back can be left unused (`sb:Append("x");`), unlike a Lust function's.
- `is a b?` compares C# values the C# way, so two equal dates are the same. `x.text()` gives a C# value as text.
- Not yet: generic types (List, Dictionary; use Lust's arrays and maps), events, `out` parameters, and C# methods kept as values.
- When C# stops the program (say, a file that can't be opened), the error shows C#'s message at the Lust line.

## Windows and buttons (ui)

`fetch ui;` brings in Lust's built-in UI file, for programs with a window. These only run on Windows (they use WinForms underneath), but `lust check` works anywhere.

```
fetch ui;

object App like Window {
    has {
        string[title "Counter"];
        int[width 300, height 160];
        int[clicks 0];
        Label[count nothing];
    }

    method build() {
        count = new Label { text "No clicks yet." };
        add(count);
        add(new PlusButton { text "Click me" });
    }
}

class PlusButton like Button {
    method clicked() {
        App.clicks = App.clicks + 1;
        App.count.text = "Clicks: " + App.clicks.text();
    }
}

App:open();

end
```

What `ui` sends:

| Class | Has | Methods |
| --- | --- | --- |
| `Window` | `title`, `width`, `height`, `background` | `build()` (yours to fill in), `open()`, `add(Widget w)`, `label(string words)`, `rebuild()`, `alert(string message)`, `close()`, and for things that move: `tick()` and `keyPressed(string key)` (yours), `startTimer(float seconds)`, `stopTimer()`, `isKeyDown(string key)` |
| `Widget` | `text`, `enabled`, `visible`, `color`, `background`, `textSize` | everything below is like Widget |
| `Label` | | text on its own |
| `Button` | | `clicked()`: make your own button like Button and fill this in |
| `Input` | | a box to type in; what's typed is in its `text` |
| `Toggle` | `on` | a tick box; `toggled()` runs when it's ticked or unticked |
| `Row` | | `add(Widget w)`: puts widgets side by side |
| `Picture` | `file`, `width`, `height` | a picture from a file (png, jpg, gif, bmp); a width or height of 0 means its own size |
| `Canvas` | `width`, `height` | somewhere to draw: `rect(x, y, w, h, paint)`, `circle(x, y, radius, paint)`, `line(x1, y1, x2, y2, paint)`, `write(x, y, words, paint)`, `picture(file, x, y)`, `clear()`, and `clicked(int x, int y)` (yours) |

- Make your window `like Window`, usually as an object, and put what it shows in `build()`. `open()` shows it, and the program waits there until the window is closed.
- Widgets go top to bottom in the order they're added. A `Row` puts its widgets side by side.
- Your buttons and toggles say what they do by being `like Button` or `like Toggle` and filling in `clicked()` or `toggled()`.
- To change what's on screen, change fields: `App.count.text = "…";`, `button.enabled = not true;`. After every click, Lust shows the new values. Keep a widget in a field when you want to change it later.
- Before a click runs, what was typed into `Input`s and ticked in `Toggle`s is already in their `text` and `on`.
- `rebuild()` empties the window and runs `build()` again. With a state, that switches screens:

```
state screen = [naming, playing];
…
method build() {
    is screen naming? then { … } or { … }
}
…
Pet.screen is playing;
Pet:rebuild();
```

- `alert("…")` shows a message in a box. A mistake while the window is open shows in a box too, since the window closes.
- **Colours** are names like `"red"`, `"gold"` or `"skyblue"` (any colour name Windows knows), or codes like `"#ff3355"`. `""` means the usual colour. `color` is a widget's text colour, `background` what's behind it, and `textSize` its text size (0 is the usual size).
- **Pictures** and files on a canvas are found next to the `.lust` file, or next to the `.exe` once it's built, so copy them there.
- **Drawing:** a canvas keeps everything drawn on it until `clear()`. x and y count from its top left corner. A game usually clears and draws everything again in each `tick()`.
- **Things that move:** `startTimer(0.05)` makes `tick()` run on its own every 0.05 seconds, until `stopTimer()`. After every tick, Lust shows the new values, just like after a click.
- **Keys:** `keyPressed(key)` runs when a key goes down, and `isKeyDown(key)` says whether one is held right now, which suits movement in `tick()`. Keys are named `"A"` to `"Z"`, `"0"` to `"9"`, `"Space"`, `"Enter"`, `"Escape"`, `"Tab"`, `"Backspace"`, `"Left"`, `"Right"`, `"Up"`, `"Down"`, `"Shift"`, `"Control"`, `"Alt"` and `"F1"` to `"F12"`.

[`examples/ui/stars.lust`](examples/ui/stars.lust) is a small game with all of these.

## Built-ins

| Built-in | Does |
| --- | --- |
| `print(x);` | shows `x` on its own line. Takes any type: a thing made from a class shows like its has block (`Dog { name "Rex", hp 10 }`), an array as `[1, 2, 3]`, and `nothing` as `nothing`. |
| `wait seconds;` | pauses, then carries on (see below). Takes an `int` or `float`. |
| `random(low, high)` | a whole number from `low` to `high`, both included |
| `x.text()` | `x` as text. Works on `int`, `float`, `truth` and state options. A float always shows its point (`3.0`), and a truth shows as `true` or `not true`. |
| `x.int()` | a `float` with the part after the point dropped |
| `x.float()` | an `int` as a `float` |
| `ask(question)` | shows the question, waits for a line to be typed, and gives it back as a `string`. Gives back `""` when there's nothing more to read. |
| `text.int()`, `text.float()` | text turned into a number, like `"42".int()`. `"3.5".int()` drops the part after the point and gives `3`. Text that isn't a number stops the program, so check first with `.isNumber()`. |
| `text.isNumber()` | `true` when the text is a number |
| `text.lines()` | the text cut into its lines, as a `string array` |
| `readText(file)` | the whole text file, as a `string` |
| `writeText(file, words)` | writes the text into the file, replacing what was there (and making its folder if needed) |

Text files are found next to the `.lust` file you run, or next to the program once it's built with `lust build`. A full path like `C:\notes\todo.txt` works too. Reading a file that isn't there stops the program with an error.

### wait

`wait` pauses only the code that's running it. In a plain program only one thing runs at a time, so it pauses the whole program. Once the game engine runs many objects side by side, a `wait` inside one object's code pauses just that object while the rest carry on.

## Errors

Lust finds as many mistakes as it can before the program starts, and says where and why:

```
player.lust:14:22  can't add an int to a string. Use hp.text() to turn hp into text.
player.lust:31:5  missing ; at the end of this line.
main.lust:3:12  player.lust doesn't send Party. Add send Party; to player.lust.
```

Mistakes only visible while running (a missing cArray id, reading past the end of an array, using `nothing`) stop the program with a message in the same style.

Functions can call each other (and themselves) up to 200000 deep. Past that, Lust stops the program, since that almost always means a function calls itself with no way to stop.

## Testing

`lust test <folder>` runs every `.lust` file in a folder that has an expected result next to it:

- `name.out`: what it should print.
- `name.err`: the mistakes Lust should report, or the error it should stop with.
- `name.in`: what's typed in while it runs, for `ask`.

Folders inside the folder are searched too, and a `.lust` file with no expected result is a helper that other tests fetch. During `lust test`, `random` gives the same numbers every time (the runs are seeded), so tests that use it stay repeatable, and every test starts with no saves. Programs that fetch `ui` can't run as tests, but their mistakes can be checked with a `.err` file.

## How Lust runs

Lust is compiled. `lust run main.lust` reads `main.lust` and everything it fetches, checks the rules above, turns the result into C#, and runs it with .NET. `lust build main.lust` makes an `.exe` instead, in a `build` folder next to the file.

You need the .NET SDK (version 8 or newer) installed. The compiler lives in [`compiler/`](compiler), and `lust cs main.lust` shows the C# it writes.

## Reserved words

These can't be used as names:

```
class object has method do give new nothing
is then or either not true like
loop stop skip in
state send fetch get end
int float string truth array cArray map
save load saved
```

`print`, `wait`, `random`, `ask`, `readText` and `writeText` are built-ins and can't be used as names.

## Still open

Decisions for later, so nobody guesses:

- `+=` and friends (`hp += 5;`). Left out for now.
- Fetching a single method, as in `fetch dog:Dog[bark]`.
- Whether `nothing` is allowed for arrays and cArrays.
- Calling the original version of a method from the one that replaces it (with `like`).
- Functions written right where they're used (lambdas), like `do(int n) { print(n); }` as a value.
- Generic C# types (List, Dictionary), C# events, and keeping C# methods as values.
- Sound, and more widgets (lists, sliders, menus).
