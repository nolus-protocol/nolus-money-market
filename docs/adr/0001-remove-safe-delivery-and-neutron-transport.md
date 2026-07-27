# ADR 0001 — Remove the DEX safe-delivery mechanism and the Neutron sudo transport

| | |
|---|---|
| **Status** | **Proposed (gated)** — blocked on an external `ibc-solray` capability decision; do not implement until the two pivotal questions in §8 are answered. |
| **Date** | 2026-07-27 |
| **Deciders** | Nolus protocol team + the `ibc-solray` (Solana Remote Lease App) owner |
| **Supersedes / Relates** | Rewrites the safe-delivery model documented in `protocol/docs/remote-lease-callback-flow.md`. **Note:** the `ibc-solray` repo (`nolus-protocol/ibc-solray`) maintains its *own* ADR series — its ADRs 0001/0002 (the passive-vault contract) are a **distinct external numbering** that this repo merely references; this document is `nolus-money-market` ADR 0001 and does not supersede them. |

---

## 1. Context

### 1.1 What the safe-delivery mechanism is

Every DEX response delivered to a `lease` instance is processed through a four-box "safe-delivery" segment — `ResponseDelivery` + a `reply_on_error` self-submessage + a `TimeAlarms` retry loop. It is documented box-by-box in `protocol/docs/remote-lease-callback-flow.md` (boxes A–D, lines 44–110) and implemented in `protocol/packages/dex/src/impl_/resp_delivery/mod.rs`.

- **Box A — persist + schedule (outer tx).** The composite state's `on_response` does the minimum: it wraps the handler + the raw response into `ResponseDelivery` and calls `enter()` (`resp_delivery/mod.rs:106`), which emits a single self-call `SubMsg::reply_on_error(WasmMsg::Execute(self, DexCallback))` via `schedule_execute_wasm_reply_on_error_no_funds(myself, ForwardToInnerMsg::msg(), REPLY_ID)` (`resp_delivery/mod.rs:111`), then returns `Ok`. No business logic runs here. The entry point is `impl_::forward_to_inner` (`protocol/packages/dex/src/impl_/mod.rs:56`), reached from the composite `State::on_response` arms (`out_local.rs:313/319/325`, `out_remote.rs:212/219`). Persisted state becomes `SwapExactInRespDelivery` / `TransferOutRespDelivery` / `TransferInInitRespDelivery`.
- **Box B — inner attempt (same tx).** `execute(DexCallback)` — gated `SameContractOnly` — loads the persisted `ResponseDelivery`, and `on_inner` (`resp_delivery/mod.rs:167`) → `do_deliver` (`:129`, calls `Delivery::deliver`) runs the *real* decode/transition. On `Ok` the wrapper is gone in one atomic step.
- **Box C — capture failure, schedule retry.** If B `Err`s, the outer submessage's `reply_on_error` fires `contract::reply(REPLY_ID = 12345678901, err)`; `ResponseDelivery::reply` (`resp_delivery/mod.rs:173`) → `setup_next_delivery(now + 1ns)` (`:137`) schedules a `TimeAlarms` alarm. The response stays buffered; the outer tx still commits.
- **Box D — retry loop.** `TimeAlarms` fires; `on_time_alarm` (`resp_delivery/mod.rs:180`) → `do_redeliver` (`:133`, calls `Delivery::deliver_again`) re-runs delivery. **This is a distinct entry point from box B** — box B is `on_inner`→`do_deliver` (same-tx attempt); box D is `on_time_alarm`→`do_redeliver` (alarm-driven re-attempt). The loop repeats every `+1ns` until success; `ExecuteMsg::Heal()` is the operator escape hatch.

The three load-bearing properties (flow doc lines 107–123): the outer `Ok` is unconditional (only storage/serde errors escape box A), no half-applied transitions are ever visible, and **recovery is host-driven, not relayer-driven** — once the outer ack commits, the relayer is done with the packet.

### 1.2 Why it exists — the Neutron `SudoMsg` callback constraints

Safe-delivery was built for Neutron's ICA `SudoMsg` transport. A Neutron sudo callback (`SudoMsg::Response`, `platform/packages/sdk/src/ica.rs:62-83`) runs under a **fixed `contractmanager` gas cap**, and on failure the packet is **parked in Neutron's failure queue** requiring a manual `ResubmitFailure` — it is **not** relayer-retried. Safe-delivery makes the gas-capped callback do almost nothing (box A) and defers the real, gas-hungry work to a locally-driven retry loop that is independent of both the gas cap and relayer liveness.

### 1.3 What changed — swap and transfer-in now ride the remote-lease controller

The remote-lease work moved swap and transfer-in off Neutron sudo onto the `remote_lease` IBC controller:

- The controller's `ibc_packet_ack` (`protocol/contracts/remote_lease/src/ibc.rs:130`) decodes the ack and dispatches `WasmMsg::Execute(lease, ExecuteMsg::RemoteLeaseCallback(cb))` via a **plain `add_message`** (`ibc.rs:194`), not `reply_*`. A lease `Err` therefore **reverts the ack tx and the relayer resubmits** — native "error ⇒ try again," the exact semantic safe-delivery had to synthesize by hand.
- Transfer-in is already sudo-free by a different route: Nolus confirms arrival by polling its **own** bank balance (`transfer_in::check_received`, `protocol/packages/dex/src/impl_/transfer_in.rs:12`; `POLLING_INTERVAL = 5s` at `transfer_in.rs:10`; poll call site `transfer_in_finish.rs:234`), driven by a time alarm — no callback transport at all.

### 1.4 The finding — safe-delivery is now redundant for controller callbacks but load-bearing for exactly one leg

The mechanism's core is **unchanged** by the remote-lease work; the outer transport changed under it, not the boxes. It is now:

- **Redundant** for the controller callbacks (swap, transfer-in-return): the controller gives native relayer-retry, so box A's "defer the real work" adds nothing.
- **Still load-bearing** for exactly one path — the **outbound transfer-out** (Nolus→DEX): funding the vault at open, and forwarding the repay payment. That leg is still emitted as a Neutron custom message `InterChainMsg::IbcTransfer` (`platform/packages/platform/src/bank_ibc/local.rs:85`, scheduled no-reply at `:108`) and its **only** Nolus-side completion signal is `SudoMsg::Response` (`ica.rs`) → `process_sudo` → `on_dex_response` (`protocol/contracts/lease/src/contract/endpoins.rs:186`) → the dex `TransferOut` state.

`platform::remote::submit_transaction` (the ICA-submit path) has **zero references** anywhere in production — it is already fully removed, so there is no dead ICA code to strip and the outbound transfer-out is the **sole remaining live `SudoMsg` consumer** (`endpoins.rs:107`, the lease `sudo` entry point). All other `fn sudo` handlers consume unrelated per-contract **governance** enums (each imports its own `msg::SudoMsg`, **not** `sdk::api::SudoMsg`): `leaser` (`contracts/leaser/src/contract.rs:179`), `lpp` (`contracts/lpp/src/contract/mod.rs:213`), `oracle` (`contracts/oracle/src/contract/mod.rs:161`), plus `platform`'s `treasury`, `admin`, `timealarms` — all must survive.

**Verified mechanism-integrity conclusions (C1–C4):**

- **C1 — CONFIRMED (high).** In-flight safe delivery is intact: for a fresh `OperationOk` while the sub-state is `SwapExactIn`/`TransferInInit`, both composite arms route through `forward_to_inner`, which only wraps into `ResponseDelivery` and calls `enter()` — no decode/transition/bank logic. No leak of the "box A can propagate business-logic `Err` to the controller" kind is constructible; the only escapes are the storage/serde-class errors the box-A invariant explicitly permits.
- **C2 — PARTIAL (high).** The lease-**open** ack already *bypasses* safe-delivery: `OpenLease::on_remote_lease_callback` (`open_lease.rs:240`) handles `OperationOk(OpenLease)` synchronously via `on_open_lease_ack` (`:114`), building the dex `Account` and calling `buy_asset::start(...).enter()` directly in the `RemoteLeaseCallback` execute — no `ResponseDelivery` wrap, no self-scheduled `DexCallback`. This is a **benign** restructuring, not a transient-failure regression. Two steps on the success path are fallible by signature: the empty/malformed-PDA guard `RemoteAccount::try_from` (`open_lease.rs:120`, `platform/packages/platform/src/remote.rs`) and `next.enter(...)` (`open_lease.rs:137`, `.map_err(ContractError::DexError)`). The latter is `TransferOut::enter` = `Ok(generate_requests(...))`, i.e. **infallible in practice** for the transfer-out request build; so the PDA guard is the only *reachable* failure, and it is *deterministic*, not transient. The only genuine gap: that guard has **no timeout fallback** and would strand the lease if it ever tripped — the recommended hardening is to route it into `OpenFailed` (like the existing unexpected-`OperationOk` arm at `open_lease.rs:252`), not to reintroduce `ResponseDelivery`.
- **C3 — PARTIAL (high).** There is **no in-flight idempotent absorber**: `TransferInFinish`'s `HandlerT` impl (`transfer_in_finish.rs:263`) defines no `on_response`/`on_error`/`on_timeout` override, and the three `*RespDelivery` states likewise inherit the default `Err("handle transaction response")` (`protocol/packages/dex/src/response.rs:53`). A callback landing there returns `Err` → reverts `ibc_packet_ack` → infinite relayer retry. **Severity LOW / informational**: this is unreachable under honest ibc-go and unreachable by a hostile counterparty in isolation; even if a broken IBC stack triggered it, blast radius is bounded — the ack reverts against unchanged state, the lease's funds/state are untouched.
- **C4 — CONFIRMED (high).** The `OperationOk` JSON round-trip is real: `state/dex.rs:74-78` serializes the already-typed `OperationResponse` with `to_json_binary`, `forward_to_inner` persists the `Binary` inside `ResponseDelivery` (`resp_delivery/mod.rs:46`), and box B `from_json`s it back (`decode_resp.rs:150`). Harmless but a needless serialize / storage-rewrap / deserialize cycle per swap ack; the only state that actually materializes the `Binary` is `SwapExactIn`.

**Mechanism facts (M1–M5).** The removal/migration groundwork rests on five verified mechanism facts, complementary to the safe-delivery-integrity facts C1–C4 above. They are the labels the §9 evidence table tags against:

- **M1 — sudo census & the single live consumer.** The lease is the sole live `SudoMsg` consumer; `submit_transaction` has zero refs repo-wide; the outbound emitter is Neutron `InterChainMsg::IbcTransfer` (scheduled no-reply); the unbounded counterparty `details` echo lives inside `process_sudo`.
- **M2 — controller transport semantics.** The controller dispatches acks via plain `add_message` (native `Err`→relayer-retry), rejects all inbound packets today (`UnsupportedInboundPacket` — the Option-A hinge), and the return leg is already sudo-free via self-balance polling under the double-credit invariant.
- **M3 — outbound funding shape.** Open funds **two** coins and `TransferOut` waits for all acks; repay funds one coin; the swap request names exact inputs + a slippage floor; a non-swapped output-currency coin is excluded from the request and re-added on Nolus.
- **M4 — absorber & Heal inventory.** `SwapExactIn::on_error` already returns `Ok`; absorber gaps exist (`TransferInInit` no `on_error`; `TransferInFinish` no late absorber); Heal gaps exist (`SwapExactIn`, `TransferOut`); the reference absorber is `OpenLease`→`OpenFailed`.
- **M5 — storage & CI.** Persisted composite `State` is externally-tagged JSON (name-safe variant drop); storage version is 10 and `migrate` refuses unconditionally; the CI `cosmwasm_capabilities` allowlist is untouched by this work.

---

## 2. Decision

**Remove the DEX safe-delivery mechanism and the Neutron sudo transport**, moving every DEX operation to the single controller model — synchronous processing inside the `RemoteLeaseCallback` execute, `Err → relayer-retry` for transient failures, and **explicit terminal-absorber classification** for permanent failures — and **eliminate the `OperationOk` JSON round-trip** (`Handler::on_response` stops taking `Binary` and takes the typed `OperationResponse`).

This decision is **gated**. Full removal is *not* tractable inside `nolus-money-market` alone, because the outbound transfer-out has no sudo-free completion signal: Nolus cannot observe the Solana vault balance (the inbound self-balance-poll trick is not reusable outbound), and a plain `IbcMsg::Transfer` rides the ICS-20 `transfer` port, delivering no callback to the controller's `wasm.<addr>` port. Every sudo-free confirmation therefore requires a Solana-side (`ibc-solray`) change. Until that lands, keeping safe-delivery alive for `TransferOut` alone would force retention of essentially the *entire* machinery — `ForwardToInnerMsg` and the `*RespDelivery` variants are shared by all composite states — while fragmenting the state machine into two callback models. That buys almost no simplification for real added confusion.

The gate is the sub-decision in §4 (Option A vs B) plus the two pivotal `ibc-solray` capability questions (§8). **This ADR is Proposed; the gate is unresolved.**

---

## 3. Scope boundary and the honest-IBC replay argument

Idempotency is **not** a new concern for state-advancing handlers under relayer-retry. A controller ack is retried **only** when its `WasmMsg::Execute(RemoteLeaseCallback)` reverted (`Err`) — i.e. precisely when state did **not** advance — so re-running the handler on unchanged persisted state is idempotent by construction. On `Ok` the packet commitment is deleted, so there is no re-delivery. The current honest-IBC dedup argument is thus *replaced by a stronger* commitment-deleted-on-`Ok` invariant. Only the **terminal / late-ack absorbers** must stay authenticated and idempotent (they can legitimately receive a duplicate on the UNORDERED channel), via the already-wired `authz_remote_callback`.

---

## 4. The gating sub-decision — outbound transfer-out migration (Options A & B)

The **passive-vault principle governs this entire section**: all policy and business logic live on Nolus; Solana is a mechanical vault + swap executor that neither indexes by user nor holds per-operation protocol state (`docs/remote-lease-wire-contract.md:67-69`, ADRs 0001/0002 in `ibc-solray`). Any option that forces Solana to correlate/index by operation is in tension with it, and the decisive unknowns are Solana-side capabilities in the *external* `ibc-solray` repo.

Option C (ibc-go callbacks middleware / IBC hooks) is dispatched and rejected in §4.6 before the finalists A and B are compared.

### 4.1 Shared context / constraints (verified against code)

- **On open, funding transfers TWO coins.** `BuyAsset::coins()` returns `SwapCoins::Two(downpayment, loan.principal.into_super_group())` (`protocol/contracts/lease/src/contract/state/opening/buy_asset/mod.rs:142-144`). The dex `TransferOut` state's `acks_left = coins.len()` and it waits for **all** acks before entering `SwapExactIn` (`protocol/packages/dex/src/impl_/transfer_out/mod.rs`: `acks_left` `:57`, `new` `:69`, `coins_len` `:105`, `on_response` last-ack→`SwapExactIn` `:208`). On repay the funding is a **single** coin — `SwapCoins::One(self.payment)` (`protocol/contracts/lease/src/contract/state/opened/repay/buy_lpn.rs:112`), `dex::start_local_local` at `:40`. Close/sell has **no** outbound transfer-out — `dex::start_remote_local` (`protocol/contracts/lease/src/contract/state/opened/close/sell_asset/task.rs:36`) — already sudo-free.
- **The swap request names EXACT input amounts + a slippage floor.** `SwapParams::One { coin_in, min_out }` / `Two { coin_in_1, coin_in_2, min_out }` (`protocol/packages/remote_lease/src/swap/mod.rs:23`; wire `From`-impls at `msg.rs:287-307`). The Solana vault must already **hold** the named coin(s) to execute.
- **The RETURN leg (Solana→Nolus) is already sudo-free.** Nolus confirms arrival by **idempotent bank-balance polling** — `TransferInFinish::try_complete` (`transfer_in_finish.rs:124`) polls via `transfer_in::check_received` (`transfer_in.rs:12`) at the poll call site `transfer_in_finish.rs:234` — guarded by the cross-repo invariant "*Solana return-transfer IBC timeout < Nolus re-issue window (`dex::IBC_TIMEOUT` = 1 day, `protocol/packages/dex/src/transport/mod.rs:17`)*" (`docs/remote-lease-wire-contract.md:27`) so an in-flight refund cannot double-credit.
- **A coin already in the lease OUTPUT currency is excluded from the swap request** (`not_out_coins_filter`) and re-added to `amount_out` on Nolus (`docs/remote-lease-wire-contract.md:26`) — funded but not swapped.
- **`ibc_packet_receive` currently REJECTS all inbound packets** (`protocol/contracts/remote_lease/src/ibc.rs:118`, returns `StdAck::error(UnsupportedInboundPacket)`). ADRs 0001/0002 (the passive-vault contract) live in the **external** `nolus-protocol/ibc-solray` repo — so **Solana-side capability is the decisive unknown**.

### 4.2 Option A — Solana emits a funding-confirmation packet

- **Mechanism.** Nolus sends the ICS-20 funding; on receipt the Solana side emits a packet back over the remote-lease controller channel; the controller's `ibc_packet_receive` (today rejecting, `ibc.rs:118`) must **accept** it and dispatch a **new** `RemoteLeaseCallback` variant (e.g. `FundingConfirmed`) to the lease, which then fires the swap. Same fund→confirm→swap sequencing as today, over the controller channel instead of Neutron sudo.
- **Needs on Nolus.** New `RemoteLeaseCallback`/`OperationResponse` variant; enable + **authenticate** `ibc_packet_receive`; correlate the confirmation to the pending `TransferOut` via an **opaque MEMO echoed back** (passive-vault-safe correlation).
- **Needs on Solana (`ibc-solray`).** (i) an ICS-20 receive hook on deposit, (ii) with memo access, (iii) able to emit a packet back over the controller channel echoing the memo, **statelessly**.
- **Pros.** Deterministic sequencing; no swap retry churn; funding failure surfaced cleanly (no confirmation ⇒ funding timed out ⇒ refund observed); clean swap-failure semantics; two-coin open handled naturally.
- **Cons.** Most new protocol surface on **both** sides; real passive-vault tension (the vault also "acks deposits"); extra round-trip latency; infra dependency on an `ibc-solray` ICS-20-memo→emit-packet capability that may not exist.

### 4.3 Option B — fire funding + swap together; the swap ack is the only sync point

- **Mechanism.** Nolus emits the funding ICS-20 **and** the controller `Swap` packet in the **same tx** (different channels — transfer port vs `wasm.<addr>` port). The vault, on the `Swap` packet, checks its balance: if the named coin(s) are present it swaps; else it returns a **RETRYABLE "insufficient funds" error-ack**. Nolus retries the swap until funds land. **No funding ack at all.**
- **Needs on Nolus.** Distinguish "insufficient funds → retry" from a genuine swap failure in `SwapExactIn`'s error path — today that path is slippage/anomaly semantics (`ReportAnomalyCmd` / `AnomalyTreatment`, `swap_exact_in/mod.rs:182`/`:245`), which would **misclassify** funds-not-here as a slippage anomaly; a funding-timeout / double-fund invariant (funding-transfer timeout < swap re-issue/timeout window, analogous to the return-leg invariant); retry-cadence / timeout tuning.
- **Needs on Solana.** Ideally **nothing new** — *if* the swap already fails on insufficient balance with a **distinguishable** error. If it fails generically, Solana needs a small change to make insufficient-funds distinguishable.
- **Big upside.** B does not **relocate** the funding ack, it **eliminates** it — funding becomes fire-and-forget, so the `TransferOut` dex state stops waiting on anything and can **collapse** into "fire funding, enter `SwapExactIn` directly." Removes the last sudo consumer outright; a bigger simplification than A.
- **Pros.** Most passive-vault-aligned (the vault stays a pure "swap if funds present, else fail cleanly" executor); least new surface; fewer round-trips; actually kills the funding ack.
- **Cons.** Retry churn against the relayer while funds are in flight; funding-failure detection is **indirect** — a bounced funding surfaces only as the swap eventually hitting its own `OperationTimeout`, after which the lease reconciles via balance-polling before re-issuing; needs the double-fund invariant + swap-error reclassification.
- **Refined B (recommended shape).** Fire-and-forget funding + swap-retry-on-insufficient, with funding-failure recovery via the **same idempotent bank-balance polling** already used for the return leg (`check_received`), triggered on swap `OperationTimeout`. **100% of new logic on Nolus**; reuses proven machinery; asks Solana for at most a distinguishable insufficient-funds error.

### 4.4 Two-coin open under B (the case the maintainer specifically flagged)

B **still holds**, but the two-coin open is where B's vault-blindness costs the most.

- **Why it holds.** You do **not** track the two fundings individually — the **swap is the aggregate gate**. Fire both funding transfers (fire-and-forget) + the `Two` swap; the swap succeeds only once **both** named inputs are in the vault, so its success is the proof both arrived. Two coins just widen the "not-all-present-yet" retry window.
- **Hard requirement it exposes.** The multi-coin swap **must be all-or-nothing** (atomic over the requested input set). If the vault holds `coin1` but not `coin2`, the `Two` swap must fail wholesale, **not** partially swap `coin1` and skip `coin2` — a partial execution would be a **silent correctness bug** (wrong `amount_out` + orphaned vault coin). Single-coin B never had to care about this. **Nothing in `nolus-money-market` enforces or verifies this atomicity** — it is entirely a property of the external `ibc-solray` vault, which is exactly why it is a pivotal open question (§8 Q3), not an assertable fact here.
- **Partial-bounce reconcile.** If one funding bounces (ICS-20 auto-refunds that coin to the lease) while the other lands, the `Two` swap fails insufficient-funds until its `OperationTimeout`. Recovery re-funds **only the shortfall** — computable from Nolus's **own** balance (a bounced coin is refunded to the lease; a landed coin is not), so "re-fund whatever came back" is correct **without observing the vault** and **even when both coins share a denom** (re-fund the returned total). Load-bearing invariant: funding-transfer timeout < swap re-issue/timeout window, so by reconcile time every funding has resolved (no in-flight ambiguity, no double-fund). Reuses the existing `check_received` idempotent balance-polling.
- **Wrinkle.** A funded coin already in the **output currency** is excluded from the swap request and re-added to `amount_out` on Nolus, so it is funded but sits **outside** the swap gate — its arrival is proven only at the **final transfer-in balance poll**, not by swap success.
- **Net.** Two-coin open tilts toward A for the open leg (A confirms each funding ⇒ two coins trivial and deterministic, no reconcile logic, no swap-atomicity requirement). A reasonable **hybrid** is **A-for-open / B-for-repay** (single coin, where B is clean), at the cost of maintaining two mechanisms.

### 4.5 Comparison

| dimension | A (confirmation packet) | B (fold into swap) |
|---|---|---|
| Sequencing | fund → confirm → swap (deterministic) | fund + swap together; swap retries |
| New Nolus surface | new callback variant, inbound `ibc_packet_receive`, memo correlation | swap-error reclassification, double-fund invariant, retry tuning |
| New Solana surface | ICS-20 receive hook + memo + emit-back packet | ideally none; at most a distinguishable insufficient-funds error |
| Passive-vault fit | stretched (vault also acks deposits) | best (vault stays a pure swap executor) |
| Funding ack | relocated to controller channel | **ELIMINATED** (bigger simplification) |
| Funding-failure detection | clean (missing confirmation) | indirect (swap timeout → balance reconcile) |
| Latency | extra round-trip | none added |
| Two-coin open | handled naturally (per-funding confirmation) | works but needs atomic swap + partial-bounce reconcile |
| Main risk | `ibc-solray` may lack memo→emit; passive-vault stretch | retry churn + must not conflate transient vs permanent swap failure |

### 4.6 Rejected — Option C (ibc-go callbacks middleware / IBC hooks)

Chain-agnostic and needs no Solana logic change, but its ack (`IBCLifecycleComplete`) is **still a gas-capped, chain-delivered, sudo-class callback**, so safe-delivery boxes A–D could not be deleted for the funding leg. It fails the removal goal and additionally requires the destination to have the callbacks middleware wired (an infra dependency likely absent). **Rejected.**

### 4.7 Pivotal `ibc-solray` questions (the cheap checks that settle the decision)

- **For B:** does the swap executor (a) treat the requested input set **atomically** (all-or-nothing) and (b) return a **distinguishable retryable insufficient-funds** error? If yes, two-coin B is sound and nearly free on Solana.
- **For A:** can `ibc-solray` surface an ICS-20 deposit to the vault program **with memo access** and **emit a packet back** over the controller channel, **statelessly**?

### 4.8 Recommendation

**Refined B is the target shape *iff* the pivotal Solana capability lands** — it is the most passive-vault-aligned, asks the least of Solana, eliminates the funding ack outright, and reuses the trusted balance-polling machinery. But that recommendation travels with two caveats it must not be read without:

1. **B is contingent on §4.7/Q3.** Without a **distinguishable retryable insufficient-funds** error *and* an **atomic** multi-coin swap on `ibc-solray`, B cannot distinguish transient "funds not here yet" from a permanent swap failure and cannot guarantee two-coin correctness — and there is nothing in this repo that enforces either. If Q3 does not resolve in B's favor, B is not viable as stated.
2. **Every lease-open funds two coins (§4.1), and two-coin is the case B handles *worst* (§4.4, "tilts toward A for the open leg").** So the open leg — the mainline path — specifically favors A's deterministic per-funding confirmation. A blanket "B overall" understates that the most common operation is the one B is weakest at.

Taken together, the **honest default is the A-open / B-repay hybrid** (A's determinism where two coins and correctness matter most; B's clean single-coin fire-and-forget for repay), at the cost of maintaining two mechanisms — **unless** the two pivotal questions (§4.7 / §8 Q3–Q4) resolve cleanly in B's favor, in which case uniform Refined B is preferable. The decisive input is **external** (`ibc-solray` capabilities); the two pivotal questions settle it.

---

## 5. Implementation plan

Phases A–G are the plan of record **for when the gate opens**; **Phase 0 runs now** (ungated), and **Phase B is additive and can also land now** (it fixes latent correctness bugs regardless of timing).

### Phase 0 — Preconditions & decisions (now, ungated)

Gather the decisions only the maintainer/architect + `ibc-solray` owner can make; lock the facts the deletion depends on; no production code change. Confirm the **outbound-migration direction** (A vs B vs hybrid; C rejected); confirm **v10 deployment status** (below); record the **storage-encoding finding**.
*Exit:* direction chosen and `ibc-solray` work scoped/committed; v10 status known; storage decision pre-agreed conditional on that status.

### Phase A — outbound transfer-out migration off sudo (the gated prerequisite)

Give the outbound `TransferOut` leg (open funding + repay forwarding) a sudo-free completion path. This phase is **what the gate blocks** (the gate itself is the §2/§4/§8 decision, not this work). **Hard prerequisite for every deletion below; requires the `ibc-solray` change.** Replace the emitter at `bank_ibc/local.rs:85` and the lease seam `transport/transfer_out.rs:19` (`TransferOutFactory`/`LocalSender`); route completion through `on_remote_lease_callback` (`state/dex.rs:63`) instead of `SudoMsg` (`endpoins.rs:186`); cover both consumers via the shared dex `TransferOut` state — open (`buy_asset/mod.rs:55`, `dex::start_local_remote`) and repay (`repay/buy_lpn.rs:40`, `dex::start_local_local`); **define + document the double-fund invariant** (outbound analog of the return-leg timeout-vs-reissue invariant).
*Exit:* open + repay funding complete via a controller callback with no `SudoMsg`; double-fund invariant documented and tested.

### Phase B — absorber classification & Heal coverage (additive; lands while safe-delivery is still present)

Make every synchronous handler classify **transient (`Err`→retry)** vs **permanent (`Ok`→terminal/recovery)**, and make `Heal()` a real backstop. Additive impls only — safe before flipping the transport, so no window exists where a failure wedges. Reference absorber: `OpenLease` → `on_open_failed` → terminal `OpenFailed` (refund + event, `Ok`; `open_lease.rs:261/267`), late-ack template `open_failed.rs:56`.

- **`SwapExactIn` decode failures of a SUCCESS ack** (`decode_resp.rs`): `NotSwapResponse` (`:158`) is a clean permanent ⇒ terminal. Corrupt bytes (`:151`), wrong out-currency (`:172`), amount overflow (`:116`) are the **hard stuck-funds case** — swap executed on Solana but proceeds unrecoverable on Nolus; divert to a **re-drivable recovery state**, not a lossy auto-terminal (see §8).
- **`TransferInInit` missing `on_error`** (the `Handler` impl at `transfer_in_init.rs:157` defines `on_response`/`on_timeout`/`heal` but no `on_error`): add `on_error` that re-enters (re-sends transfer-back), returning `Ok` — a transfer-back rejection is transient.
- **`TransferInFinish` missing late/duplicate absorber** (the `HandlerT` impl at `transfer_in_finish.rs:263` defines only `authz_remote_callback`/`heal`/`on_time_alarm` — no `on_response`/`on_error`/`on_timeout` override): a late/duplicate `RemoteLeaseCallback` on the UNORDERED channel hits the default `on_response`/`on_error`/`on_timeout` → `Err` → infinite retry. Add an **authenticated, idempotent, no-mutation, `Ok`-returning** absorber — the analog of `OpenFailed::on_remote_lease_callback`, auth via `spec.authz_remote_callback`.
- **Heal coverage gaps:** `SwapExactIn` (`swap_exact_in/mod.rs:153`) and `TransferOut` (`transfer_out/mod.rs:187`) have no `heal`, so composite `State::heal` delegates to default `Err`. Add `heal` (re-enter/re-attempt) to both.
- **Verify (no change):** `SwapExactIn::on_error` (`:182`/`:245`) → `AnomalyTreatment::{Retry,Exit}` already returns `Ok`; `on_timeout` re-enters `Ok`; `OpenLease` success-path construction (`RemoteAccount::try_from` `open_lease.rs:120`, `buy_asset::start().enter()` `:129-137`) must be confirmed infallible-for-hardened-input or routed to `OpenFailed`; the absorb path's own overflow guards (`open_lease.rs:293/:299`).

*Classification posture (bake in):* conservative — ambiguous cases return `Err` (retry) **plus** a `Heal` path, never a lossy auto-terminal.
*Exit:* every leaf handler returns `Ok`-into-terminal/recovery for deterministic failures and `Err` only for genuinely transient ones; `Heal()` advances all wedge-prone states.

### Phase C — flip the transport to synchronous + eliminate the `OperationOk` JSON round-trip

The behavioral switch (do both together; they touch the same arms). Rewire `on_response` arms from `impl_::forward_to_inner::<_,ForwardToInnerMsg,Self>(inner,response,env)` to direct `Handler::on_response(inner,response,querier,env).map_into()` (`out_local.rs:313/319/325`, `out_remote.rs:212/219`) — this is what makes inner failures surface as `Err` at `ibc_packet_ack` time, which is why **Phase B must precede it**. Eliminate the round-trip: change `Handler::on_response` (`response.rs:53`) from `Binary` to typed `OperationResponse`; drop `decode_swap_response`'s `from_json` (`decode_resp.rs:150-151`); update `state/dex.rs:74` (`OperationOk(response)` → `on_dex_response` collapses) and every leaf/composite `on_response`.
*Exit:* controller callbacks run the real dex transition synchronously; no JSON re-encode remains; full suite green (watch message/event cardinality — the `DexCallback` self-submessage is gone).

### Phase D — delete the dead safe-delivery machinery + the `ForwardToInnerMsg` cascade

Whole-file deletes: `dex/src/impl_/resp_delivery/mod.rs` (`ResponseDeliveryImpl`, `REPLY_ID`, `setup_next_delivery`/`do_deliver`/`do_redeliver`) + `resp_delivery/adapter.rs`; `dex/src/resp_delivery.rs` (`ForwardToInner`) and `dex/src/time_alarm.rs` (`TimeAlarm` trait — **not** `transfer_in::setup_alarm` nor `TimeAlarmsRef::setup_alarm`); `lease/src/contract/state/resp_delivery.rs` (`ForwardToDexEntry`). Module/export removals in `impl_/mod.rs` (`mod resp_delivery`, `pub use ResponseDelivery`, the three `*RespDelivery` aliases, `fn forward_to_inner`), `lib.rs` (2 exports + 2 mods), `contract/state/mod.rs:44`. Trait-method deletes: `dex/src/response.rs` `Handler::on_inner` + `Handler::reply` (**keep** `on_time_alarm` — `TransferInFinish` uses it) and the three `TimeAlarm` impl blocks. Remove all 5 `*RespDelivery` variants + every `From`/handler/contract/display/migration arm (`out_local.rs`, `out_remote.rs`).

**Mandatory `ForwardToInnerMsg` cascade (E0392 if missed):** thread the now-unused param out of `enum State`, the `StartXxxState` aliases + `start_*` fns, the `SwapExactIn` Handler impls, the `TransferIn{Init,Finish}` `Display` names (cosmetic; **keep** `TransferInFinish` + its poll/`on_time_alarm`), and the four lease consumers (`buy_asset`, `buy_lpn`, `sell_asset`, `paid/transfer_in`). Lease-side deletes: `api/mod.rs:80` `ExecuteMsg::DexCallback`; the `endpoins.rs` DexCallback arm; `state/dex.rs` `on_dex_inner` + `reply` override; `contract/api.rs` `Contract::on_dex_inner`; `access-control` `DexResponseSafeDeliveryPermission` alias (**keep** `SameContractOnly`).

**Keep (do not touch):** the lease `reply` entrypoint + `Contract::reply`/`Handler::reply` (LPP open-loan `OPEN_LOAN_REQ_ID=0`); `on_time_alarm` + `TransferInFinish` balance polling; `SameContractOnly`; `RemoteLeaseCallback`; `Heal`. Run the `migration`-feature gate too (cfg-gated `*RespDelivery` arms).
*Exit:* grep for `DexCallback`/`ResponseDelivery`/`REPLY_ID`/`forward_to_inner`/`ForwardToInner`/`ForwardToInnerMsg` empty in production; builds under default + `migration`.

### Phase E — delete the sudo surface (only after Phase A removed the last consumer)

Delete the lease `sudo` entry_point (`endpoins.rs:106-113`), `process_sudo` (`:178-198`) and its `use sdk::api::SudoMsg` (`:11`) — the unbounded counterparty `details` echo (`:191-192`, a CLAUDE.md bound-length violation) disappears with it and must **not** be reintroduced. Delete the `ica.rs` transport (`SudoMsg`, `RequestPacket`, `InterChainMsg`, `IbcFee`, `RequestPacketTimeoutHeight`, `impl From`/`CustomMsg`); revert `sdk/src/lib.rs` `cosmwasm_ext::{Response,CosmosMsg,SubMsg}` from `InterChainMsg`-parametrized back to `Empty`; remove the `bank_ibc/local.rs` emitter; retarget `sdk/src/testing/{mod,contract_wrapper}.rs` `InterChainMsg`→`Empty` in lockstep; rewrite/delete `tests/src/common/ibc.rs:75-102` `send_response`/`send_blank_response`. **Keep** `platform/src/remote.rs::ErrorResponse` (still the type for `OperationErr`) and `trx::decode_msg_responses`.
*Exit:* no `SudoMsg`/`InterChainMsg`/`IbcFee`/`RequestPacket` refs; the only surviving `fn sudo` handlers are the unrelated per-contract governance enums (`leaser`/`lpp`/`oracle`/`treasury`/`admin`/`timealarms`, each on its own `msg::SudoMsg`).

### Phase F — storage version & migrate (decided in Phase 0, executed at deploy)

Persisted composite `State` is **externally-tagged JSON** (`#[derive(Serialize, Deserialize)]`, default serde), so removing unused `*RespDelivery` variant **names** is safe — no positional index shift. **If v10 is unreleased** (no live leases in an in-flight dex state; mainnet population is zero per the flow doc): in-place change, **no bump**, `migrate` stays refusing (`UnsupportedMigration`) per the wire-format-versioning-in-place precedent (PR #736); update only the migrate doc comment. **If v10 is deployed with live leases:** bump `CONTRACT_STORAGE_VERSION` (`endpoins.rs:29`) to 11 and **drain all leases to a terminal state before upgrade** (flow doc line 174 — there is no `ExecuteMsg` escape hatch; a lease persisted **in** a `*RespDelivery` state would misdeserialize).

### Phase G — tests, docs, CI reconciliation

No test asserts safe-delivery symbols, so the work is re-verification for cardinality drift + doc rewrites. Re-run `remote_lease_callback.rs`, `remote_lease_swap.rs` (incl. `swap_delayed_ack_visible_in_query` — controller stub `Delayed`, **kept**), `heal.rs`, and the lifecycle suites; fix any drained-count/`expect_empty`/`assert_event` assertion that counted the removed `DexCallback` submessage. **Preserve** `common/swap.rs:92` `deliver_transfer_in` (TransferInFinish poll alarm) and `remote_lease_controller_stub.rs:96` `Delayed`/`deliver_pending` — both distinct from safe-delivery. Rewrite `protocol/docs/remote-lease-callback-flow.md` (boxes A–D → synchronous model), `lease/src/api/mod.rs` doc comments for `RemoteLeaseCallback` and `Heal`, and the root `CLAUDE.md` project-overview paragraph. **CI:** no change expected — safe-delivery uses only `reply` + time-alarm, no `cosmwasm_X_Y` capability, no cargo-each tag or `[workspace.lints]` change; the `ci/Containerfile` allowlist is untouched (those capabilities belong to the `remote_lease` controller).

### 5.1 Blast-radius table

Size: S ≤ ~20 LOC / trivial · M = one file's arms/impls · L = multi-file cascade or new design. Rows tagged **(A-only)** are direction-contingent — they exist only if Phase A adopts Option A; the `(A-only)` marker calls them out so a B/hybrid reading drops them.

| File | Action | Size | Phase |
|---|---|---|---|
| `ibc-solray` (Solana app, **external** repo) | add — outbound confirmation path | **L** | A |
| `platform/.../bank_ibc/local.rs` | modify then delete emitter | M | A→E |
| `lease/.../transport/transfer_out.rs` | modify (rework factory) | M | A |
| `dex/.../transfer_out/mod.rs` | retarget completion + add `heal` | M | A/B |
| `remote_lease/src/callback.rs`, `msg.rs`, `contracts/remote_lease/src/ibc.rs` | add variant / accept inbound **(A-only)** | M | A |
| `dex/.../swap_exact_in/mod.rs` | add `heal`, classify decode failures | M | B |
| `dex/.../swap_exact_in/decode_resp.rs` | permanent-failure classification + drop `from_json` | M | B/C |
| `dex/.../transfer_in_init.rs` | add `on_error` | S | B |
| `dex/.../transfer_in_finish.rs` | add late-ack absorber | M | B |
| `lease/.../opening/open_lease.rs` | verify/classify construction failures | S | B |
| `dex/src/response.rs` | delete `on_inner`/`reply`; change `on_response` sig | M | C/D |
| `dex/.../out_local.rs` | arm rewire + variant/cascade delete | **L** | C/D |
| `dex/.../out_remote.rs` | arm rewire + variant/cascade delete | **L** | C/D |
| `lease/.../state/dex.rs` | delete `on_dex_inner`/`reply`; drop round-trip | M | C/D |
| `dex/.../resp_delivery/mod.rs` + `adapter.rs` | **delete files** | M | D |
| `dex/src/resp_delivery.rs` (`ForwardToInner`) | **delete file** | S | D |
| `dex/src/time_alarm.rs` (`TimeAlarm`) | **delete file** | S | D |
| `dex/.../impl_/mod.rs` | delete aliases + `forward_to_inner` + mod | M | D |
| `dex/src/lib.rs` | delete 2 exports + 2 mods | S | D |
| `dex/.../{transfer_out,transfer_in_init,swap_exact_in}` | delete `TimeAlarm` impls | S | D |
| `lease/src/api/mod.rs` | delete `DexCallback` + doc rewrites | S | D/G |
| `lease/.../endpoins.rs` | delete DexCallback arm (D) + sudo surface (E) | M | D/E |
| `lease/.../contract/api.rs` | delete `on_dex_inner` + doc | S | D |
| `lease/.../state/resp_delivery.rs` (`ForwardToDexEntry`) | **delete file** | S | D |
| `lease/.../state/{buy_asset,buy_lpn,sell_asset,transfer_in}` | drop `ForwardToDexEntry` + phantom | M | D |
| `access-control/src/permissions.rs` | delete `DexResponseSafeDeliveryPermission` alias | S | D |
| `platform/packages/sdk/src/ica.rs` | delete `SudoMsg`/`RequestPacket`/`InterChainMsg`/`IbcFee`/… | M | E |
| `platform/packages/sdk/src/lib.rs` | drop `SudoMsg`, revert `cosmwasm_ext` aliases | S | E |
| `sdk/src/testing/{mod,contract_wrapper}.rs` | retarget `InterChainMsg`→`Empty` | M | E |
| `tests/src/common/ibc.rs` | rewrite/delete `send_response` | S | E |
| `lease/.../endpoins.rs` (`CONTRACT_STORAGE_VERSION`, `migrate`) | version + doc decision | S | F |
| `protocol/docs/remote-lease-callback-flow.md`, root `CLAUDE.md` | doc rewrite | M | G |
| `tests/src/lease/*` lifecycle suites | verify cardinality, fix counts | M | G |

---

## 6. Consequences (lose / gain ledger)

**Lose:**

- **Host-driven → relayer-driven recovery.** Today failures during response processing retry locally via the time-alarm loop (boxes C/D), independent of relayer liveness. After removal, retry of a confirmed-but-failing-to-process op depends on the relayer resubmitting the ack (or a human calling `Heal`). A real reduction in autonomy on the controller legs — acceptable because the controller transport gives native relayer-retry and `Heal` remains, but a genuine trade.
- **The gas-cap safety margin for the outbound leg — only if Phase A is skipped.** Deleting safe-delivery while `TransferOut` still rides Neutron `SudoMsg` re-exposes exactly the fixed-gas + failure-queue problem it was built for. This is why the whole program is gated on Phase A.
- **Counterparty-state agreement nuance.** None lost on-chain (the commitment-deleted-on-`Ok` invariant is *stronger* than the current honest-IBC dedup argument), but the passive-vault principle is put under pressure by Phase A Option A (a per-lease Solana confirmation packet risks making Solana correlate by operation) — must be kept stateless/memo-echo to stay compliant.

**Gain:**

- **Entrypoint reduction:** the lease `sudo` entry_point + `process_sudo` gone; the `DexCallback` `ExecuteMsg` + its permission gone.
- **Enum-variant reduction:** 5 `*RespDelivery` composite variants removed (3 `out_local` + 2 `out_remote`) and the `ForwardToInnerMsg` type parameter threaded out of ~6 dex files + 4 lease consumers.
- **Whole-file deletions:** `resp_delivery/mod.rs`, `resp_delivery/adapter.rs`, `resp_delivery.rs`, `time_alarm.rs`, `state/resp_delivery.rs`, plus the `ica.rs` Neutron transport enums — order-of-hundreds LOC net removal.
- **JSON round-trip eliminated:** `OperationOk` → `to_json_binary` → `on_dex_response(Binary)` → `from_json` collapses to a typed `OperationResponse` passed directly; `Handler::on_response` stops taking `Binary`. Removes a serialize/deserialize per callback and a class of decode-of-our-own-encoding failure.
- **One transport model** (once Phase A + E land): every DEX operation completes via the controller callback with a single documented `Err`/`Ok` contract — no more two-transport reasoning (controller relayer-retry vs Neutron failure-queue).
- **Clearer failure semantics:** explicit transient/permanent classification per state, replacing the implicit "defer-and-retry-locally-forever" default.

---

## 7. Pre-mortem — assume it shipped and broke

1. **A permanent failure wedges an ack forever (infinite relayer retry).** *Trigger:* after Phase C removes `forward_to_inner`, a deterministic failure (bad decode / wrong out-currency / overflow at `decode_resp.rs:151/172/116`, or a handler with no absorber) returns `Err` at `ibc_packet_ack`; the packet commitment is retained and the relayer resubmits the identical ack every block, permanently. *Mitigation:* Phase B lands **before** Phase C; every reachable leaf handler is proven to return `Ok`-into-terminal for deterministic failures; a post-Phase-C grep confirms no operation reaches a default `Err(unsupported)` arm. The two known gaps (`TransferInInit::on_error`, `TransferInFinish` late-ack) are closed in B.
2. **Stuck-funds on an undeliverable SUCCESS swap ack.** *Trigger:* Solana executed the swap (`OperationOk`) but the proceeds amount is unrecoverable on Nolus (corrupt bytes / wrong currency / overflow). A naive terminal strands the funds; a naive `Err` loops forever. *Mitigation:* Phase B routes these to a **re-drivable recovery state** (not a lossy terminal) and adds `SwapExactIn::heal` so `Heal()` can advance it after a code/migration fix. Explicitly escalated to the maintainer (§8) — do not auto-terminal.
3. **Permanent failure *inside* a permanent-failure absorber.** *Trigger:* an overflow in `OpenLease`'s refund path (`open_lease.rs:293/299`) prevents ever reaching `OpenFailed`; the ack `Err`s forever with only `Heal`/migration as recovery. *Mitigation:* Phase B verifies the absorb path's math is transient-only or reachable via `Heal`; the conservative posture (Err + Heal, never lossy terminal) plus complete `Heal` coverage is the escape valve. Documented as a known residual.
4. **Relayer-liveness gap strands a confirmed operation.** *Trigger:* removing host-driven redelivery means a genuinely-completed-on-Solana operation whose ack keeps failing relies entirely on the relayer resubmitting; if the relayer goes dark, nothing local re-drives it. *Mitigation:* `Heal()` (public, idempotent, `api/mod.rs:104`) is the manual backstop, extended in Phase B to `SwapExactIn` and `TransferOut`; the invariant "`Ok` deletes the commitment, `Err` retains it" is documented so operators know a wedged op is always Heal-recoverable.
5. **Losing the reply-based sub-message error catch that `Heal` depended on.** *Trigger:* Phase D deletes the dex `Handler::reply` and the `DexState::reply` override; if the LPP open-loan reply (`OPEN_LOAN_REQ_ID=0`) or the final-repay LPP-submsg out-of-gas catch (`active.rs:267`) were accidentally routed through the deleted path, opens/repays silently stop recovering. *Mitigation:* Phase D explicitly **keeps** the lease `reply` entrypoint + `Contract::reply`/`Handler::reply`; a targeted test asserts open-loan reply and the final-repay Heal path still fire; the deletion is scoped to the *dex-package* `Handler::reply` only.
6. **Stranded persisted `*RespDelivery` state on upgrade.** *Trigger:* a live v10 lease sitting in a transient `*RespDelivery` state at upgrade; after Phase D that variant name no longer deserializes → the lease is bricked. *Mitigation:* Phase 0 confirms v10 deployment status; Phase F either keeps it in-place (unreleased) or bumps to 11 and **drains all leases to terminal before upgrade**. The JSON-name-tagged encoding bounds the risk to leases actually *in* a `*RespDelivery` state (a ~1-block window); the drain removes it entirely.

*Bonus watch-item — silent test drift:* removing the `DexCallback` self-submessage changes message/event cardinality inside driving txs; lifecycle tests using `expect_empty`/`ignore_response`/`assert_event` may pass-then-fail on count. Phase G re-runs the full suite specifically for cardinality, not assertion rewrites.

---

## 8. Open questions / decisions needed

1. **Is a coordinated `ibc-solray` (Solana) change in scope and available now?** The decisive gate for the entire program — without a sudo-free outbound-completion signal, full removal cannot proceed.
2. **Outbound-completion direction: A, B, or hybrid?** (A) new Solana→Nolus confirmation packet + a `RemoteLeaseCallback` variant; (B) fold funding into the Swap ack with a retryable "insufficient funds" ack; **hybrid** = A-open / B-repay. C (callbacks-middleware) is rejected (still a gas-capped sudo-class callback). Requires the protocol architect + `ibc-solray` owner. Note the §4.8 default: absent a clean B-favorable answer to Q3, the honest default is the hybrid, because every open funds two coins and two-coin favors A.
3. **Pivotal `ibc-solray` capability — for B:** does the swap executor treat the requested input set **atomically** (all-or-nothing) and return a **distinguishable retryable insufficient-funds** error? Nothing in `nolus-money-market` enforces or verifies this today; B's viability and two-coin correctness both depend on it.
4. **Pivotal `ibc-solray` capability — for A:** can it surface an ICS-20 deposit to the vault program **with memo access** and **emit a packet back** over the controller channel **statelessly** (needed for passive-vault-compliant correlation)?
5. **Is v10 deployed to any testnet/devnet with live leases** that could sit in an in-flight dex sub-state at upgrade? Determines in-place (no bump, migrate stays refusing) vs bump-to-11 + drain-before-upgrade. Mainnet population is zero per the flow doc.
6. **Recovery target for an undeliverable SUCCESS swap response** (repay/close, `decode_resp.rs:151/172/116`): (a) a new re-drivable "proceeds-unrecoverable" recovery state fixed via `Heal`/migration, (b) accept the `Err` loop until migration, or (c) bounded local retry. The one case with no clean terminal — a product decision.
7. **`SwapExactIn::on_error` attempt-bounding:** is unconditional identical retry (`AnomalyTreatment::Retry`) always correct, or can the vault return a *deterministic* `OperationErr` (permanent slippage/price condition) that needs an attempt bound / permanent classification to avoid a benign-`Ok`-but-never-progressing loop?

---

## 9. Appendix — verification evidence

Compact, auditable table of the load-bearing code facts. Verdict labels map to the safe-delivery-integrity conclusions **C1–C4 (§1.4)** and the mechanism facts **M1–M5 (§1.4, "Mechanism facts")**.

| Fact | Evidence (file:line) | Verdict |
|---|---|---|
| Box A only wraps + schedules `reply_on_error(DexCallback)`; no business logic | `enter` `resp_delivery/mod.rs:106`, `reply_on_error` call `:111`; entry `impl_/mod.rs:56`; arms `out_local.rs:313/319/325`, `out_remote.rs:212/219` | C1 CONFIRMED (high) |
| Real decode/transition runs only in box B (`on_inner`→`do_deliver`) | `resp_delivery/mod.rs:167` (`on_inner`), `:129` (`do_deliver`) | C1 CONFIRMED |
| Box D retry is a distinct path: `on_time_alarm`→`do_redeliver`→`deliver_again` | `resp_delivery/mod.rs:180` (`on_time_alarm`), `:133` (`do_redeliver`); `reply`→`setup_next_delivery` `:173`/`:137` | C1 CONFIRMED |
| Open ack bypasses safe-delivery — synchronous, no `ResponseDelivery` wrap | `open_lease.rs:240` (`on_remote_lease_callback`), `:114` (`on_open_lease_ack`) builds Account + `buy_asset::start().enter()`; plain dispatch `endpoins.rs:171` | C2 PARTIAL (high) |
| Two success-path steps fallible by signature; only PDA guard reachable, and it lacks a timeout fallback | `RemoteAccount::try_from` `open_lease.rs:120` (reachable, deterministic); `next.enter()` `:137` (`TransferOut::enter` = `Ok(...)`, infallible in practice); harden via `OpenFailed` (`open_lease.rs:252`) | C2 gap |
| In-flight states inherit default `Err` `on_response`; no idempotent absorber | `dex/src/response.rs:53`; `transfer_in_finish.rs:263` (`HandlerT` impl, no override); `resp_delivery/mod.rs:149` | C3 PARTIAL (high), LOW severity |
| Unreachable under honest IBC; blast radius bounded (revert + retry, funds untouched) | `endpoins.rs` `RemoteLeaseCallback` returns handler `Err` directly; controller reverts ack | C3 |
| `OperationOk` JSON round-trip real: `to_json_binary`→persist `Binary`→`from_json` | `state/dex.rs:74-78`; persisted `resp_delivery/mod.rs:46`; decode `decode_resp.rs:150` | C4 CONFIRMED (high), harmless |
| Lease is sole live `SudoMsg` consumer; `submit_transaction` fully removed (0 refs) | `endpoins.rs:107` sudo, `:186` `SudoMsg::Response`→`on_dex_response`; `platform/src/remote.rs` (no `submit_transaction`) | M1 |
| Other `fn sudo` handlers consume per-contract governance `msg::SudoMsg`, not `sdk::api::SudoMsg` | `leaser/src/contract.rs:179`; `lpp/src/contract/mod.rs:213`; `oracle/src/contract/mod.rs:161`; + platform `treasury`/`admin`/`timealarms` | M1 (Phase E exit) |
| Outbound emitter is Neutron `InterChainMsg::IbcTransfer`, scheduled no-reply | `bank_ibc/local.rs:85`, `:108` | M1 |
| Unbounded counterparty `details` echo (CLAUDE.md violation) dies with `process_sudo` | `endpoins.rs:191-192` | M1 (Phase E) |
| Controller dispatches ack via plain `add_message`, not `reply_*` | `remote_lease/src/ibc.rs:130` (`ibc_packet_ack`), `:194` (`add_message`) | M2 |
| Controller rejects all inbound packets today | `remote_lease/src/ibc.rs:118` (`UnsupportedInboundPacket`) | M2 (Option A hinge) |
| Return leg already sudo-free via self-balance polling | `transfer_in_finish.rs:124` (`try_complete`); poll `transfer_in.rs:12` (`check_received`) at `transfer_in_finish.rs:234` | M2 |
| Return-leg double-credit invariant (Solana timeout < Nolus `IBC_TIMEOUT` = 1 day) | `dex/src/transport/mod.rs:17` (`Duration::from_days(1)`); `docs/remote-lease-wire-contract.md:27` | M2 |
| Open funding transfers TWO coins; TransferOut waits for all acks; repay funds one | `buy_asset/mod.rs:142-144` `SwapCoins::Two`; `transfer_out/mod.rs:57/69/105/208`; repay `buy_lpn.rs:112` `SwapCoins::One`, `:40` | M3 (two-coin B) |
| Swap request names exact inputs + slippage floor | `swap/mod.rs:23` `SwapParams::{One,Two}`; wire `From` `msg.rs:287-307` | M3 |
| Non-swapped output-currency coin excluded from swap, re-added on Nolus | `docs/remote-lease-wire-contract.md:26` | M3 (two-coin wrinkle) |
| Multi-coin swap atomicity is an EXTERNAL ibc-solray property; unenforced here | (no local code) — see §4.4 / §8 Q3 | M3 (open question) |
| `SwapExactIn::on_error` already `Ok` via `AnomalyTreatment::{Retry,Exit}` | `swap_exact_in/mod.rs:182/245` | M4 |
| Absorber gaps: `TransferInInit` no `on_error`; `TransferInFinish` no late absorber | `transfer_in_init.rs:157` (`Handler` impl, no `on_error`); `transfer_in_finish.rs:263` (`HandlerT` impl, no override) | M4 |
| Heal gaps: `SwapExactIn` + `TransferOut` no `heal`; composite delegates default `Err` | `swap_exact_in/mod.rs:153`; `transfer_out/mod.rs:187` | M4 |
| Reference absorber: `OpenLease`→`OpenFailed` (`Ok`); late-ack template | `open_lease.rs:261/267`; `open_failed.rs:56` | M4 |
| Persisted composite `State` is externally-tagged JSON (name-safe variant drop) | `out_local.rs` `#[derive(Serialize,Deserialize)]` default serde | M5/F |
| Storage version currently 10; migrate refuses unconditionally | `endpoins.rs:29` (`CONTRACT_STORAGE_VERSION`), `:60-82` (`UnsupportedMigration`) | M5/F |
| CI allowlist untouched — safe-delivery uses only `reply`+time-alarm, no capability | `ci/Containerfile` `cosmwasm_capabilities` | M5/G |
