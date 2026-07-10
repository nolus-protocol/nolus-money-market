# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Nolus Money Market is a DeFi lending protocol implemented as CosmWasm smart contracts in Rust. It provides collateralized loans (leases) with liquidity pools, oracle price feeds, and multi-DEX swap support across Cosmos-ecosystem chains.

## Build Commands

The build requires the `SOFTWARE_RELEASE_ID` environment variable (arbitrary string identifying the release).

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

IDE setup: Set `SOFTWARE_RELEASE_ID=dev-release` in your editor's environment.

## Workspace Structure

The project is a monorepo with three interconnected Cargo workspaces:

### `platform/` - Network-agnostic foundation

**Packages:**
- **`finance`** - Core financial types: coins, prices, fractions, percentages, ratios, interest calculations, liability modeling. Uses `bnum` for arbitrary-precision arithmetic. No cosmwasm-std dependency — wall-clock time is its own `Instant` type (serde binary-compatible with cosmwasm `Timestamp`).
- **`cw-time`** - Bridge crate converting between `finance::Instant` and cosmwasm `Timestamp` (`IntoInstant` / `IntoTimestamp` extension traits).
- **`currency`** - Type-safe currency system with compile-time mismatch prevention. Core traits: `Currency` (type marker), `CurrencyDef` (group associations). Visitor pattern for runtime currency dispatch.
- **`sdk`** - CosmWasm SDK wrapper with feature-gated re-exports (`contract`, `storage`, `testing`, `cosmos_ibc`, `cosmos_proto`). Abstracts external dependencies.
- **`platform`** - Contract orchestration: state machine, banking operations, response/reply handling. There is no `trx` module; the `ica` module carries only `HostAccount` and `ErrorResponse`.
- **`versioning`** - Contract migration and version management. Supports semver with separate storage versioning.
- **`access-control`** - Permission management.
- **`lpp`** - Liquidity Provider Pool abstractions.
- **`oracle`** - Oracle protocol interfaces.
- **`time-oracle`** - Time-based alarm system.
- **`tree`** - Tree data structure utilities.

**Contracts:** `admin` (protocol management), `timealarms`, `treasury`

### `protocol/` - Network and DEX-specific implementations

**Packages:** `currencies` (protocol-specific definitions), `dex` (DEX abstraction layer), `marketprice`, `remote_lease` (typed cross-chain lease operations: stub client, callback classification, envelope/response types), `remote_lease_wire` (standalone wire-types crate shared with the Solana-side Remote Lease App per ADR 0001/0002 — no internal deps, MSRV 1.89 enforced in CI, hardened deserialization), `remote_profit` (typed cross-chain profit operations — the profit-side twin of `remote_lease`), `remote_profit_wire` (standalone wire-types crate shared with the Solana-side Remote Profit App — no internal deps, MSRV 1.89, hardened deserialization)

**Contracts:** `lease` (loan positions), `leaser` (lease origination), `lpp` (lending pool), `oracle` (price feeds), `profit` (distribution), `remote_lease` (IBC controller paired with the Solana Remote Lease App per ADR 0001/0002), `remote_profit` (crate `remote_profit_controller`; IBC controller paired with the Solana Remote Profit App — singleton profit per ADR-0008), `drain_vault` (Instantiate2-addressed, profit-owned NLS sink with owner-gated `Sweep`), `reserve`, `void`

### `tests/` - Integration tests

Cross-workspace integration tests. The whole suite is DEX-agnostic and compiles+runs **once** — there is no per-DEX matrix. `tests/Cargo.toml` declares a single `{ tags = ["ci", "@agnostic"], include-rest = false }` combination; suite modules in `src/lib.rs` are plain `mod` declarations, not feature-gated.

### `tools/` - Build tooling

- **`cargo-each`** - Custom cargo subcommand for running operations across workspace members with tag-based filtering. Powers `cargo lint` and `cargo run-test`.
- **`topology`** - Network topology validation.
- **`json-value`** - JSON manipulation utilities.

## Key Architectural Patterns

1. **Feature-gated compilation**: The `contract` feature enables actual contract implementations. Without it, only API types compile (useful for clients). Test utilities are gated behind `testing`.

2. **Type-safe currency system**: Currencies are compile-time types, not runtime strings. The `Currency` trait + `CurrencyDef` with group associations prevent financial operation mismatches at compile time.

3. **Remote-swap abstraction**: The `dex` package drives asynchronous swaps that execute remotely on the Solana-side controller contracts (`remote_lease` / `remote_profit`). There are no local per-DEX request builders, no `swap` package, and no per-DEX build matrix — the whole workspace builds **once**, DEX-agnostic. No Interchain-Account path remains anywhere in the workspace.

   **Transport.** The lease opens with no ICA (`dex::Account::funding`, `host: None`): the downpayment + principal are ICS-20-transferred to the lease's Solana-side `LeaseAuthority` over the paired transfer channel — the Funding leg (`dex::StateFundRemote`), one coin in flight at a time, gated on the last funding ack before the opening swaps run — and the swaps execute on the remote-lease controller. Opening and repay share the Funding leg and the `remote_lease_host` bridge in `contract/state/mod.rs` (`RemoteLeaseId` → `HostAccount`); the in-progress opening query is `Funding { receiver }` (the `LeaseAuthority`). The lease is swap/DEX-client-agnostic but **not fully DEX-agnostic**: it still carries the DEX `ConnectionParams` at runtime to address the transfer channel it funds the lease over (`FundingClient::transfer_channel` → `dex().transfer_channel.local_endpoint`). `SudoMsg::OpenAck` returns a typed `UnsupportedOperation` error — the lease can never receive an `OpenAck`.

   The profit buy-back rides the same transport: the profit funds its Solana authority over ICS-20 (`Account::funding`, `StateFundRemote = Funding → RemoteSwap`), swaps to NLS on the `remote_profit` controller (singleton profit — the callback target is fixed at instantiation, not carried on the wire), then drains the bought-back NLS home (`StateDrain = RemoteTransferOut → FundsArrival`) into the profit-owned `drain_vault` contract before `Idle::send_nls`. The `drain_vault` isolates passively-received NLS so the drain's balance-baseline arrival gate stays sound. Profit's state machine is `OpenProfit → Idle → FundRemote → Drain`; outcomes arrive as `ExecuteMsg::RemoteProfitCallback`, authorized against `Config.remote_profit_controller`.

   **Failure handling.** An opening remote swap that errors with **no leg acknowledged** (`total_out == 0`, nothing swapped) clean-unwinds: the lease drains both inputs home from the `LeaseAuthority`, closes the LPP loan IN FULL (`principal + interest_due`, the opening-window interest covered from the RESERVE contract), refunds the WHOLE downpayment, and lands `OpenFailed` (`OpeningUnwind` state — `dex::StateDrain<OpeningUnwindTask>`; the in-progress query shows `opening::OngoingTrx::Unwinding`, serialized as the bare string `"unwinding"`). The reserve-covered full-close-and-refund batch lives in `opening::refund::refund_to_open_failed`, shared with the synchronous failed-open path. A `total_out > 0` (partial) swap error parks at `SlippageAnomaly`; a timeout re-emits unbounded.

   All three remote-lease drains — opening-unwind, the opened-lease proceeds drain, and the paid-lease asset transfer-out — gate completion on MEASURED arrival: `contract::state::arrival` (`snapshot_baseline` / `arrived_over_baseline`, generic over the currency `Group`) snapshots a per-currency local-account baseline at drain entry and requires `arrived − baseline ≥ Σ expected`, so a pre-existing balance or an external send cannot complete a drain (or trigger a refund) early. Worst case is recoverable stranding (operator `heal`).

   **Liquidation timeout auto-requote.** A liquidation swap leg (`AcceptUpToMaxSlippage`, the only `SlippageCalculator::REQUOTES_ON_TIMEOUT = true` class) re-quotes its `min_out` floor from the live oracle on each **in-budget** remote-swap timeout re-emission — bounded by `MaxSlippages.liquidation`, both directions, clamped `>= 1` — overwriting the pinned `in_flight_min_out` before re-emitting so the promise tracks the moving price; acknowledgment validation still uses the pinned value. An oracle-query failure falls back to the pinned floor and marks the retry event `requote = skipped`. The nonce bumps **once** per re-emission and the retry counters carry across, so the park-after-budget terminal stays intact and carries the last re-quoted floor. Past-budget escalation **never** re-quotes — the requote class hardcodes `SlippageEscalation::Park`. Opening / repay / customer-close / profit buy-back legs stay verbatim-pinned, and `Heal` on a live leg re-emits the pinned floor. Requote emissions carry `min-out-prev` / `min-out` event attributes. Seam: `SlippageCalculator::REQUOTES_ON_TIMEOUT` + `SwapTask::requote_on_timeout()`.

   **Storage.** The lease persists at storage v10 — the whole remote-lease state reshape (`LeaseDTO` carrying the non-optional `remote_lease_id` + `remote_lease_controller`, `dex::Account.host` as `Option<HostAccount>`, the funding-leg layouts, the `OpeningUnwind` in-flight variant, and the per-currency `baseline` on the three drains) lands as a single v9 → v10 step. Migration is storage-gated: pre-v10 layouts are refused with `ContractError::UnsupportedMigration`; same-storage v10+ in-family upgrades run the standard `update_software` like sibling contracts. Forward-only and **not rollback-safe once a lease persists mid-drain** (an older binary cannot load the layout). Remote-lease deploys only as a fresh protocol instance, so no existing v9 lease is migrated.

4. **`cargo-each` tag system**: Workspace members declare tags in `[package.metadata.cargo-each]` in their Cargo.toml. CI uses these tags to select which packages to build/test for each configuration. New workspace members must declare appropriate tags.

5. **Synchronized workspace lints**: The `[workspace.lints]` section is synchronized across all three workspace Cargo.toml files (marked with `### [SYNC WITH OTHER WORKSPACES]` comments).

6. **IBC-enabled contracts** (`remote_lease` and `remote_profit`): turning a contract into an IBC counterparty requires three coordinated changes:
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

Code style and unit-test conventions are imported below. The imports reference a private toolkit and will not resolve for external contributors; they can be ignored when building the repo.

@~/.claude/kit/snippets/rust-style.md
@~/.claude/kit/snippets/tests-style.md

### Project-specific overrides & exceptions

- **`expect()` posture**: clippy here denies `unwrap_used` / `unwrap_in_result` but not `expect_used`. `expect()` outside tests is still forbidden — reviewer-verified, not lint-enforced.
- **`const fn` / `Addr`**: `Addr` is not `Copy` (it wraps `String`), so methods returning `Addr` by value cannot be `const fn`.
- **Carve-out — `dex` composite state-machine vocabulary (issue #657)**, exception to *Avoid parasite words in names*: The `dex` crate's composite-workflow layer uses a deliberate, uniform vocabulary that is domain terminology here, not parasite words. Do not flag these names: the per-composite `pub enum State` and its disambiguating `State as StateXxx` re-exports (`out_local`→`StateLocalOut`, `drain`→`StateDrain`, `remote_swap_only`→`StateSwap`, `out_swap`→`StateOutSwap`), the `StartXxxState` start-state aliases, the per-composite `mod impl_handler` modules, and the lease-side `DexState` / `DrainState` / `StartState` aliases that name those workflows. These are the established state-machine vocabulary of `protocol/packages/dex/src/impl_/` and its consumers; renaming one composite in isolation only fragments the convention. The fully-qualified `Handler::method(inner, …)` call style is likewise uniform across these composites and intentional — treat it as the established convention here, not a *Method syntax over fully-qualified trait calls* violation. New, unrelated abstractions remain subject to the rule.
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
