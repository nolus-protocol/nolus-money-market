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
