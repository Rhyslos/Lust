# States first: the designs

> **Designed, not built.** These are what the project lead chose on 2026-11-27, after building the game Firefight in Lust and Solis. Each part is built in its own pull request, in the order below. Once a part is built, it moves into [LUST.md](LUST.md) and is removed from this file.

States were Lust's first idea: a thing is always in one of its options, and the language helps you say what each one means. These parts put states in charge of more, and tidy how values are passed and grouped.

## 1. Data on states

Built in alpha 94: see [LUST.md](LUST.md), section 8, Values on options.

## 2. Named values

Built in alpha 95: see [LUST.md](LUST.md), section 11, Named values.

## 3. Records

Built in alpha 96 (Lust's half): see [LUST.md](LUST.md), section 10, Records, and [ENGINES.md](ENGINES.md), alpha 96, for what Solis builds.

## 4. Events that count in a state

Built in alpha 97: see [LUST.md](LUST.md), section 11, Events in some options: in.

## 5. Methods that exist in some states

Built in alpha 98: see [LUST.md](LUST.md), section 11, Methods in some options: in.

## 6. Allowed moves between options

Built in alpha 99: see [LUST.md](LUST.md), section 8, Allowed moves between options.

## 7. Starting in a state (built, alpha 100)

Built in alpha 100: see [LUST.md](LUST.md), section 8, Starting in a state: new … in.

## 8. Ledger shows states live

Not a language change. While `lust run` runs a program from Ledger, a **States** panel lists each thing that has a state of its own (by its variable or field name where Lust knows it, and its class) and its current option. It updates as they change, with the last few changes listed and timed. `lust run` sends the changes on a side channel only when Ledger asks for them, so programs run at full speed otherwise. How exactly is settled while building.

## Build order

1. Data on states (built, alpha 94)
2. Named values (built, alpha 95)
3. Records (Lust's half built, alpha 96; Solis's half in ENGINES.md)
4. Events that count in a state (built, alpha 97)
5. Methods that exist in some states (built, alpha 98)
6. Allowed moves between options (built, alpha 99)
7. Starting in a state
8. Ledger's States panel
