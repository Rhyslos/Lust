# Lust v2: Core Strengths & Optimal Development Patterns

Lust v2 occupies a unique niche in the programming language ecosystem. By pairing C#'s structural clarity with Lua's expressive minimalism, checks as explicit questions, and state-bound memory ownership, it eliminates entire classes of common bugs without sacrificing performance or readability.

---

## 1. Primary Strengths of Lust v2

### A. Predictable State-Bound Memory Ownership
Instead of relying on a background garbage collector (which introduces runtime stalls) or manual allocation (which introduces memory leaks and dangling pointers), Lust ties memory lifetimes directly to `state` and `node` boundaries.
* **Zero GC Overhead:** Transitioning out of a state automatically reclaims allocated memory arenas instantly.
* **Deterministic Lifecycles:** When a state exits or a `node` closes, background processing and entities bound to that context immediately halt and clean up.

### B. High Readability & Unambiguous Syntax
* **Explicit Question Constructs:** `is hp <= 0? then { ... }` eliminates ambiguity and syntax mistakes common in C-style `if` statements.
* **Clean Negation:** Replacing `false` with `not true` enforces logical consistency across checks.
* **Strict File Boundaries:** Requiring every file to end in `end` and every statement in `;` ensures parser predictability and easy tool/compiler building.

### C. Multi-Target Versatility
Lust's three-tier target model allows writing core logic once while deploying everywhere:
1. **Native Machine Code (.lust):** LLVM-backed speed for desktop/system applications.
2. **WebAssembly (.lust):** Near-native performance inside web browsers.
3. **LustScript (.lsc):** Line-by-line transpilation into clean Lua for embeddable engine scripting and game modding.

### D. Relational Data Without Database Overhead
The built-in `cArray(...)` type offers lightweight, auto-indexed tabular storage natively in code, making inventory management, entity rosters, and status lists straightforward without needing an external ORM or SQL engine.

---

## 2. Optimal Architecture & Best Practices

To get the most out of Lust v2, structure applications around these core paradigms:

### 1. Structure Around States First
Do not manage object lifecycles manually. Design your application using state hierarchies.

```lust
state AppState = [
    Booting,
    ActiveSession [
        InMenu,
        InGame,
    ],
];
```
* Put temporary data inside state boundaries so it cleans up automatically when switching scenes or screens.
* Keep global state strictly to long-lived systems (e.g., user profiles, settings).

### 2. Leverage `node` for Local Subsystems
Use `node` blocks for short-lived operational scopes (such as UI modals, level sectors, or temporary AI routines):

```lust
node DialogueModal {
    state is Active;
    string speaker = "NPC";
    // Once DialogueModal closes, 'speaker' and related memory are reclaimed.
}
```

### 3. Use `assembly` Blueprints for Entity Spawning
Instead of complex object constructors and factory patterns, define clean `assembly` blocks:

```lust
assembly EnemySpawner {
    create Goblin {
        string type = "Goblin";
        int hp = 20;
    }
}

Enemy g = EnemySpawner:create(Goblin);
```

### 4. Separate Core Logic from Modding APIs
* Put core runtime logic in `.lust` files targeting Native/Wasm performance.
* Put external user interfaces, triggers, and mod hooks into `.lsc` (LustScript) files so modders can run Lua scripts directly against your exposed hooks using `fetch lua;`.

---

## 3. Ideal Use Cases

| Domain | Why Lust v2 Fits Best |
| --- | --- |
| **Game Engines & Games** | Arena memory via `state` eliminates frame drops from Garbage Collection. First-class Lua transpilation (`.lsc`) makes modding seamless. |
| **Cross-Platform Desktop UI** | Declarative `fetch ui;` templates and C# interop offer high productivity with compact binary sizes. |
| **Embedded & WebAssembly Apps** | Minimal footprint, strict type checking, and compilation down to native or Wasm targets. |
| **Data-Heavy Tools & Simulations** | Native `cArray` tabular structures allow rapid lookup and record manipulation. |
