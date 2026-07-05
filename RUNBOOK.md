# RUNBOOK

Discovery-tax patterns. Grouped by domain. Append on first re-encounter.

## Build

### `cargo build` fails in a fresh worktree: missing `build-configuration/` (`protocol.json` + `topology.json`)

**Symptom:** A fresh `git worktree add` checkout fails at build time because the currencies build script reads `build-configuration/protocol.json` and `build-configuration/topology.json` and the files are absent.

**Cause:** The whole `/build-configuration` directory is git-ignored (`.gitignore:2`) and populated locally per checkout. New worktrees inherit the workspace tree but not that directory's contents.

**Fix:** Copy its JSON definitions from a working checkout:

```
cp <main-checkout>/build-configuration/*.json \
   <new-worktree>/build-configuration/
```

Then re-run `SOFTWARE_RELEASE_ID=dev-release cargo build`.

Tried first (did not work): `cargo build` from the bare worktree; setting `SOFTWARE_RELEASE_ID` alone.

### Setting `SOFTWARE_RELEASE_ID` (and friends) without polluting the repo

**Symptom:** Tempting to add an `[env]` block to the repo-tracked `/.cargo/config.toml` so `cargo build` works without exporting `SOFTWARE_RELEASE_ID`. Diff-review rejects this — the value is developer-local, not a project default.

**Cause:** Cargo walks upward from the invocation directory and merges every `.cargo/config.toml` it finds. The repo-level one is tracked; anything checked in there becomes the default for everyone.

**Fix:** Put developer-local env in `protocol/.cargo/config.toml` (or any nested workspace `.cargo/config.toml`). `.gitignore:13` already excludes `**/.cargo` while keeping the tracked root file (`!/.cargo/`), so the override is automatically ignored. Cargo merges nested files over the root one when invoked from the protocol workspace.

Example `protocol/.cargo/config.toml` (the `PROTOCOL_*` trio is read by the
workspace-wide gates; the values mirror `ci/Containerfile`'s `ENV` block):

```
[env]
SOFTWARE_RELEASE_ID = "dev-release"
PROTOCOL_NETWORK = "ci-network"
PROTOCOL_NAME = "ci-protocol"
PROTOCOL_RELEASE_ID = "ci-protocol-release"
```

Tried first (rejected at review): editing the tracked `/.cargo/config.toml`.

### Local clippy (1.95) diverges from CI clippy (1.94)

**Symptom:** `cargo lint` / `cargo lint-all` fails locally on lints CI does not
run (e.g. `unnecessary_sort_by` in the untouched oracle contract), or passes
locally while CI fails — the verdicts differ on identical code.

**Cause:** There is no `rust-toolchain.toml`; CI pins its toolchain via the
`rust_image_digest` build argument in `ci/Containerfile` (currently 1.94),
while the local default toolchain follows `rustup` (currently 1.95). Clippy
adds and tightens lints between minor releases.

**Fix:** Run the gate on the CI-pinned toolchain:

```
rustup toolchain install 1.94.0
cargo +1.94.0 lint
cargo +1.94.0 lint-all
cargo +1.94.0 run-test
```

A local-only finding from a newer clippy is not a gate failure — fix it only
if it survives on 1.94. Bump the pinned version here in lockstep with
`ci/Containerfile`.

Tried first (did not work): treating local 1.95-only findings as CI blockers
and patching untouched code to silence them.

### `cargo lint` / `cargo run-test` from the repo root silently match 0 packages

**Symptom:** Running `cargo lint` or `cargo run-test` from the checkout **root** exits
`0` with no output — it reads like a clean pass. It isn't: nothing was linted or tested.

**Cause:** These are `cargo each` aliases that select packages by cargo-each **tag**, and
the tags live in the per-workspace members. The repo root has **no `Cargo.toml`** (the
workspaces are `protocol/`, `tests/`, `platform/`, plus tooling in `tools/`), so the tag
match resolves to the empty set and `each` exits `0`. Worse, a stray `Cargo.toml`
**above** the checkout hijacks cargo's upward manifest walk and runs against the wrong
tree entirely.

**Fix:** Always run the gates **inside** a workspace dir — `protocol/`, `tests/`,
`platform/`, or `tools/` — never from the repo root. Real CI iterates the workspaces via
`ci/for-each-workspace.sh`; mirror that. Two corollaries:

- `cargo lint` runs **without** `--all-targets`, so it does not compile `#[cfg(test)]`
  modules. When the change adds test code, lint it explicitly:
  `cargo clippy -p <crate> --features <...> --all-targets` (or `cargo lint-all` — see the
  Cargo section's "Pre-push lint" entry).
- **Exit `0` with zero output is not a PASS.** A real test run prints per-suite counts;
  require that evidence before calling it green.

Tried first (did not work): `cargo lint` / `cargo run-test` from the repo root and
reading the `0` exit as success.

## CosmWasm

### `CosmosMsg::Any` rejected by the WASM optimizer's `cosmwasm-check` step

**Symptom:** Contract code uses `CosmosMsg::Any { type_url, value }` (e.g., to emit `MsgChannelOpenInit` directly). Unit tests pass. CI `cosmwasm-check` fails with a capability error during the WASM optimizer step.

**Cause (two-part):**

1. In cosmwasm-std 3.x, `CosmosMsg::Any` is gated by the `cosmwasm_2_0` feature, not by `stargate` (which only gates the legacy `CosmosMsg::Stargate`). Enabling `cosmwasm-std/stargate` alone does not unlock `Any`.
2. Enabling `cosmwasm-std/cosmwasm_2_0` makes the compiled wasm advertise the `cosmwasm_2_0` capability. The chain runtime already declares the full `cosmwasm_1_1 .. cosmwasm_3_0` range, but the optimizer's `cosmwasm-check` runs against the static `cosmwasm_capabilities` ARG in `ci/Containerfile` — and that allowlist did not include `cosmwasm_2_0`+.

**Fix:**

- Enable both `cosmwasm-std/stargate` and `cosmwasm-std/cosmwasm_2_0` in the contract's `contract` feature.
- Add every newly-required capability to `ci/Containerfile`'s `cosmwasm_capabilities` ARG. As of PR #621 the allowlist is:
  `cosmwasm_1_1,cosmwasm_1_2,cosmwasm_1_3,cosmwasm_1_4,cosmwasm_2_0,cosmwasm_2_1,cosmwasm_2_2,cosmwasm_3_0,iterator,neutron,staking,stargate`.

**Alternative (not taken in #621):** Use the deprecated `CosmosMsg::Stargate { type_url, value }` — works under `stargate` alone, no `cosmwasm_2_0` needed, no Containerfile change. Rejected because `Stargate` is deprecated and `Any` is the forward path.

Tried first (did not work): enabling only `cosmwasm-std/stargate`; adding only `cosmwasm_2_0` to the contract Cargo.toml without expanding the Containerfile allowlist.

## Cargo

### A workspace dep's feature does not propagate from a consumer's same-named feature

**Symptom:** Contract crate `A` declares `feature = "stub"` and depends on workspace crate `B` which also has `feature = "stub"` (typically gating test or mock helpers). `cargo build --features stub` on `A` compiles, but the stub-gated items in `B` are unreachable — the items behind `#[cfg(feature = "stub")]` in `B` never compile in. Or: tests in `A` that rely on `B`'s stub helpers fail to find symbols.

**Cause:** Cargo features do not propagate by name. Declaring `[features] stub = []` in `A` does not turn on `stub` in `A`'s deps. The propagation must be spelled explicitly: `stub = ["B/stub"]`.

**Fix:** In `A`'s Cargo.toml:

```
[features]
stub = ["B/stub"]
```

If `A` also has a higher-level feature (e.g. `contract = [..., "stub"]`) that already lists `stub`, the propagation rides along automatically once the line above is in place — no extra work.

**Reviewer check:** any new `stub` (or test-helper / mock) feature on a contract crate that depends on a workspace crate exposing the same-named feature must propagate explicitly. Greppable: `git grep -n '^stub = \[\]$' protocol/contracts` should be empty.

Tried first (did not work): `stub = []` on the contract crate, expecting Cargo to forward by name.

### cargo-udeps flags a dev-dependency used only by feature-gated tests

**Symptom:** CI's "Check for unused dependencies" job fails for a package at
the feature-less combination: a dev-dependency that only tests inside a
feature-gated module use is reported unused — and the same-named *optional
regular* dependency is reported unused alongside it, even though it is off
without its `dep:` feature.

**Cause (two-part):** dev-dependencies cannot be feature-gated, so under a
cargo-each combination where the consuming test modules are `cfg`-ed out the
dev-dependency is genuinely unused there. Its presence in the resolved graph
then also makes `cargo-udeps` report the disabled optional regular dependency
as unused.

**Fix:** declare both as known false positives next to the cargo-each
metadata, with a comment:

```
[package.metadata.cargo-udeps.ignore]
development = ["<crate>"]
normal = ["<crate>"]
```

Reproduce/verify locally instead of pushing blind (versions from
`ci/Containerfile`):

```
cargo install cargo-udeps --version 0.1.59 --locked
rustup toolchain install nightly-2026-03-05 --profile minimal
cd <workspace>
cargo +nightly-2026-03-05 udeps --all-targets --locked -p <pkg>              # per
cargo +nightly-2026-03-05 udeps --all-targets --locked -p <pkg> --features … # combo
```

Tried first (did not work): ignoring only `development` — the induced
`normal` report persists; the plain stable toolchain — `cargo udeps` requires
nightly, and a too-old nightly fails the workspace's `rust-version`.

### Pre-push lint: `cargo lint-all`, not `cargo lint`

**Symptom:** `cargo lint` passes locally, CI's "Lint codebase with tests" job fails with errors in `#[cfg(test)]` code — e.g. an unresolved test-only path like `versioning::ReleaseId::new_test` under a feature combination that does not activate `versioning/testing`.

**Cause:** The repo has two distinct CI lint jobs, mirroring two distinct aliases in `/.cargo/config.toml`:

- `lint = "each --tag ci run --print-command -- clippy --locked"` — runs every cargo-each feature combination **without** `--all-targets`. Test binaries are not compiled. `cfg(test) = false`. Any compile error gated `#[cfg(test)]` is invisible.
- `lint-all = "each --tag ci run --print-command -- clippy --all-targets --locked"` — same matrix, **with** `--all-targets`. Test binaries compile under every combination. `cfg(test) = true`. This catches errors in `mod tests {}` blocks that only surface under feature combinations the test was not authored against.

CI runs both. Running only `cargo lint` locally and assuming "lint passes ⇒ ready to push" misses every cross-combination test-compile bug.

A second trap: `cargo test --features contract,testing` and `cargo clippy --features contract,testing --all-targets` both exercise test code, but only in the union combo. Test code that references symbols gated on `versioning/testing` (or any other dev-only feature) will compile under `contract,testing` but fail under `contract` alone — which CI's lint-with-tests matrix exercises.

**Fix / pre-push checklist:** before `git push` on any branch that touches `#[cfg(test)]` or `[dev-dependencies]`, run **both**:

```
cargo lint           # all combos, library only
cargo lint-all       # all combos, library + tests + bins + examples
cargo run-test       # tests across every CI feature combo
```

If `cargo lint-all` errors only under specific combinations, fix the root cause by activating the needed dev-only feature via `[dev-dependencies]` (Cargo unifies dev-dep features with regular-dep features whenever test/all-targets builds run) rather than wiring the test-only feature into the crate's own `[features]` table — the latter only helps when the consumer explicitly passes that feature flag, and CI's matrix doesn't.

Tried first (did not catch the bug): `cargo lint` + `cargo clippy --features contract,testing --all-targets` + `cargo test --features contract,testing`. None of these compile test code under the `--features contract` (alone) combination that CI exercises.
