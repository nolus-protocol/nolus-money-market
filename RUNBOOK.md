# RUNBOOK

Discovery-tax patterns. Grouped by domain. Append on first re-encounter.

## Build

### `cargo build` fails in a fresh worktree: missing `build-configuration/protocol.json`

**Symptom:** A fresh `git worktree add` checkout fails at build time because the build script reads `build-configuration/protocol.json` and the file is absent.

**Cause:** `build-configuration/protocol.json` is git-ignored and is populated locally per checkout. New worktrees inherit the workspace but not that file.

**Fix:** Copy it from a working checkout:

```
cp <main-checkout>/build-configuration/protocol.json \
   <new-worktree>/build-configuration/protocol.json
```

Then re-run `SOFTWARE_RELEASE_ID=dev-release cargo build`.

Tried first (did not work): `cargo build` from the bare worktree; setting `SOFTWARE_RELEASE_ID` alone.

### Setting `SOFTWARE_RELEASE_ID` (and friends) without polluting the repo

**Symptom:** Tempting to add an `[env]` block to the repo-tracked `/.cargo/config.toml` so `cargo build` works without exporting `SOFTWARE_RELEASE_ID`. Diff-review rejects this — the value is developer-local, not a project default.

**Cause:** Cargo walks upward from the invocation directory and merges every `.cargo/config.toml` it finds. The repo-level one is tracked; anything checked in there becomes the default for everyone.

**Fix:** Put developer-local env in `protocol/.cargo/config.toml` (or any nested workspace `.cargo/config.toml`). `.gitignore:13` already excludes `**/.cargo` while keeping the tracked root file (`!/.cargo/`), so the override is automatically ignored. Cargo merges nested files over the root one when invoked from the protocol workspace.

Example `protocol/.cargo/config.toml`:

```
[env]
SOFTWARE_RELEASE_ID = "dev-release"
```

Tried first (rejected at review): editing the tracked `/.cargo/config.toml`.

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
