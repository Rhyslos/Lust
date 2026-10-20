<img src="assets/lust.svg" width="96" alt="Lust logo">

# Lust

A small, strict language for apps, games, tools and scripting. Checks are written as questions, states are built in, and every file ends with `end`.

```
state Mood = [Cheerful, Grumpy];

class Player {
    has {
        string name = "Hero";
        int hp = 20;
        Mood mood = Mood.Cheerful;
    }

    method hurt(int damage) {
        hp = hp - damage;

        is hp <= 0? then {
            print(name + " is knocked out!");
        } or is hp < 10? then {
            mood = Mood.Grumpy;
            print(name + " has " + hp.text() + " hp left.");
        }
    }
}

Player ada = new Player { name = "Ada" };
ada:hurt(12);

end
```

## Lust 2

This is **Lust 2**, a fresh start written in Rust. One language, two outputs:

| Output | Files | For |
| --- | --- | --- |
| Rust, compiled to real programs | `.lust` | apps, tools and servers on Windows, macOS and Linux *(coming next)* |
| LustScript, turned into Lua | `.lsc` | scripting and modding inside other programs, like Garry's Mod, Aventail and Halberd |

**What works today (alpha 1):** LustScript. Every `.lsc` file is checked before it runs, with mistakes explained in plain words, then turned into plain Lua 5.1 that Garry's Mod (LuaJIT) and Luau can run. `lust run` runs a file straight away, with Lua built into `lust` itself, so nothing else needs installing.

The full rules are in **[LUST.md](LUST.md)**, and the long-term plan is in [IDEAS.md](IDEAS.md). Lust 1, the version that compiled to C#, lives on the [`v1`](https://github.com/Rhyslos/Lust/tree/v1) branch.

## Try it

You need [Rust](https://rustup.rs). Then:

```
cargo build --release
target/release/lust run tests/run/player.lsc
```

| Command | Does |
| --- | --- |
| `lust run file.lsc` | checks the file, then runs it. `lust file.lsc` works too |
| `lust check file.lsc` | only checks, and lists any mistakes |
| `lust lua file.lsc` | shows the Lua it turns into |
| `lust build file.lsc` | writes that Lua next to it, as `file.lua`, ready for Garry's Mod or any Lua host |
| `lust test tests` | runs the test programs |

Mistakes come out all at once, before anything runs:

```
basics.lsc:3:1  Lust asks questions instead of if: is hp == 0? then { }
basics.lsc:6:23  can't add an int to a string. Use lives.text() to turn it into text.
classes.lsc:18:19  Dog has no field called nam. Did you mean name?
```

Mistakes that only show up while running point at the Lust line, not the Lua one:

```
missing-row.lsc:2  there's no row with id 7. The ids are 1, 2.
```

Every push builds a ready-made `lust.exe`: open the **Actions** tab, click the latest **Test** run, and download **lust-windows**.

## Writing Lua inside LustScript

Anything Lust doesn't cover yet can be written in Lua directly:

```
fetch lua;

method shout(string words) {
    lua {
        print(string.upper(words) .. "!")
    };
}

shout("hello");

end
```

## The name

Lust started as "Rust + Lua". Then it spent a year mostly being C#. Now it really is Rust and Lua.
