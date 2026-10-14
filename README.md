<img src="assets/lust.svg" width="96" alt="Lust logo">

# Lust

A small, strict, slightly silly programming language for things that have a state, carry stuff, and change over time. Think text adventures, RPG systems and little simulations.

```
state = [idle, fighting, knocked_out];

class Player {
    has {
        string[name "Hero"];
        int[hp 20];
        state is idle;
    }

    method hurt(int damage) {
        hp = hp - damage;

        is hp <= 0? then {
            state is knocked_out;
            print(name + " is knocked out!");
        } or {
            print(name + " has " + hp.text() + " hp left");
        }
    }
}

Player ada = new Player { name "Ada" };
ada:hurt(7);

end
```

## What makes it Lust

- **Checks are questions.** No `if`, no `==`: `is hp 0? then { } or { }`, plus the one-liner `either alive? "ready" or "napping"`.
- **`state` is built in.** Every file can have one, and things just *are* in a state: `state is fighting;`.
- **cArrays.** A connected array is a tiny table with ids: perfect for inventories and shops.
- **One loop.** `loop` does forever, repeat, while and for-each.
- **Strict and friendly.** Types first, `;` everywhere, every file ends with `end`, and mistakes are caught before the program runs, with errors that say what to do.
- **`true` and `not true`.** There is no `false`.

The full rules are in **[LUST.md](LUST.md)**. Examples are in [`examples/`](examples).

## Status

The language is designed; the compiler isn't written yet. The plan:

1. ~~The rulebook~~ ([LUST.md](LUST.md))
2. The compiler: `.lust` → C# → .NET, with friendly errors. First goal: `examples/player.lust` runs.
3. Running and errors inside [Ledger](https://github.com/Rhyslos/Ledger), which already highlights `.lust` files.
4. A small 2D game engine, written in Lust.

## The name

Lust started as "Rust + Lua", then turned out to be mostly C# with a Lua accent. The name stuck because it's funny. It may get a new one before anyone else sees it.
