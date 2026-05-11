# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Nolus Money Market is a DeFi lending protocol implemented as CosmWasm smart contracts in Rust. It provides collateralized loans (leases) with liquidity pools, oracle price feeds, and multi-DEX swap support across Cosmos-ecosystem chains.

## Build Commands

The build requires the `SOFTWARE_RELEASE_ID` environment variable (arbitrary string identifying the release).

```bash
# Build (from any workspace directory)
SOFTWARE_RELEASE_ID='dev-release' cargo build

# Lint across all workspaces (uses cargo-each tool)
cargo lint

# Run all tests across all workspaces
cargo run-test

# Run a single test (within a workspace)
cargo test --all-features test_name

# Build optimized WASM (from protocol workspace)
SOFTWARE_RELEASE_ID='dev-release' cargo each run -x -t build -t <protocol> -t <net> --exact -- cargo build --profile "production_nets_release" --lib --locked --target=wasm32-unknown-unknown

# Lint protocol/tests workspaces (needs features)
./lint.sh "net_${NET},${PROTOCOL}"
```

IDE setup: Set `SOFTWARE_RELEASE_ID=dev-release` in your editor's environment (see `.vscode/settings.json` or `.zed/settings.json` for examples).

## Workspace Structure

The project is a monorepo with three interconnected Cargo workspaces:

### `platform/` - Network-agnostic foundation

**Packages:**
- **`finance`** - Core financial types: coins, prices, fractions, percentages, ratios, interest calculations, liability modeling. Uses `bnum` for arbitrary-precision arithmetic.
- **`currency`** - Type-safe currency system with compile-time mismatch prevention. Core traits: `Currency` (type marker), `CurrencyDef` (group associations). Visitor pattern for runtime currency dispatch.
- **`sdk`** - CosmWasm SDK wrapper with feature-gated re-exports (`contract`, `storage`, `testing`, `cosmos_ibc`, `neutron`). Abstracts external dependencies.
- **`platform`** - Contract orchestration: state machine, transaction routing, ICA (Interchain Accounts), banking operations, response/reply handling.
- **`versioning`** - Contract migration and version management. Supports semver with separate storage versioning.
- **`access-control`** - Permission management.
- **`lpp`** - Liquidity Provider Pool abstractions.
- **`oracle`** - Oracle protocol interfaces.
- **`time-oracle`** - Time-based alarm system.
- **`tree`** - Tree data structure utilities.

**Contracts:** `admin` (protocol management), `timealarms`, `treasury`

### `protocol/` - Network and DEX-specific implementations

**Packages:** `currencies` (protocol-specific definitions), `dex` (DEX abstraction layer), `marketprice`, `swap`

**Contracts:** `lease` (loan positions), `leaser` (lease origination), `lpp` (lending pool), `oracle` (price feeds), `profit` (distribution), `reserve`, `void`

### `tests/` - Integration tests

Cross-workspace integration tests with DEX-specific feature combinations (`dex-astroport_test`, `dex-astroport_main`, `dex-osmosis`).

### `tools/` - Build tooling

- **`cargo-each`** - Custom cargo subcommand for running operations across workspace members with tag-based filtering. Powers `cargo lint` and `cargo run-test`.
- **`topology`** - Network topology validation.
- **`json-value`** - JSON manipulation utilities.

## Key Architectural Patterns

1. **Feature-gated compilation**: The `contract` feature enables actual contract implementations. Without it, only API types compile (useful for clients). Test utilities are gated behind `testing`.

2. **Type-safe currency system**: Currencies are compile-time types, not runtime strings. The `Currency` trait + `CurrencyDef` with group associations prevent financial operation mismatches at compile time.

3. **Multi-DEX abstraction**: The `dex` package abstracts over Astroport and Osmosis DEXes. Protocol contracts are built with specific DEX/network feature flags.

4. **`cargo-each` tag system**: Workspace members declare tags in `[package.metadata.cargo-each]` in their Cargo.toml. CI uses these tags to select which packages to build/test for each configuration.

5. **Synchronized workspace lints**: The `[workspace.lints]` section is synchronized across all three workspace Cargo.toml files (marked with `### [SYNC WITH OTHER WORKSPACES]` comments).

## Linting Rules

All clippy lints denied. Key rules:
- `unwrap_used` and `unwrap_in_result` denied - use proper error handling
- `allow-unwrap-in-tests = true` (in `clippy.toml`)
- Future-incompatible and deprecated features forbidden
- All warnings are errors

## Code Style

1. **Trait-based generics over enums**: When behavior varies by type, prefer generic type parameters with trait bounds and `PhantomData` over enum fields with match statements.

2. **Prefer `<` and `<=` over `>` and `>=`**: When writing comparisons, prefer `<` and `<=` operators over `>` and `>=` for consistency and readability. Use natural `PartialOrd` operators directly rather than converting to `std::cmp::Ordering` first.

3. **Module-qualified function calls; do not import functions**: Always call functions with their defining module name (e.g., `merge::merge_asc()` not just `merge_asc()`). Functions are not brought into scope via `use`; call them qualified by their right-most module. To make a module callable by its short name, import it with `use crate::foo::{self, ...}`. Exception: `pub use` re-exports that expose functions as part of a module's API surface.

4. **Function size limits**: Max 20 non-doc lines and less than 3 indent levels. Break down large functions into smaller, focused helpers.

5. **Prefer plain functions over single-method structs**: If a struct exists only to call one method, use a plain function instead. Avoid unnecessary abstractions.

6. **Avoid parasite words in names**: Never use vague, overloaded words like "Context", "State", "Metadata", "Manager", "Handler", "Helper", "Utils" in names of new abstractions. Use descriptive domain-specific names.

7. **`debug_assert!` liberally for in-body assertions**: Throughout function bodies, use `debug_assert!` to document and verify non-trivial expectations: loop invariants, intermediate state, post-conditions of private helpers, branch reachability ("cannot be empty here"). They serve as executable documentation and as a regression net under test runs. For function entry-point preconditions see rule 23; for type-level invariants see rule 10.

8. **Methods over free functions**: Logic that operates on a struct's data should be implemented as methods of that struct, not as free functions that take the struct as a parameter. Free functions are appropriate only for standalone operations that don't belong to any specific type.

9. **Prefer `T: Trait` over `impl Trait` in parameters**: When writing generic function parameters, prefer the explicit `T: Trait` form over `impl Trait`. This makes type parameters explicit and allows for better type inference and turbofish syntax when needed.

10. **Encode type invariants as `invariant_held` and check after every mutation**: Define entity invariants as a member function `fn invariant_held(&self) -> bool`. Invoke it via `debug_assert!(self.invariant_held())` immediately after every constructor and after every mutating operation. This makes the invariant executable, documents it next to the type, and ensures it is enforced consistently rather than re-derived ad hoc.

11. **Functional programming style**: Use iterator chains (`map`, `filter`, `fold`, `collect`) over imperative loops. Chain operations with `and_then` for Result types. Extract pure transformations into separate methods. Prefer expressions over statements where natural.

12. **Reuse over duplication**: Reuse existing methods rather than duplicating logic inline.

13. **Generic traits with item types**: When a trait's behavior depends on the item being compared, make the trait generic over `Item`.

14. **Consistent helper usage**: When you have helper methods (e.g., `take_left()`/`take_right()`), use them consistently in all code paths rather than mixing with inline implementations.

15. **Never use `as` for type conversions**: Use `from`/`into` instead. Exception: `const` context where `From` isn't const-stable yet -- isolate the `as` cast in a `const fn` helper, and only for widening conversions.

16. **Prefer chaining over `?` operator**: Use `and_then`, `map`, `ok_or`, etc. to chain `Result`/`Option` operations instead of early-returning with `?`. This keeps functions as single expressions with no hidden control flow. Use `?` only when a chain would require naming an intermediate binding that outlives a single expression — for example, when its value must be inspected or reused across separate statements that cannot be collapsed into one chain. Note: a `mut` binding captured by successive closures is not a blocker for chaining; Rust's NLL handles the non-overlapping borrows.

17. **Expected values first in assertion helpers**: When defining test helper functions that compare actual results against expected values, always place the expected-values parameter before other parameters.

18. **Consts at function start**: Define function-local `const` declarations at the very beginning of the function, before any `let` bindings or other statements.

19. **Prefer immutability over mutability**: Prefer consuming `self` and returning a new value over `&mut self` mutation. For example, prefer `fn advance(self, count: usize) -> Self` over `fn advance(&mut self, count: usize)`. This makes data flow explicit and avoids hidden side effects.

20. **Trait bounds in `where` clauses**: Place trait bounds in `where` clauses rather than inline on the generic parameter. Write `fn foo<T>(...) where T: Trait` instead of `fn foo<T: Trait>(...)`.

21. **Method syntax over fully-qualified trait calls**: Prefer `self.0.fmt(f)` over `Display::fmt(&self.0, f)` when there is no ambiguity.

22. **Avoid format-string overhead in Display impls**: Prefer calling `.fmt(f)` and `f.write_str()` in sequence (chained with `and_then`) over `write!` / `f.write_fmt(format_args!(...))`.

23. **Enforce function preconditions at the entry point**: Every function with non-trivial expectations on its parameters must document and enforce them as one-line guards before any other work begins. The enforcement form depends on the API surface:
    - **Internal APIs** (private items, `pub(crate)`, anything not reachable outside the crate): check with `debug_assert!` as the first statements in the body. The call graph is closed, violations are programmer errors, and checks vanish in release.
    - **Public APIs** (anything callable from outside the crate — contract entrypoints, exported helpers, API types consumed by clients): validate and return a typed `Err`. Callers can pass arbitrary input, so violations must be observable in release.

24. **Single point of return**: A function body has exactly one return path — the final tail expression. The main flow is one expression; no mid-function `return`, no branches whose arms each yield via `return`. The only permitted exception is short, specific guard checks at the very top of the function — most often early-success short-circuits (e.g., `if coins.is_empty() { return Ok(()); }`), occasionally input-validation rejections. Each guard must be a single line, must precede every other statement in the body, and must terminate the function rather than compute a partial result. Anything beyond a guard belongs in the main expression. Rule 16 covers the orthogonal `?`-vs-chaining axis.

25. **Module item ordering**: Order items within a module as follows:
    1. `use` statements in decreasing order of their scope
    2. `mod` declarations, if any, alphabetically
    3. Constants in decreasing order of their visibility
    4. New types in decreasing order of their visibility
    5. `impl` blocks with functions ordered in decreasing order of their visibility

26. **Import all types**: Always import types via `use` statements rather than referencing them inline with full paths (e.g., `use std::cell::RefMut;` then `RefMut<...>`, not `std::cell::RefMut<...>`).

27. **Minimum necessary visibility**: Default to private. Increase visibility only when a concrete consumer outside the current scope already exists — never speculatively.

28. **`pub` inside crate-private modules**: Non-private items inside a module that is itself not `pub` should be declared `pub`. The module's own visibility is the boundary; marking items `pub` inside a private module lets parent modules re-export them selectively via `pub use` without requiring redundant `pub(crate)` annotations on every item.

29. **Encapsulate struct data**: Keep struct fields private and expose behavior through methods, not raw data. Exception: pure data-transfer types — structs that hold no invariant and exist solely to carry data between layers (e.g., serde-deserialized message structs) — may expose their fields directly.

30. **No magic literals in production code**: Every literal value (string, integer, byte, etc.) used in production code must be named as a `const`. Unnamed literals are only acceptable inside `#[cfg(test)]` modules, where they serve as explicit test data whose meaning is self-evident from the assertion context.

31. **Name lifetimes after their binding — one name per shared constraint**: When a lifetime belongs to exactly one binding, name it after that binding (e.g., `'lease` for `lease: &'lease T`). When multiple bindings share the same lifetime constraint, give the shared lifetime a single descriptive conceptual name instead of splitting it. Never use opaque single-letter names like `'a`.

32. **`const` methods by default**: Declare every method `const fn` unless it calls a non-const function or performs heap allocation. Methods that move or copy fields into a struct, return a `Copy` type (e.g., `u8`, `u64`, `Addr` once const-stable), or return a shared reference (`&T`) to a field qualify unconditionally. Returning `&str` from a `String` field does not (`.as_str()` is not `const`-stable) — keep those non-const or change the return type to `&String` if `const` is needed.

33. **Struct fields follow the natural English description of the operation they represent, grouped semantics-first, infrastructure last.** Declare semantic fields in the order they appear in the sentence that describes the operation, then group infrastructure fields (storage keys, derived IDs, authority references) after them. Apply the same ordering to struct initialisers and destructuring patterns. Example — a lease repayment: `lease_id → payer → amount → currency` (semantic), then `storage handle → bumps/ids` (infrastructure). When a single field encapsulates multiple semantic concepts, it stands first as the primary payload.

34. **Closure parameter naming**: For a unit parameter write the tuple pattern `|()| ...`, not `|_| ...`. For an unused non-unit parameter give it a descriptive name prefixed with `_` (e.g. `|_events| ...`), not bare `_`. Bare `_` discards type information and makes the closure harder to read.

35. **Avoid internal clones**: Design APIs so callers decide whether to clone. Accept borrowed references (`&T` / `&mut T`), owned values (`T`), or types implementing `Into` rather than cloning data inside the function. Cloning at the call site is the caller's explicit choice; cloning inside the function is a hidden cost. A `&T` parameter signals "I only read this"; `T` signals "I consume this". Match the signature to the actual use.
    - Prefer borrowing when you only need read access: `fn foo(arg: &T)`.
    - Prefer `&mut T` when you need to mutate caller's data.
    - Accept owned `T` when the function logically consumes the value: `fn take(self, v: T)`.
    - Use `Into` for ergonomic conversions without forcing clones: `fn new<S: Into<String>>(s: S) { let s = s.into(); }`.
    - Return owned values when creating data; return references only when tied to input lifetimes.
    - For `Copy` or small types, cloning is acceptable; avoid cloning large heap data (`String`, `Vec`, `Arc` unless intended).
    - If you must store a copy internally, take ownership (`T`) or require the caller to pass `Arc`/`Rc` so cloning is explicit.

36. **Doc-comments on private items only when the *why* is non-obvious**: Public API items (`pub` / `pub(crate)` re-exports) use doc-comments to document their contract. Private items (private fns, private structs, inherent impls of private types) default to no doc-comment. A private item gets a doc-comment only when the comment encodes a non-obvious *why* — a hidden invariant, a workaround for a specific CosmWasm / IBC / DEX behaviour, a non-trivial choice between two reasonable implementations. The function's own name and signature carry the *what*. The word "helper" is banned — it carries no information the function name does not already convey.

37. **Prefix unused-yet items with `_`; never `#[allow(dead_code)]`**: When adding an item that has no caller yet (because the calling code is landing in a follow-up PR), prefix the item's name with `_` rather than attaching `#[allow(dead_code)]`. Example: `fn _build_lease_id(ordinal: u64) -> LeaseId { … }`. The leading underscore is local, mechanical, and disappears the moment a real caller arrives. `#[allow]` is invisible to grep, survives forever, and tends to be forgotten when the caller eventually lands.

38. **No global mutable state**: All mutable state must be either (a) configuration — set at construction and mutated only via explicit owner/admin operations (`owner`, allowlists, parameters); or (b) per-entity — keyed by the entity's identifier in a structured collection (`Map<lease_id, Lease>`, `Map<currency, PoolState>`). Singleton flags whose only purpose is to gate a subsequent operation are forbidden; this includes `Item<bool>` armed/disarmed booleans, module-level `static mut`, `lazy_static! { Mutex<...> }`, and any cross-call coordination via thread-locals or singletons. If a gate is needed, derive it from the parameters of the gated call or from per-entity state — never from a sticky flag. Pure-immutable singletons (constants, configuration loaded once and never mutated) are allowed; the rule is about *mutable* global state.

## Unit Test Guidelines

1. **Test module ordering**: Follow the same ordering as production code — type aliases and constants first, then `#[test]` functions, then helper functions and assertion utilities.

2. **Test every public method**: Each public method should have at least one test exercising it directly, not only indirectly through other methods.

3. **Group related tests**: Place new tests next to existing tests that exercise the same API. Merge boundary-case tests into the existing test function when they add a single assertion; create a separate test function when the scenario needs distinct setup.

4. **Exercise all branches per method**: Each match arm, if/else branch, and early-return path in a public method should be hit by at least one test. Use `cargo llvm-cov` to verify.

5. **Test through the public API only**: Do not test private methods directly. Exercise them through the public methods that call them.

6. **Respect `debug_assert` preconditions**: When writing tests for code with `debug_assert!` invariants, ensure test inputs satisfy those assertions. Study the production call sites to understand valid input ranges.

## Project Overrides to Global Rules

These extend or replace the corresponding entries in the global `~/.claude/CLAUDE.md`.

- **Security-skill mapping for this project:**
  - CosmWasm contract code (anything under `contracts/` in any workspace) → `building-secure-contracts` (Cosmos scanner)
  - Oracle / price-feed comparison, fraction / ratio math on untrusted input → `constant-time-analysis`
  - Code handling private keys, mnemonics, or any sensitive material → `zeroize-audit`
  - New/updated workspace dependencies → `supply-chain-risk-auditor`
- **Diff Reviewer additions** (on top of the global checklist):
  - No `unwrap()` / `expect()` outside tests — clippy enforces, but reviewers verify the test/non-test split is correct
  - No `as` casts except in `const fn` widening helpers (Code Style rule 15)
  - Feature-flag correctness: items used only under `contract` or `testing` are properly gated
  - New workspace members declare appropriate `[package.metadata.cargo-each]` tags
  - Errors are typed in library code (no `anyhow` strings reaching public APIs)
  - Invariant methods (`invariant_held`) called after constructors and mutations (Code Style rule 10)
  - `[workspace.lints]` blocks remain in sync across `platform/`, `protocol/`, `tests/` (Architectural Pattern 5)
- **Coding-agent gate:** every coding agent must pass `cargo build`, `cargo lint`, and `cargo run-test` before handoff.
- **Discovery-tax location:** runbook entries for this repo live in `RUNBOOK.md` at the repo root (not in personal memory), grouped by domain (CosmWasm, IBC, DEX, build).

## Build Profiles

- `ci_dev` - CI builds: no debug info, abort on panic
- `ci_dev_no_debug_assertions` - CI without debug assertions
- `test_nets_release` - Test network release (with debug assertions)
- `production_nets_release` - Production release (optimized for size, LTO, stripped)

## Environment Variables

- `SOFTWARE_RELEASE_ID` (required) - Release identifier string
- `NET` - Target network (e.g., `dev`, `main`)
- `PROTOCOL` - Protocol identifier (e.g., `osmosis-osmosis-usdc_axelar`, `neutron-astroport-usdc_noble`)
