# States first: the designs

> **Designed, not built.** These are what the project lead chose on 2026-11-27, after building the game Firefight in Lust and Solis. Each part is built in its own pull request, in the order below. Once a part is built, it moves into [LUST.md](LUST.md) and is removed from this file.

States were Lust's first idea: a thing is always in one of its options, and the language helps you say what each one means. These parts put states in charge of more, and tidy how values are passed and grouped.

## 1. Data on states

Each option carries its own values, so code reads them instead of a long `check` over every option.

```
state Pose = [
    Idle   { clip = Clip.Idle,  speed = 0.0 },
    Run    { clip = Clip.Run,   speed = 6.5 },
    Battle { clip = Clip.Fight, speed = 4.0 } [
        Melee  { clip = Clip.Swing },              -- speed 4.0 comes from Battle
        Ranged { clip = Clip.Aim, speed = 2.5 },
    ],
];

sprite:animate(pose.clip);
float s = Pose.Run.speed;
```

- **Written straight on each option,** in `{ }` after its name. Lust works out the shape: every option has the same names, and each name keeps one type. A whole number where another option has a `float` counts as a `float`.
- **Options inside options inherit:** an inner option takes its outer one's values, and can change any of them. Every option ends up with every value, so a missing one is a mistake that names the option and the value.
- **Fixed:** the values are part of the state, so `Pose.Run.speed = 7.0;` is a mistake. They can be read anywhere a state's value can (`pose.speed`, `Pose.Run.speed`) and in engines.
- **What they hold:** anything a field can hold that never changes: numbers, text, bools, other states, vectors, colours, and records (part 3) once they exist.

## 2. Named values

A call or a `send` can name each value, so long lists read clearly and can't be swapped by mistake.

```
send Wants(jump = true, crouch = false, fire = true) to body;
spawn(x = 3, y = 5);
```

- **For method calls and `send` alike** (and `new hook` calls too, if they have names).
- **All or none:** a call names every value or none of them, never some.
- **Any order:** named values can come in any order, and each name appears once.
- **The names are the method's or event's own:** a wrong name is a mistake that suggests the nearest one, and a missing one is a mistake that says which.
- Nothing changes on the receiving side: the method's or `on` block's names are already there.

## 3. Records

A plain group of values that can go anywhere, games included. An engine can carry it in a component's fields and in events, because it's made only of simple values.

```
record Loadout {
    FighterClass role;
    Gun primary;
    Gun secondary;
    int grenades = 2;
}

component Fighter {
    has {
        Loadout loadout;
    }
}

Loadout kit = new Loadout { role = FighterClass.Scout, primary = Gun.M4a1, secondary = Gun.Glock };
send MatchStarted(kit) to player;
```

- **Holds only simple values:** numbers, text, bools, states, vectors, rotations, colours, entities and other records. No methods, no classes and no arrays.
- **Copied when given:** setting a variable or field, passing it, or sending it copies the record. Changing one copy never changes another. `==` compares every value.
- **Starting values** work as in `has`. A value with no starting value must be given in `new`.
- **Solis builds its half:** storing records in component fields, copying them, and carrying them in events. Lust's half is the language, both outputs and the engine API (ENGINES.md says exactly what an engine must do). Until an engine supports records, using one in a component is a plain mistake naming the engine.

## 4. Events that count in a state

An entry in an `on` block can say which option of the thing's own state it's for.

```
on {
    Hit(entity by, float damage) in Battle {
        hp -= damage;
    }
    Hit(entity by, float damage) in Calm {
        state = Phase.Battle;
    }
}
```

- **`in Option`** after an entry's values. The same event can have several entries, each for different options.
- **Options inside options:** `in Battle` covers `Battle.Melee` and `Battle.Ranged`. The most precise entry wins.
- **An entry without `in`** covers every option no other entry covers.
- **An event that arrives in an option no entry covers** is ignored.
- Two entries for the same option are a mistake. So is `in` on a thing with no state of its own.

## 5. Methods that exist in some states

The same idea for methods: `method fire() in Battle { … }`.

- **A method can have several bodies,** each `in` different options, all with the same values and result. A body without `in` covers the rest.
- **Calling a method while no body covers the thing's current option** is an error with a new kind of problem, `Problem.WrongState`: "fire is only for Battle, but hero is Calm."
- Only for things with a state of their own.

## 6. Allowed moves between options

A state can say which option may follow which, so a move the game's rules don't allow is caught.

```
state Door = [Locked -> Shut, Shut -> [Open, Locked], Open -> Shut];
```

- **`Option -> Next`,** or `-> [A, B]` for several. As soon as any option has an arrow, only the moves written are allowed. An option with no arrow is an end: nothing follows it.
- With data (part 1): `Shut { sound = Sound.Click } -> Open`.
- **Options inside options:** arrows name options by their path from the state (`Battle.Melee -> Battle.Ranged`). An arrow from an outer option covers its inner ones.
- **Checked when the state changes:** a move that isn't allowed stops with `Problem.WrongState`: "a Door can't go from Locked to Open." A thing's starting state can be any option.
- Waits, events and `check` don't change.

## 7. Starting in a state

`new Hero in Phase.Battle` is the same as `new Hero { state = Phase.Battle }`, with the state up front. It goes with the other starting values: `new Hero in Phase.Battle { hp = 5 }`. Writing both `in` and `state =` is a mistake.

## 8. Ledger shows states live

Not a language change. While `lust run` runs a program from Ledger, a **States** panel lists each thing that has a state of its own (by its variable or field name where Lust knows it, and its class) and its current option. It updates as they change, with the last few changes listed and timed. `lust run` sends the changes on a side channel only when Ledger asks for them, so programs run at full speed otherwise. How exactly is settled while building.

## Build order

1. Data on states
2. Named values
3. Records (Lust's half, and a note for Solis)
4. Events that count in a state
5. Methods that exist in some states
6. Allowed moves between options
7. Starting in a state
8. Ledger's States panel
