# Remote Lease Controller — `protocol_admin` Operator Runbook

Operator-facing procedures for the `remote_lease` controller: channel lifecycle,
lease-code rotation, recovery, and deployment. Audience: a `protocol_admin` /
`lease_admin` operator with **no prior ADR context**.

Source of truth is the contract code on `main`; the ibc-solray ADRs are linked for
design rationale only — when a message or config shape differs, the code wins.
Cross-references:

- Wire types and per-operation packet surface: [`docs/remote-lease-wire-contract.md`](../../docs/remote-lease-wire-contract.md).
- In-lease callback consequences and lifecycle traces: [`remote-lease-callback-flow.md`](./remote-lease-callback-flow.md).
- Design rationale: `nolus-protocol/ibc-solray` ADR 0001 (Remote Lease Protocol) and ADR 0002 (Remote Lease App). Section numbers are cited inline.

> **Production values are not in this repository by design.** Every concrete
> `connection_id`, `transfer_channel`, `dex_label`, `lease_code` id, controller
> address, `protocol_admin`, and `lease_admin` address is deployment configuration
> the operator records out-of-band. This runbook uses `<placeholders>` and describes
> message *shapes*; substitute the live values from your deployment records.

---

## 0. Preconditions & identities (read first)

### 0.1 Who may call what

- **`protocol_admin`** — set **once** at instantiate (`InstantiateMsg.protocol_admin`)
  and stored in a single-user access slot. **No** message rotates it and **no** query
  exposes it; record the address from your instantiate records. All three admin
  operations (`OpenChannel`, `CloseChannel`, `NewLeaseCode`) require it; any other
  caller fails with `Unauthorized`.
- **`lease_admin`** — a *different* identity, living in the **leaser**'s
  `Config.lease_admin`, not in the controller. It is the only authority that may
  `Heal` a lease parked in the slippage-anomaly terminal (§4.4). Identify it by
  querying the relevant leaser's config out-of-band; it is not derivable from the
  controller or lease code.

### 0.2 Which contract receives each message

| Operation | Send to |
|---|---|
| `OpenChannel`, `CloseChannel`, `NewLeaseCode` | the **controller** contract |
| `Heal` | the **lease** contract (per-lease recovery, §4) |
| `RemoteLeaseCallback` | controller → lease, internal — **operators never send this** |

### 0.3 Config immutability map

`connection_id`, `dex_label`, and `transfer_channel` are **immutable** after
instantiate — no message mutates them. The only mutable field is `lease_code`
(via `NewLeaseCode`, §3). The leaser's `remote_lease_controller` binding is
likewise immutable. **Changing any immutable field requires a fresh
instantiate / redeploy**, not an admin call.

---

## 1. Open a channel — `ExecuteMsg::OpenChannel()`

`{"open_channel":[]}` *(unit-tuple variant → array form)*

### 1.1 Auth & preconditions

- `protocol_admin` only.
- Allowed **only when no channel is recorded**. The guard reads the controller's own
  persisted channel record (written on `OpenAck`), so it is safe to re-issue while a
  handshake is still in flight; a recorded channel returns `ChannelAlreadyExists`.

### 1.2 What it does

Emits exactly one fire-and-forget `MsgChannelOpenInit` (no reply/sub-message). The
handshake version is composed as:

```
nls-remote-lease.v1+transfer=<transfer_channel>
```

i.e. the bare protocol version plus the `+transfer=channel-<N>` suffix naming the
paired Solana ICS-20 transfer channel (ADR 0002 §3.3). The `+transfer=` suffix exists
at the **handshake layer only**; per-packet envelopes carry the bare version.

### 1.3 Procedure

1. `QueryMsg::Config()` → confirm `connection_id`, `dex_label`, and especially
   `transfer_channel` are the intended values (they drive the handshake version and
   are immutable).
2. Confirm the paired transfer channel is fully open and fee-free (§5.4–5.5).
3. Send `OpenChannel()` as `protocol_admin`.
4. Poll `QueryMsg::Channel()` until it returns a channel (see §1.5).

### 1.4 What the contract validates during the handshake

This controller **only ever initiates** — an inbound `OpenTry` is rejected
(`UnsupportedCounterpartyOpen`). On its own `OpenInit` (validated in the same tx) and
again on `OpenAck`, it checks, in order: unordered channel, the **exact** version
string (including the full `+transfer=` suffix — a bare/no-suffix version is
rejected), the `connection_id`, and the counterparty port `nls-remote-lease.<dex_label>`.
On `OpenAck` it additionally requires the counterparty's echoed version to match
verbatim, then persists the channel as `Open`.

### 1.5 Verifying success

`QueryMsg::Channel()` → `{"channel":[]}` returns `{"channel": null}` until the
handshake completes, then `ChannelInfo { local_channel_id, counterparty_channel_id,
counterparty_port_id, version, state: "open" }`.

### 1.6 Failure & recovery

- **Solana rejects / `OpenTry` never relayed** → no channel is persisted; `Channel()`
  keeps returning null. **Recovery: re-issue `OpenChannel()`** — idempotent, because the
  "already exists" guard reads persisted storage (still empty). See ADR 0001 §3.4, §6.
- **Local mismatch at the self-call** (wrong version/port/connection) → the whole
  `OpenChannel` tx reverts with a typed error. Because config is immutable, fixing it
  requires a fresh **re-instantiate**, then `OpenChannel`.
- **Zombie `INIT` channel** — every `OpenChannel` creates a fresh ibc-go `INIT`-state
  channel bound to `wasm.<controller>`; a failed handshake leaves it dangling. It is
  **harmless** (retry idempotency keys on the controller's own persisted record, not the
  chain INIT) and the contract has no handler to clear it. Clearing the stale INIT
  channel is a relayer/IBC-layer action (e.g. `MsgChannelCloseInit` against the INIT id,
  found via `nolusd query ibc channel channels` filtered by port `wasm.<controller>`),
  outside the contract surface. ADR 0001 §6/§7.2 mark this operator-level/out-of-scope.

---

## 2. Close a channel — `ExecuteMsg::CloseChannel()`

`{"close_channel":[]}`

### 2.1 Auth & preconditions

`protocol_admin` only; requires a recorded channel currently in `Open` state.

### 2.2 What it does

Moves the recorded channel `Open → Closing` (a one-way local soft-lock) and emits one
`IbcMsg::CloseChannel`. Once `Closing`, the controller rejects **every** new outbound
operation (`OpenLease`/`CloseLease`/`Swap`/`TransferOut`) with `ChannelNotOperational`.
The record is removed only when the counterparty's `CloseConfirm` arrives. There are
only two stored states — `Open` and `Closing`; "closed" means the record is gone.

### 2.3 Drain first — the invisible-lease window

**A zero registered-lease count is not proof the channel is idle.** A customer
paid-close finalizes (deregisters) the lease *before* the `CloseLease` acknowledgment
returns; the lease then sits in `ClosingRemoteLease` while reporting `Closed()`, and is
invisible to registered-lease counts (see `remote-lease-callback-flow.md`). Once the
channel is `Closing`, such an in-flight lease can no longer emit its cleanup leg.

**Before `CloseChannel`:** drain the `ClosingRemoteLease` population — watch the
lease-side `wasm-ls-close-remote-lease` events out-of-band until none remain in flight.

### 2.4 Procedure

1. Stop opening new leases on this protocol.
2. Drain all in-flight operations and the `ClosingRemoteLease` population (§2.3).
3. Send `CloseChannel()` as `protocol_admin`.
4. Verify (§2.5).

### 2.5 Verifying

`Channel()` reports `state: "closing"` immediately, then `{"channel": null}` once
`CloseConfirm` clears the record.

### 2.6 Failure & recovery

- **No channel** → `ChannelNotOpen`. **Already `Closing`** → `ChannelNotOperational`
  (this is idempotency protection — do **not** retry; wait for `CloseConfirm`).
- **Stuck/zombie `Closing`** (counterparty never confirms) — the channel stays `Closing`
  indefinitely; there is **no controller-side timeout or force-clear**. Recovery is
  purely at the relayer/IBC layer. While `Closing`, no outbound packets are possible and
  `OpenChannel` is blocked (record still present).
- **Unsolicited counterparty `CloseInit`** on a healthy `Open` channel →
  `UnsolicitedChannelClose`. A relayer cannot force-close an `Open` channel; the operator
  must `CloseChannel()` first. (ADR 0001 §6.)

---

## 3. Rotate the lease code — `ExecuteMsg::NewLeaseCode { lease_code }`

`{"new_lease_code":{"lease_code":<u64>}}`

> **Shape note:** here `lease_code` is a **bare integer** (`Code` is transparent over a
> `u64`). This differs from `InstantiateMsg.lease_code`, which is a `Uint64` →
> **quoted string** (`"9"`). Do not copy the quoting between the two.

### 3.1 What it does

Updates **only** `Config.lease_code` and emits **zero** messages. No channel impact.
`protocol_admin` only.

### 3.2 The silent-success trap

`Code` deserializes with **no on-chain existence check** — a typo'd or non-existent code
id **succeeds silently**. The only later symptom is that **every** legitimate lease call
fails with `UnauthorisedCaller` (the controller authorizes packet senders against
`Config.lease_code`). **Mandatory:** immediately after `NewLeaseCode`, run
`QueryMsg::Config()` and confirm `lease_code_id` equals the redeployed Lease code id.

### 3.3 Coordination with a Lease redeploy

The leaser's `MigrateLeases` rotates the **leaser's** own lease code and batch-migrates
Lease instances, then pushes the new code to LPP and Reserve only — it does **not**
notify the remote_lease controller. You must therefore issue the controller's
`NewLeaseCode` as a **separate** `protocol_admin` tx, matching the redeployed Lease code
id. This two-tx coordination is enforced nowhere; do it deliberately.

**Two different authorities sign these.** `MigrateLeases` is gated by the leaser's
**`ContractOwner`** (its wasm-admin / governance), **not** `protocol_admin`; the
controller's `NewLeaseCode` is gated by `protocol_admin`. Line them up before you start —
using the `protocol_admin` key for the `MigrateLeases` step will be rejected.

### 3.4 Drain old-code leases first

The Lease contract's `migrate` returns `UnsupportedMigration` **unconditionally**
(layouts are binary-incompatible; no escape hatch). After rotation, old-code leases can
no longer emit packets (`UnauthorisedCaller`) and cannot be migrated — though they can
still *finish* via callbacks (acks/timeouts do not re-check the code). **Drain every
old-code lease to a terminal state before rotating.**

### 3.5 Procedure

1. Drain all old-code leases to terminal (§3.4).
2. Redeploy the Lease wasm; note the new code id.
3. leaser `MigrateLeases { new_code_id, … }` — signed by the leaser **`ContractOwner`**
   (governance / wasm-admin), **not** `protocol_admin`. This rotates the leaser's own
   `lease_code` and its LPP/Reserve references; it does **not** migrate remote leases (the
   Lease `migrate` is unconditionally refused and §3.4 already drained them), so for this
   protocol the step is leaser-side bookkeeping only.
4. controller `NewLeaseCode { lease_code: <new id> }` — signed by `protocol_admin`.
5. `QueryMsg::Config()` → confirm `lease_code_id == <new id>` (§3.2).

### 3.6 Failure & recovery

- **Wrong id** → re-issue `NewLeaseCode` with the correct id (no other state touched).
- **Stranded old-code lease** → none after the fact; it must reach terminal via the
  existing callback flows. It cannot be upgraded.
- `NewLeaseCode` on an uninstantiated/corrupted config → `Std` error (config stays unset).

---

## 4. Recovery & `Heal`

### 4.1 Mental model

The controller is a **stateless one-shot dispatcher**: on `ack`/`timeout` it decodes the
committed outbound envelope and dispatches exactly **one** `RemoteLeaseCallback` to the
lease — no loop, no retry counter, no re-emit. **All recovery logic lives in the
lease's dex state machine.**

### 4.2 Who retries what

- **Relayer** re-delivers a packet whose `ack` the lease rejected (the lease returns an
  error *only* on synchronous infra faults — auth/serialize/storage — which reverts the
  ack so the relayer retries). Every content/protocol fault is **absorbed** as `Ok` + an
  event so the ack commits and the relayer loop unblocks.
- **Lease** auto-recovers: timeouts re-emit up to a per-operation budget then park at the
  slippage-anomaly terminal; under-floor errors escalate per policy; underpaid acks
  re-emit with a bumped nonce.
- **Controller** never retries on the lease's behalf.
- Relayer retry **cadence/max-retries** is hermes-lite relayer configuration, not a
  contract property — consult the relayer for the actual numbers.

`OperationErr` vs `OperationTimeout` for the operator: **Err** = Solana/DEX/vault
rejection, funds **not** moved; **Timeout** = packet never acked, funds **may still be in
flight** until the channel-level timeout — treat as potentially-pending, not failed.

### 4.3 `Heal` — sent to the **lease**

`{"heal":[]}` *(unit-tuple variant on the lease; no fields, no funds)*

States that expose `Heal`: a live `RemoteSwap` leg (re-emits the in-flight leg with a
**pinned** floor and bumped nonce), the parked slippage-anomaly terminal (re-quotes a
**fresh** oracle floor and resets counters), `ClosingRemoteLease` (re-emits
`CloseLease`), `Closed` (drain), and an opened/active lease (re-run a stuck final repay).
All other states reject `Heal`.

### 4.4 State-dependent authorization (critical)

The **same** `{"heal":[]}` message is **permissionless** on a live `RemoteSwap` leg but
requires **`lease_admin`** on a parked slippage-anomaly terminal. The message itself
gives no hint which applies — **query the lease state first**:

- `StateResponse == SlippageProtectionActivated` → **`lease_admin` required** (an
  unauthorized `Heal` is rejected before any re-quote; the leg stays parked).
- otherwise → **permissionless**.

### 4.5 Idempotency — why `Heal` is safe to repeat

Each swap leg carries a strictly-monotonic nonce, bumped on every emission/re-emission/
`Heal`. On callback, a stale (smaller-nonce) packet's late ack is absorbed as
`nonce-mismatch` and never double-credited. So `Heal` is **safe to repeat regardless of
timing** — you need not wait for the original timeout.

**Caveat:** only `Swap` carries a real nonce; `OpenLease`/`CloseLease`/`TransferOut` use
nonce `0` and rely on the IBC at-most-once single-packet property instead of nonce
matching.

### 4.6 Recovery playbook by symptom

| Symptom | Action |
|---|---|
| Stuck `Swap` leg, no callback | `Heal()` on the lease (permissionless); safe to repeat |
| `StateResponse = SlippageProtectionActivated` (parked) | `Heal()` from **`lease_admin`** — re-quotes a fresh floor + resets counters |
| Stuck `ClosingRemoteLease` (best-effort Solana close failed) | permissionless `Heal()` re-emits `CloseLease`; customer payout already done, funds not stranded |
| `OperationTimeout`, funds maybe in flight | wait for the channel-level timeout before treating as failed |
| Persistent error-revert loop | investigate the lease's pinned `remote_lease_controller` / storage — **not** auto-recovery |

### 4.7 Observability

There is no contract query for the in-flight nonce. Diagnose via emitted events:
`heal`/re-emit, `anomaly/under-min-out`, `anomaly/slippage-anomaly-parked`,
`anomaly/price-alarm-dropped`, `timeout/retry`, and `absorbed/<reason>` (reasons include
`nonce-mismatch`, `parked-response`/`-error`/`-timeout`, `undecodable-response`,
`out-currency-mismatch`) — plus `StateResponse`.

---

## 5. Deployment checklist & invariants

### 5.1 Standalone deployment

`remote_lease` is deployed **outside** the standard protocol bundle — it is **not** in
`scripts/deploy-protocol.sh` and **not** in the admin contract's managed protocol set
(`leaser`/`lpp`/`oracle`/`profit`/`reserve`). It is a standalone wasm contract whose wasm
admin is the configured `protocol_admin`.

### 5.2 Two-step deploy (same authority)

1. **Instantiate** the controller.
2. `protocol_admin` sends `OpenChannel()` afterward — instantiate emits **zero**
   sub-messages, so the channel never auto-opens.

### 5.3 `InstantiateMsg` & validation order

```json
{
  "protocol_admin": "<bech32 addr>",
  "connection_id": "<connection-N>",
  "dex_label": "<dex>",
  "transfer_channel": "<channel-N>",
  "lease_code": "<u64 as string>"
}
```

Validated fail-fast in this order: non-empty `connection_id`, non-empty `dex_label`,
**canonical** `transfer_channel`, `addr_validate(protocol_admin)` (the admin contract is
**not** existence-checked — it may not be instantiated yet), grant the admin slot, then
`lease_code` existence check, then store.

**Canonical `transfer_channel`:** exactly `channel-` + a decimal `u16` with **no leading
zeros**. Accepted: `channel-0`, `channel-42`. Rejected: `""`, `42`, `channel-`,
`channel-abc`, `channel-007`, `channel-+5`, `channel-70000` (> u16), `transfer/channel-4`.

### 5.4 Invariant A — pair a fully-open transfer channel

The Solana responder validates the named transfer channel for existence, **`Open`**
state (its own handshake complete, not merely `TryOpen`), transfer-ness (ics20-1,
counterparty port `transfer`), and a **shared connection** (same first hop / light
client). The Nolus controller does **not** check any of this locally — a misconfig
surfaces only as a cross-chain `OpenTry` rejection. **Finish the ICS-20 transfer-channel
handshake first, then `OpenChannel`.** (ADR 0002 §3.3.)

### 5.5 Invariant B — the transfer channel must be fee-free

The Solana side asserts **exact-debit** (received amount equals sent amount) and rejects
fee-skimming transports (e.g. a Token-2022 transfer-fee mint). A fee-bearing channel
under-delivers and the lease strands with **no in-band Nolus signal** (it fails closed).
Prevention is the only recovery — **pair only a plain fee-free ics20-1 channel.**

> This "fee-free" rule is a **derived** invariant, synthesized from exact-debit
> (ADR 0002 §3.8) and the fact that ICS-29 relayer-fee middleware is dead upstream — it
> is not a single named rule in the code. Treat it as a hard operator precondition.

### 5.6 Out-of-band prerequisites (not contract-enforced)

Two trust-anchor decisions are **not** enforced on-chain and must be verified before
relying on the protocol (ADR 0001 §5.1): the IBC-Solray **program id** chosen at light-client
setup, and the `nls-remote-lease.<dex>` → Lease App **program-id mapping** at instance
setup. Record and audit both off-chain.

### 5.7 Migrate safety

`migrate` re-checks the config invariant and **refuses** on drift — a pre-`transfer_channel`
stored config → `IncompatibleStoredConfig`, a non-canonical stored channel →
`MalformedStoredConfig`, a release mismatch → `UpdateSoftware`. A refused migrate leaves
the instance **intact** (no brick).

### 5.8 Post-deploy verification

1. `QueryMsg::Config()` → confirm `connection_id` / `dex_label` / `transfer_channel` /
   `lease_code_id`.
2. `QueryMsg::Channel()` → `null` until `OpenAck`, then `state: "open"`.
3. Build/capability note: emitting `MsgChannelOpenInit` requires the `CosmosMsg::Any` +
   `cosmwasm_2_0` capability allowlist at build time — see the repo `RUNBOOK.md`. Nolus
   `pirin-1` accepts `CosmosMsg::Any(MsgChannelOpenInit)` (no message filter); a testnet
   smoke test before mainnet is recommended (ADR 0001 §9.1).

---

## 6. Quick reference

### 6.1 Message JSON

| Send to | Message | JSON |
|---|---|---|
| controller | `OpenChannel` | `{"open_channel":[]}` |
| controller | `CloseChannel` | `{"close_channel":[]}` |
| controller | `NewLeaseCode` | `{"new_lease_code":{"lease_code":9}}` *(bare int)* |
| controller | `Config` (query) | `{"config":[]}` |
| controller | `Channel` (query) | `{"channel":[]}` |
| controller | `ProtocolPackageRelease` (query) | `{"protocol_package_release":{}}` |
| **lease** | `Heal` | `{"heal":[]}` |

### 6.2 Error → cause → recovery

| Error | Cause | Recovery |
|---|---|---|
| `Unauthorized` | caller is not `protocol_admin` | send from the admin key |
| `ChannelAlreadyExists` | channel already recorded | none needed; this is the idempotency guard |
| `ChannelNotOpen` | `CloseChannel` with no recorded channel | nothing to close |
| `ChannelNotOperational` | op while `Closing` | wait for `CloseConfirm`; do not retry |
| `UnsolicitedChannelClose` | counterparty `CloseInit` on a healthy channel | `CloseChannel()` first if intended |
| `UnauthorisedCaller` (on lease ops) | `lease_code` points at the wrong id | re-issue `NewLeaseCode` with the correct id |
| `IncompatibleStoredConfig`/`MalformedStoredConfig`, `UpdateSoftware` | migrate drift | fix the migration target; instance is intact |

### 6.3 Cross-references

- ADR 0001 §3.2 (per-message guard matrix), §3.4 (lifecycle), §3.7/§3.7.1 (callback
  design), §5/§5.1 (trust model & out-of-band prerequisites), §6 (failure modes),
  §9.1 (`MsgChannelOpenInit` acceptance / smoke test).
- ADR 0002 §3.3 (paired transfer-channel validation), §3.8 (exact-debit / fee-free).
- [`remote-lease-callback-flow.md`](./remote-lease-callback-flow.md) — in-lease callback
  consequences, the invisible-lease close window, the drain-before-rotate procedure.
- [`docs/remote-lease-wire-contract.md`](../../docs/remote-lease-wire-contract.md) —
  pinned constants, the per-operation packet surface, envelope/callback shapes.
