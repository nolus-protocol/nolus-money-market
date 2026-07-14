# CLAUDE.md

Guidance for Claude Code (claude.ai/code) in this repository.

## Project Overview

Nolus Money Market: a DeFi lending protocol implemented as CosmWasm smart contracts in Rust. Collateralized loans (leases), liquidity pools, oracle price feeds, multi-DEX swap support across Cosmos-ecosystem chains.

## Build Commands

Requires the `SOFTWARE_RELEASE_ID` env var (arbitrary string identifying the release).

```bash
# Build (from any workspace directory; platform/ needs only SOFTWARE_RELEASE_ID —
# protocol/ and tests/ additionally require PROTOCOL_NETWORK, PROTOCOL_NAME, PROTOCOL_RELEASE_ID)
SOFTWARE_RELEASE_ID='dev-release' cargo build

# Lint across all workspaces (uses cargo-each tool)
cargo lint

# Run all tests across all workspaces
cargo run-test

# Run a single test (within a workspace)
cargo test --all-features test_name

# Build optimized WASM (from the protocol workspace; full pipeline incl. RUSTFLAGS in ci/build.sh)
RUSTC_BOOTSTRAP=1 SOFTWARE_RELEASE_ID='dev-release' cargo each --tag build --tag @agnostic run --exact -- build -Zbuild-std="panic_abort,std" --profile "production_nets_release" --lib --locked --target wasm32-unknown-unknown

# CI-equivalent lint of one workspace (two args: workspace + lint subcommand; PROFILE required)
PROFILE=ci_dev ci/lint.sh protocol lint
```

IDE setup: set `SOFTWARE_RELEASE_ID=dev-release` in your editor's environment.

## Workspace Structure

Monorepo with three interconnected Cargo workspaces:

### `platform/` - Network-agnostic foundation

**Packages:**
- **`finance`** - Core financial types: coins, prices, fractions, percentages, ratios, interest calculations, liability modeling. Uses `bnum` for arbitrary-precision arithmetic. No cosmwasm-std dependency — wall-clock time is its own `Instant` type (serde binary-compatible with cosmwasm `Timestamp`).
- **`cw-time`** - Bridge crate converting between `finance::Instant` and cosmwasm `Timestamp` (`IntoInstant` / `IntoTimestamp` extension traits).
- **`currency`** - Type-safe currency system with compile-time mismatch prevention. Core traits: `Currency` (type marker), `CurrencyDef` (group associations). Visitor pattern for runtime currency dispatch.
- **`sdk`** - CosmWasm SDK wrapper with feature-gated re-exports (`contract`, `storage`, `testing`, `cosmos_ibc`, `cosmos_proto`). Abstracts external dependencies.
- **`platform`** - Contract orchestration: state machine, banking operations, response/reply handling, protobuf transactions (`trx`), Interchain-Account transaction submission (`ica`).
- **`versioning`** - Contract migration and version management. Supports semver with separate storage versioning.
- **`access-control`** - Permission management.
- **`lpp`** - Liquidity Provider Pool abstractions.
- **`oracle`** - Oracle protocol interfaces.
- **`time-oracle`** - Time-based alarm system.
- **`tree`** - Tree data structure utilities.

**Contracts:** `admin` (protocol management), `timealarms`, `treasury`

### `protocol/` - Network and DEX-specific implementations

**Packages:** `currencies` (protocol-specific definitions), `dex` (DEX abstraction layer), `marketprice`, `swap` (per-DEX request builders: `astroport`, `osmosis`), `remote_lease` (typed cross-chain lease operations: stub client, callback classification, envelope/response types), `remote_lease_wire` (standalone wire-types crate shared with the Solana-side Remote Lease App per ADR 0001/0002 — no internal deps, MSRV 1.89 enforced in CI, hardened deserialization)

**Contracts:** `lease` (loan positions), `leaser` (lease origination), `lpp` (lending pool), `oracle` (price feeds), `profit` (distribution), `remote_lease` (IBC controller paired with the Solana Remote Lease App per ADR 0001/0002), `reserve`, `void`

### `tests/` - Integration tests

Cross-workspace integration tests. The suite builds per DEX: `tests/Cargo.toml` declares a `{ tags = ["ci", "$dex"], include-rest = false }` combination with `$dex` generics over `dex-astroport_test`, `dex-astroport_main`, and `dex-osmosis`.

### `tools/` - Build tooling

- **`cargo-each`** - Custom cargo subcommand for running operations across workspace members with tag-based filtering. Powers `cargo lint` and `cargo run-test`.
- **`topology`** - Network topology validation.
- **`json-value`** - JSON manipulation utilities.

## Key Architectural Patterns

1. **Feature-gated compilation**: The `contract` feature enables actual contract implementations. Without it, only API types compile (useful for clients). Test utilities are gated behind `testing`.

2. **Type-safe currency system**: Currencies are compile-time types, not runtime strings. The `Currency` trait + `CurrencyDef` with group associations prevent financial operation mismatches at compile time.

3. **DEX and remote-swap abstraction**: The `dex` package drives asynchronous swaps over IBC Interchain Accounts (`dex::Account`); the per-DEX request builders live in the `swap` package (`astroport`, `osmosis`). The `remote_lease` contract is the IBC controller paired with the Solana-side Remote Lease App (ADR 0001/0002); its typed operations and wire types live in the `remote_lease` and `remote_lease_wire` packages. Lease **open** derives its `dex::Account` from the controller's `OpenLease` ack (`RemoteLeaseId` → `platform::ica::HostAccount`) rather than registering a Cosmos ICA; swap/repay/close transport still submits over ICA (`submit_tx`).

4. **`cargo-each` tag system**: Workspace members declare tags in `[package.metadata.cargo-each]` in their Cargo.toml. CI uses these tags to select which packages to build/test for each configuration. New workspace members must declare appropriate tags.

5. **Synchronized workspace lints**: The `[workspace.lints]` section is synchronized across all three workspace Cargo.toml files (marked with `### [SYNC WITH OTHER WORKSPACES]` comments).

6. **IBC-enabled contracts** (`remote_lease`): turning a contract into an IBC counterparty requires three coordinated changes:
   - Enable `cosmwasm-std/stargate`, `cosmwasm-std/cosmwasm_2_0`, and `sdk/cosmos_ibc` inside the contract's `contract` feature (the first emits `CosmosMsg::Ibc`, the second is required for `CosmosMsg::Any` in cosmwasm-std 3.x).
   - Gate the six IBC entry points (`ibc_channel_open`, `ibc_channel_connect`, `ibc_channel_close`, `ibc_packet_receive`, `ibc_packet_ack`, `ibc_packet_timeout`) under `#[cfg(feature = "contract")]` in a dedicated `ibc.rs` module — keep them out of the API-only build.
   - Update `ci/Containerfile`'s `cosmwasm_capabilities` allowlist if the contract enables a `cosmwasm_X_Y` feature not yet listed. The optimizer's `cosmwasm-check` runs against this allowlist independently of the chain runtime, so omissions surface as a CI-step failure on what looks like valid wasm.

## Linting Rules

All clippy lints denied. Key rules:
- `unwrap_used` and `unwrap_in_result` denied - use proper error handling
- `allow-unwrap-in-tests = true` (in `clippy.toml`)
- Future-incompatible and deprecated features forbidden
- All warnings are errors

## Code Style & Test Conventions

Imports below reference a private toolkit and won't resolve for external contributors; ignore them when building the repo.

@~/.claude/kit/snippets/rust-style.md
@~/.claude/kit/snippets/tests-style.md

### Project-specific overrides & exceptions

- **`expect()` posture**: clippy here denies `unwrap_used` / `unwrap_in_result` but not `expect_used`. `expect()` outside tests is still forbidden — reviewer-verified, not lint-enforced.
- **`const fn` / `Addr`**: `Addr` is not `Copy` (it wraps `String`), so methods returning `Addr` by value cannot be `const fn`.
- **Carve-out — `dex` composite state-machine vocabulary (issue #657)**, exception to *Avoid parasite words in names*: The `dex` crate's composite-workflow layer uses a deliberate, uniform vocabulary that is domain terminology here, not parasite words. Do not flag these names: the per-composite `pub enum State` and its disambiguating `State as StateXxx` re-exports (`out_local`→`StateLocalOut`, `out_remote`→`StateRemoteOut`), the `StartXxxState` start-state aliases (`StartLocalLocalState`, `StartLocalRemoteState`, `StartTransferInState`), and the lease-side `DexState` aliases that name those workflows. These are the established state-machine vocabulary of `protocol/packages/dex/src/impl_/` and its consumers; renaming one composite in isolation only fragments the convention. The fully-qualified `Handler::method(inner, …)` call style is likewise uniform across these composites and intentional — treat it as the established convention here, not a *Method syntax over fully-qualified trait calls* violation. New, unrelated abstractions remain subject to the rule.
- **Prefix unused-yet items with `_`; never `#[allow(dead_code)]`**: When adding an item that has no caller yet (because the calling code lands in a follow-up PR), prefix the item's name with `_` rather than attaching `#[allow(dead_code)]`. Example: `fn _build_lease_id(ordinal: u64) -> LeaseId { … }`. The leading underscore is local, mechanical, and disappears the moment a real caller arrives. `#[allow]` is invisible to grep, survives forever, and tends to be forgotten when the caller eventually lands.
- **Doc-comments on private items only when the *why* is non-obvious**: Public API items (`pub` / `pub(crate)` re-exports) use doc-comments to document their contract. Private items (private fns, private structs, inherent impls of private types) default to no doc-comment. A private item gets a doc-comment only when the comment encodes a non-obvious *why* — a hidden invariant, a workaround for a specific CosmWasm / IBC / DEX behaviour, a non-trivial choice between two reasonable implementations. The function's own name and signature carry the *what*. The word "helper" is banned **in identifiers and doc-comments** — it carries no information the function's own name and signature do not already convey. (Prose use of "helper" as a descriptive noun is fine.)

### Review checklist additions

- No `expect()` outside tests (see the `expect()` posture override above)
- Feature-flag correctness: items used only under `contract` or `testing` are properly gated
- New workspace members declare appropriate `[package.metadata.cargo-each]` tags
- `[workspace.lints]` blocks remain in sync across `platform/`, `protocol/`, `tests/`
- IBC-enabled contracts: every `cosmwasm_X_Y` / `stargate` / `neutron` feature enabled in the contract's Cargo.toml must appear in `ci/Containerfile`'s `cosmwasm_capabilities` allowlist
- IBC error-path strings sourced from a counterparty (handshake versions, packet `ack` payloads, etc.) must not be echoed back unbounded — bound the length or hash before storing/emitting

### Security review mapping

- CosmWasm contract code (anything under `contracts/` in any workspace) → `building-secure-contracts` (Cosmos scanner)
- Oracle / price-feed comparison, fraction / ratio math on untrusted input → `constant-time-analysis`
- Code handling private keys, mnemonics, or any sensitive material → `zeroize-audit`
- New/updated workspace dependencies → `supply-chain-risk-auditor`

### Quality gate for delegated work

Any agent performing a coding task must pass `cargo build`, `cargo fmt --all -- --check`, `cargo lint`, and `cargo run-test` (per-workspace — never from the repo root) before reporting the task complete.

## Build Profiles

- `ci_dev` - CI builds: no debug info, abort on panic
- `ci_dev_no_debug_assertions` - CI without debug assertions
- `test_nets_release` - Test network release (with debug assertions)
- `production_nets_release` - Production release (optimized for size, LTO, stripped)

## Environment Variables

- `SOFTWARE_RELEASE_ID` (required) - Release identifier string
- `PROTOCOL_NETWORK`, `PROTOCOL_NAME`, `PROTOCOL_RELEASE_ID` - Compile-time `env!` requirements of the protocol release in `versioning` (arbitrary strings; CI uses `ci-network` / `ci-protocol` / `ci-protocol-release`). Export them alongside `SOFTWARE_RELEASE_ID` before any protocol/tests workspace gate (`cargo lint`, `cargo lint-all`, `cargo run-test`); a missing one is a compile error.
- `NET` - Target network (e.g., `dev`, `main`)
- `PROTOCOL` - Protocol identifier (e.g., `osmosis-osmosis-usdc_axelar`, `neutron-astroport-usdc_noble`)
