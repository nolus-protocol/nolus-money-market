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

3. **Module-qualified function calls**: Always call functions with their defining module name (e.g., `merge::merge_asc()` not just `merge_asc()`). This makes dependencies explicit and easier to trace.

4. **Function size limits**: Max 20 non-doc lines and less than 3 indent levels. Break down large functions into smaller, focused helpers.

5. **Prefer plain functions over single-method structs**: If a struct exists only to call one method, use a plain function instead. Avoid unnecessary abstractions.

6. **Avoid parasite words in names**: Never use vague, overloaded words like "Context", "State", "Metadata", "Manager", "Handler", "Helper", "Utils" in names of new abstractions. Use descriptive domain-specific names.

7. **Debug assertions for invariants**: Use `debug_assert!` liberally to document and verify pre-conditions, post-conditions, and invariants throughout the code.

8. **Methods over free functions**: Logic that operates on a struct's data should be implemented as methods of that struct, not as free functions that take the struct as a parameter. Free functions are appropriate only for standalone operations that don't belong to any specific type.

9. **Prefer `T: Trait` over `impl Trait` in parameters**: When writing generic function parameters, prefer the explicit `T: Trait` form over `impl Trait`. This makes type parameters explicit and allows for better type inference and turbofish syntax when needed.

10. **Invariant checking via functors**: Define entity invariants as member boolean functions named `invariant_held` (e.g., `fn invariant_held(&self) -> bool`) and use them as post-conditions on entity instantiation and all update operations. This ensures invariants are documented and consistently enforced.

11. **Functional programming style**: Use iterator chains (`map`, `filter`, `fold`, `collect`) over imperative loops. Chain operations with `and_then` for Result types. Extract pure transformations into separate methods. Prefer expressions over statements where natural.

12. **Reuse over duplication**: Reuse existing methods rather than duplicating logic inline.

13. **Generic traits with item types**: When a trait's behavior depends on the item being compared, make the trait generic over `Item`.

14. **Consistent helper usage**: When you have helper methods (e.g., `take_left()`/`take_right()`), use them consistently in all code paths rather than mixing with inline implementations.

15. **Never use `as` for type conversions**: Use `from`/`into` instead. Exception: `const` context where `From` isn't const-stable yet -- isolate the `as` cast in a `const fn` helper, and only for widening conversions.

16. **Prefer chaining over `?` operator**: Use `and_then`, `map`, `ok_or`, etc. to chain `Result`/`Option` operations instead of early-returning with `?`. This keeps functions as single expressions with no hidden control flow. Use `?` only as a fallback when chaining becomes awkward (e.g., when intermediate values are needed in multiple later steps).

17. **Expected values first in assertion helpers**: When defining test helper functions that compare actual results against expected values, always place the expected-values parameter before other parameters.

18. **Consts at function start**: Define function-local `const` declarations at the very beginning of the function, before any `let` bindings or other statements.

19. **Prefer immutability over mutability**: Prefer consuming `self` and returning a new value over `&mut self` mutation. For example, prefer `fn advance(self, count: usize) -> Self` over `fn advance(&mut self, count: usize)`. This makes data flow explicit and avoids hidden side effects.

20. **Trait bounds in `where` clauses**: Place trait bounds in `where` clauses rather than inline on the generic parameter. Write `fn foo<T>(...) where T: Trait` instead of `fn foo<T: Trait>(...)`.

21. **Method syntax over fully-qualified trait calls**: Prefer `self.0.fmt(f)` over `Display::fmt(&self.0, f)` when there is no ambiguity.

22. **Avoid format-string overhead in Display impls**: Prefer calling `.fmt(f)` and `f.write_str()` in sequence (chained with `and_then`) over `write!` / `f.write_fmt(format_args!(...))`.
## Build Profiles

- `ci_dev` - CI builds: no debug info, abort on panic
- `ci_dev_no_debug_assertions` - CI without debug assertions
- `test_nets_release` - Test network release (with debug assertions)
- `production_nets_release` - Production release (optimized for size, LTO, stripped)

## Environment Variables

- `SOFTWARE_RELEASE_ID` (required) - Release identifier string
- `NET` - Target network (e.g., `dev`, `main`)
- `PROTOCOL` - Protocol identifier (e.g., `osmosis-osmosis-usdc_axelar`, `neutron-astroport-usdc_noble`)
