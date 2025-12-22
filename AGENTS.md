# AGENTS.md

This repository is optimized for Codex-style agent work. Follow the rules below for *all* changes unless a specific module explicitly documents an exception.

## Hard rules

- **Avoid closures**
  - No lambdas capturing variables.
  - No nested functions closing over outer scope.
  - No callbacks that capture mutable state.
  - Prefer **named types (classes)** with explicit dependencies passed via constructors.
  - Pure functions are allowed only if they do **not** capture outer scope and remain trivial.

- Prefer **Object-Oriented Programming as described by Yegor Bugayenko**
  - Systems are composed of collaborating objects that encapsulate behavior.
  - Avoid data structures plus procedural logic.

---

## Object-oriented design principles (Yegor Bugayenko style)

### 1) No getters / setters
- Do not expose internal state via getters.
- Do not mutate state via setters.
- Ask objects to **do work**, not to reveal data (“tell, don’t ask”).

### 2) Prefer immutability
- Objects must be fully initialized at construction time.
- After construction, objects should not change internal state.
- Changing real-world data should be represented by stable objects whose *results* may vary, not by mutating fields.

### 3) No `null` (or equivalents)
- Do not accept or return `null`, `nil`, `None`, etc.
- Use explicit objects instead.

### 4) No static methods or utility classes
- Avoid static helpers and global utility classes.
- Behavior belongs in objects, not in procedural helpers.

### 5) No type inspection, reflection, or casting
- Do not branch on runtime types.
- Avoid reflection for core domain behavior.
- Use polymorphism and composition instead.

### 6) Minimize procedural orchestration
- Avoid methods that coordinate data via loops and `if/else`.
- Push behavior and decisions into objects.

---

### 7) Commands and queries are strictly separated

As specified by **Yegor Bugayenko**, a method must be **either** a command **or** a query — **never both**.

#### Query
- Returns a value.
- Has **no observable side effects**.
- Does **not** change object state.
- Returns **objects**, not primitives.
- Method names describe **what the object provides**, not *how it is computed*.
- Examples:
  - `message() -> Message`
  - `client() -> Client`
  - `total() -> Money`

#### Command
- Performs an action.
- Returns **nothing** (or only a language-mandated void/unit).
- Does **not** return data or status flags.
- Method names are **clear domain verbs**, not procedural flow words.
- Examples:
  - `connect()`
  - `open()`
  - `receive()`

#### Forbidden
- Methods that return a value **and** mutate state.
- Methods that return booleans or status codes.
- Predicate-style methods such as:
  - `hasX()`
  - `isY()`
  - `canZ()`
- Methods that expose internal state for external branching.

---

### 8) Method naming rules (message-oriented)

- Methods are **messages sent to objects**, not questions asked about them.
- Avoid interrogative or predicate naming.
- Avoid orchestration nouns such as:
  - `*Launcher`
  - `*Runner`
  - `*Executor`
  - `*Manager`
- If such a class exists (infrastructure boundary only):
  - Prefer **query methods** that return objects:
    - `client()`
    - `message()`
  - Or **command methods** with clear domain verbs:
    - `connect()`
    - `receive()`
  - Never combine construction, side effects, and return values.

---

### 9) Avoid global constants as coupling points
- Do not centralize logic in public constants or configuration bags.
- Prefer objects that encapsulate configuration and behavior together.

### 10) Prefer composition over inheritance
- Favor small composable objects.
- Inheritance is acceptable only when substitutability is preserved and no shared mutable state is introduced.

---

## Practical guidance for PRs

- Introduce small objects with a single responsibility.
- Replace closure-based callbacks with:
  - dedicated objects implementing an interface/protocol, or
  - objects passed in and messaged explicitly (`handler.handle(x)`).
- Keep dependencies explicit via constructors.

## When these rules clash with existing code

- New code must follow these rules even if legacy code does not.
- When touching legacy code, improve adherence opportunistically:
  - remove setters and move behavior into the object,
  - replace booleans with polymorphic objects,
  - eliminate static helpers by introducing objects.
