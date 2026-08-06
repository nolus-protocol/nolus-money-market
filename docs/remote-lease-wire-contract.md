# Remote Lease Wire Contract

The `remote_lease` crate defines the IBC packet types exchanged between the Nolus CosmWasm controller and the Solana Remote Lease App. Both sides deserialise the same Rust types via `serde`; the canonical definition lives in `protocol/packages/remote_lease/`.

## Pinned constants

- Protocol version: `nls-remote-lease.v1` (`remote_lease::VERSION`). Encoded on every packet as the `ProtocolVersion` ZST; mismatches are rejected at deserialisation, not in business code.
- IBC port: `nls-remote-lease.<dex>` — built via `remote_lease::port_id_for`.
- Callback error payload: max 200 bytes (`OPERATION_ERR_MAX_BYTES`), measured on the counterparty's prose **after** the code frame is stripped; enforced in the `RemoteErrorMessage` visitor before allocation. The payload length is limited to allow message acknowledgements to fit in the Solana return data, max size = 1024 bytes.
- Error code token: max 16 bytes (`REMOTE_ERROR_CODE_MAX_BYTES`), max 19 bytes framed (`REMOTE_ERROR_CODE_FRAME_MAX_BYTES`). Bounds the controller's parse scan, so a hostile 200-byte acknowledgement cannot make it walk the whole string.
- Remote-lease id: the Solana lease PDA, carried on `OperationResponse::OpenLease.remote_lease_id` as a `RemoteLeaseId`. The Solana Remote Lease App MUST emit it as the canonical base58 encoding of the 32-byte PDA pubkey (32–44 chars); the controller rejects any non-base58 or over-64-byte value (`REMOTE_LEASE_ID_MAX_BYTES`) at ack-decode. This id is **load-bearing** — it is the recipient of the Nolus→Solana funds push, not merely observability — so a non-conforming value fails closed (the lease strands at the OpenLease ack, before any funds move) rather than risk a transfer to a bad address. A conforming counterparty never trips the check; the only path to a reject is a Solana-side bug, which the light-client trust model already excludes from normal operation.

### Channel handshake version

The lease channel's **handshake** version is not the bare protocol version — it carries the paired ICS-20 transfer channel as a suffix:

```
nls-remote-lease.v1+transfer=channel-5
```

The channel is a **type**, not a string: `remote_lease::Ics20ChannelId` wraps the ordinal alone, so a non-canonical value is unrepresentable and a parse-then-render round-trip is the identity. It lives in the dep-free wire crate so the two repositories cannot drift, and it carries the grammar in both directions — `FromStr`/`Display` for the id itself, `channel_version()` to compose the handshake version (infallible, since the id is already canonical) and `Ics20ChannelId::from_channel_version` to recover it.

`channel-<n>` is canonical when it is ASCII digits only, with no leading zero unless the ordinal is exactly `0`, no sign or whitespace, and `<n>` within `u16` — the counterparty derives a program address from the ordinal at that width. That caps an id at 13 bytes (`ICS20_CHANNEL_ID_MAX_BYTES`) and a whole version string at 42 (`CHANNEL_VERSION_MAX_BYTES`); a longer input is rejected before any of it is retained, and a rejected version echoed in an error is first passed through `bounded_channel_version`.

Serde goes through the same grammar — the JSON form is the rendered string `"channel-5"`, and a non-canonical id is refused by the deserialiser. That is what makes `ExecuteMsg::OpenChannel` fail at decode rather than in the controller, so the controller carries no error variant for a malformed id at all.

**Handshake layer only.** Packets keep carrying the bare `VERSION` through the `ProtocolVersion` ZST. The two must never be mixed: a suffixed version on a packet fails the envelope deserialiser, and a bare version in the handshake fails the checks below. `VERSION` and `ProtocolVersion` are unchanged by this suffix.

### Channel state machine

The controller keeps one channel record, under one storage key, as a three-phase state machine. The handshake phases and the live channel are mutually exclusive, so a single item is what makes states like "a proposal beside an open channel" unrepresentable:

```
(absent) --OpenChannel--> Proposed --OnChanOpenInit--> InitAccepted --OpenAck--> Established{Open}
                              |                             |                          |
                              +------ CancelChannelProposal -+                    CloseChannel
                                                                                       v
                                                                              Established{Closing}
                                                                                       |
                                                                                 OnChanCloseInit
                                                                                       v
                                                                                   (absent)
```

Every transition validates, and every other combination is a typed error. The close completes at the `CloseInit` callback: ibc-go closes the local end within the `MsgChannelCloseInit` the controller emits, and no later callback arrives on the initiating side — `CloseConfirm` belongs to the passive side of a close handshake, which this controller never is (the counterparty rejects close-inits of its own), so it is rejected like `OpenConfirm`. What the record stores is the typed `Ics20ChannelId`, never the composed version: the id is canonical by construction and `channel_version()` is a pure function of it, so recomposition is byte-stable and the emitted open-init, the `OpenInit` check, and the `OpenAck` check cannot drift apart. Two bytes of state carrying the invariant beats a 42-byte string carrying the same one. The pairing is fixed once, at `Proposed`, and every later phase carries it unchanged.

Three checks defend the handshake, all against that one stored expectation:

1. On the local `OpenInit` callback, `channel.version` must equal the recorded version exactly, and the record must be in `Proposed` — anything else is `UnsolicitedChannelOpen`. The callback then **consumes** `Proposed` into `InitAccepted`, recording the chain-assigned local channel id.
2. On `OpenAck` the record must be in `InitAccepted`, and `counterparty_version` must equal the recorded version exactly. The Solana responder echoes the version it accepted verbatim (ADR 0002 §3.3), so an exact match is what proves it bound the transfer channel that was proposed rather than one of its own choosing; a mismatch is `InvalidCounterpartyVersion`.
3. Also on `OpenAck`, `channel.endpoint.channel_id` must equal the id recorded at `OpenInit`, else `LocalChannelIdMismatch` — an ack belonging to some other channel cannot complete this handshake.

**Why `InitAccepted` exists.** Consuming the proposal in the `OpenInit` callback makes that callback single-use. wasmd dispatches the `MsgChannelOpenInit` the controller emits within the same transaction that emitted it, so consumption is atomic with the emission: a second `OpenInit` against the same proposal — including one an attacker submits against the controller's port — finds nothing left to consume and is rejected. That assumption on wasmd's in-transaction dispatch is what the guarantee rests on; the unit tests simulate it by driving the callbacks in sequence.

The query surface reports the phase, so an operator can tell a proposal still awaiting its own `OpenInit` from one awaiting the counterparty's ack. Every phase — not just `Established` — also exposes the proposed pairing twice over: `ics20_channel_remote` as a first-class `"channel-<n>"` value to check the deployment against, and `version` as the exact bytes to diff against the counterparty's own logs. A misconfigured pairing is therefore visible from the moment it is proposed, rather than only once the channel opens.

## Envelope

`PacketEnvelope { lease: LeaseAddrOnWire, operation: Operation, version: ProtocolVersion }`. `deny_unknown_fields` everywhere. The lease address is wrapped in `LeaseAddrOnWire`; receivers must call `into_validated(api)` (CosmWasm) before treating it as an `Addr`.

## Operations

- `OpenLease { expected_instance_ordinal: u16, downpayment_currency, lpn_currency, asset_currency }` — the only enforced inequality is `lpn_currency != asset_currency`. `downpayment_currency == lpn_currency` and `downpayment_currency == asset_currency` are both permitted; the Solana side does not constrain those pairs. The wire-level invariant is intentionally permissive — any tighter constraint belongs in the Nolus-side caller, not the wire.
- `CloseLease {}`
- `Swap` — externally-tagged enum, one variant per input arity:
  - `One { coin_in, min_out }` — single input coin; wire shape `{"swap":{"one":{"coin_in":…,"min_out":…}}}`.
  - `Two { coin_in_1, coin_in_2, min_out }` — two input coins; wire shape `{"swap":{"two":{"coin_in_1":…,"coin_in_2":…,"min_out":…}}}`.

  All coins non-zero; each input currency distinct from `min_out`; for `Two`, the two inputs are also distinct from each other (else `DuplicateSwapInputCurrency`).

  `amount_out` on the response covers only the swapped inputs. Any coin already in the output currency is excluded from the request and — the counterparty being a passive vault that only executes the swap it is asked for — is never returned by it either; the Nolus-side caller re-adds such non-swapped coins to the decoded `amount_out` itself.
- `TransferOut { amount }` — amount non-zero. The funds-return leg (Solana vault → Nolus). **Cross-repo timeout invariant:** the Solana-side return-transfer IBC timeout MUST be shorter than the Nolus-side re-issue window (`dex::IBC_TIMEOUT`, 1 day) — otherwise a return transfer still in flight when Nolus re-issues `TransferOut` on timeout could double-credit. Enforced Solana-side per ADR 0001/0002; Nolus confirms arrival by idempotent bank-balance polling, so an early vault refund only delays, never loses, funds.

Invariants are enforced both in constructors (`new`, or `one` / `two` for `Swap`) and on the deserialiser path via `try_from` raw shadows.

## Callback

`RemoteLeaseCallback::{OperationOk(OperationResponse), OperationErr(RemoteError), OperationTimeout}`. Timeout is structurally separate from error — recovery paths differ.

`RemoteError { kind: RemoteErrorKind, message: RemoteErrorMessage }` pairs a machine-readable cause with the counterparty's prose. **A consumer that branches at all branches on `kind`, never on `message`:** the counterparty's rendered text tracks an upstream DEX API whose wording changes across minor releases, so it is not a stable key. Keeping only the prose is legitimate where the outcome is unconditional — a failed lease open refunds and terminates whatever the cause, so it retains `message` for the audit event alone.

### Error code frame

An error acknowledgement carries its kind as a leading `[<token>] ` frame ahead of the prose:

```
[min_out_unmet] ibc-solray: Swap engine min_out '41' is below the required protocol min_out '42'
```

The frame is at **byte offset 0, before** any provenance prefix the counterparty adds, for two reasons: the counterparty truncates the **tail** to fit the byte cap, so a leading token survives truncation structurally rather than by luck; and it keeps the wire crate ignorant of the counterparty's own prefix, so a second counterparty reuses the frame unchanged.

Token vocabulary — **append-only**; a published token is never renamed or repurposed:

| `RemoteErrorKind` | token | meaning |
|---|---|---|
| `MinOutUnmet` | `min_out_unmet` | the swap could not fulfil the requested `min_out` |
| `Permanent` | `permanent` | a deterministic refusal of the request; identical bytes fail again |
| `Transient` | `transient` | a stale counterparty view; a fresh emission may pass |

The controller's `ack_to_callback` parses the frame exactly once and **rejects** any acknowledgement whose code is absent, malformed, or unknown — it never coerces one to a default. Guessing would invent a meaning the counterparty never sent and then route funds with it. Both ends of this protocol deploy in lockstep, so a code that does not parse is a deployment fault to fix rather than a case to absorb; the cost is that such an acknowledgement reverts `ibc_packet_ack` and the relayer redelivers until the fault is deployed away. A **classified** failure never returns `Err` from the parse.

The frame is stripped from the retained `message`, so anything downstream that reports the reason (the `OpenFailed` terminal, its query response, the `ls-remote-lease-open-failed` event) sees the counterparty's prose alone.

## Controller surface (Nolus side)

The `remote_lease` controller exposes one `ExecuteMsg` variant per `Operation`:

- `ExecuteMsg::OpenLease { params: OpenLeaseParams, timeout: Duration }`
- `ExecuteMsg::CloseLease { params: CloseLeaseParams, timeout: Duration }`
- `ExecuteMsg::Swap { params: SwapParams, timeout: Duration }`
- `ExecuteMsg::TransferOut { params: TransferOutParams, timeout: Duration }`

Each call:

1. Authorises the sender against `Config.lease_code` — the caller must be a contract instance of the configured lease code id. Non-contract callers and contracts with a different code id collapse to a single `UnauthorisedCaller`; the controller does not distinguish them on the protocol surface.
2. Loads the channel and rejects anything other than `Open` (absent → `ChannelNotOpen`, `Closing` → `ChannelNotOperational`).
3. Wraps the operation in `PacketEnvelope { lease, operation, version }` and emits `IbcMsg::SendPacket` on the locally stored channel id.
4. Sets the packet timeout to `env.block.time + timeout` — the caller owns its own retry cadence.

Channel lifecycle is separate and protocol-admin only:

- `ExecuteMsg::OpenChannel { ics20_channel_remote: Ics20ChannelId }` — starts the handshake, proposing `ics20_channel_remote` (the **counterparty-side** ICS-20 transfer channel) in the handshake version. Requires that **no** channel record exists: a handshake already in flight gives `ProposalPending`, an established channel `ChannelAlreadyExists`. The field is typed, so a non-canonical id fails at message decode rather than here.

  **Operator ordering:** that transfer channel must already be **fully open** when this call is made — per ADR 0002 §3.3 the counterparty binds the pair while validating the handshake version and cannot revisit the binding afterwards.
- `ExecuteMsg::CancelChannelProposal()` — abandons a handshake still in flight (`Proposed` or `InitAccepted`), clearing the record so a fresh `OpenChannel` can be issued. Rejected once the channel is `Established` (`ChannelAlreadyExists`) and when nothing is pending (`NoProposalToCancel`).

  **Cancel, then reopen.** A proposal is never replaced silently — abandoning one is an explicit operator act. This is the only escape from a counterparty that never acknowledges: without it the controller would hold the proposal forever. Note that cancelling is local bookkeeping only; it emits no IBC message, so an in-flight handshake the counterparty later acks will fail its checks rather than open a channel.
- `ExecuteMsg::CloseChannel()` — begins closing a channel that is currently `Open`.

## Controller → Lease callback dispatch

On `ibc_packet_ack` and `ibc_packet_timeout` the controller decodes the original packet's `PacketEnvelope`, builds the appropriate `RemoteLeaseCallback` variant, and forwards it to the originating lease via a plain `WasmMsg::Execute` — `add_message`, not `SubMsg::reply_*`. The dispatched payload is:

```json
{"remote_lease_callback": <RemoteLeaseCallback>}
```

mapping the IBC outcomes:

- `StdAck::Success(data)` → `RemoteLeaseCallback::OperationOk(OperationResponse)` (decoded from `data`).
- `StdAck::Error(message)` → `RemoteLeaseCallback::OperationErr(RemoteError)` via `RemoteError::parse_ack`. `StdAck::Error` is a bare string, so this is the single point at which the counterparty's failure becomes typed. Rejected if the code frame is absent, malformed, or unknown, or if the prose exceeds 200 bytes once the frame is stripped — note the cap is applied **after** stripping, so an acknowledgement over-long only by its frame is accepted.
- timeout → `RemoteLeaseCallback::OperationTimeout` (unit; the original `Operation` is recoverable from the lease's own pending-state).

The lease address travels with the packet (`envelope.lease`) — the controller keeps no per-packet correlation map. The lease contract authorises the call by querying its leaser (`QueryMsg::CheckRemoteLeaseCallbackPermission { by: info.sender }`); the leaser compares the caller against its protocol-wide `Config.remote_lease_controller`, set at leaser instantiation. That address is immutable — no `ExecuteMsg` or `SudoMsg` variant updates `remote_lease_controller` — so the live-query semantic is equivalent to a pin set at lease open. The controller does not retry on the lease's behalf. See ADR 0001 §3.7 in `nolus-protocol/ibc-solray` for the atomicity model.

## Design principle

All policy lives on Nolus. Solana is a passive vault — see ADRs 0001 / 0002 in `nolus-protocol/ibc-solray`.
