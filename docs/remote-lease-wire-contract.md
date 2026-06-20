# Remote Lease Wire Contract

The `remote_lease` crate defines the IBC packet types exchanged between the Nolus CosmWasm controller and the Solana Remote Lease App. Both sides deserialise the same Rust types via `serde`; the canonical definition lives in `protocol/packages/remote_lease/`.

## Pinned constants

- Protocol version: `nls-remote-lease.v1` (`remote_lease::VERSION`). Encoded on every packet as the `ProtocolVersion` ZST; mismatches are rejected at deserialisation, not in business code.
- Channel handshake version: `nls-remote-lease.v1+transfer=channel-<N>` — the protocol version extended with the Solana-side ICS-20 transfer channel paired with the lease channel (ADR-0002 §3.3, ibc-solray#322/#388). The controller composes it from its `transfer_channel` config at `MsgChannelOpenInit`, requires it on every handshake callback, and verifies the counterparty's echoed version on `OpenAck`; the responder validates the suffix and persists the binding. `channel-<N>` is canonical: `channel-` prefix, decimal ordinal, no sign or leading zeros, within `u16` range. The suffix exists at the handshake layer only — the per-packet `ProtocolVersion` pin stays bare.
- IBC port: `nls-remote-lease.<dex>` — built via `remote_lease::port_id_for`.
- Callback error payload: max 512 bytes (`OPERATION_ERR_MAX_BYTES`); enforced in the `RemoteErrorMessage` visitor before allocation.
- Remote-lease id: the lease's **`LeaseAuthority`** PDA — the address the Nolus side funds via the ICS-20 push (ibc-solray#486, ADR-0002 §3.4 step 9), not the lease state PDA — carried on `OperationResponse::OpenLease.remote_lease_id` as a `RemoteLeaseId`. The Solana Remote Lease App MUST emit it as the canonical base58 encoding of the 32-byte PDA pubkey (32–44 chars); the controller rejects any non-base58 or over-64-byte value (`REMOTE_LEASE_ID_MAX_BYTES`) at ack-decode. This id is **load-bearing** — it is the recipient of the Nolus→Solana funds push, not merely observability — so a non-conforming value fails closed (the lease strands at the OpenLease ack, before any funds move) rather than risk a transfer to a bad address. A conforming counterparty never trips the check; the only path to a reject is a Solana-side bug, which the light-client trust model already excludes from normal operation.

## Envelope

`PacketEnvelope { lease: LeaseAddrOnWire, operation: Operation, version: ProtocolVersion, nonce: u64 }`. `deny_unknown_fields` everywhere. The lease address is wrapped in `LeaseAddrOnWire`; receivers must call `into_validated(api)` (CosmWasm) before treating it as an `Addr`. `nonce` is the last field and `#[serde(default)]` — the lease's per-emission identifier, optional-at-decode so it requires no channel-version bump (see "Correlation nonce (#636)").

## Operations

- `OpenLease { expected_instance_ordinal: u16, downpayment_currency, lpn_currency, asset_currency }` — the only enforced inequality is `lpn_currency != asset_currency`. `downpayment_currency == lpn_currency` and `downpayment_currency == asset_currency` are both permitted; the Solana side does not constrain those pairs. The wire-level invariant is intentionally permissive — any tighter constraint belongs in the Nolus-side caller, not the wire.
- `CloseLease {}`
- `Swap { coin_in, min_out }` — both amounts non-zero, currencies distinct.
- `TransferOut { amount }` — amount non-zero.

Invariants are enforced both in constructors (`new`) and on the deserialiser path via `try_from` raw shadows.

## Callback

`RemoteLeaseCallback { nonce: u64, outcome: RemoteOperationOutcome }`, where `RemoteOperationOutcome::{OperationOk(OperationResponse), OperationErr(RemoteErrorMessage), OperationTimeout}`. Timeout is structurally separate from error — recovery paths differ. The `nonce` carries the per-emission identifier back to the lease (see "Correlation nonce (#636)").

## Controller surface (Nolus side)

The `remote_lease` controller exposes one `ExecuteMsg` variant per `Operation`:

- `ExecuteMsg::OpenLease { params: OpenLeaseParams, timeout: Duration }`
- `ExecuteMsg::CloseLease { params: CloseLeaseParams, timeout: Duration }`
- `ExecuteMsg::Swap { params: SwapParams, timeout: Duration }`
- `ExecuteMsg::TransferOut { params: TransferOutParams, timeout: Duration }`

Each call:

1. Authorises the sender against `Config.lease_code` — the caller must be a contract instance of the configured lease code id. Non-contract callers and contracts with a different code id collapse to a single `UnauthorisedCaller`; the controller does not distinguish them on the protocol surface.
2. Loads the channel and rejects anything other than `Open` (absent → `ChannelNotOpen`, `Closing` → `ChannelNotOperational`).
3. Wraps the operation in `PacketEnvelope { lease, operation, version, nonce }` and emits `IbcMsg::SendPacket` on the locally stored channel id.
4. Sets the packet timeout to `env.block.time + timeout` — the caller owns its own retry cadence.

## Controller → Lease callback dispatch

On `ibc_packet_ack` and `ibc_packet_timeout` the controller decodes the original packet's `PacketEnvelope`, reads the `nonce` back from its own light-client-committed outbound packet (never from the counterparty's reply), builds the appropriate `RemoteOperationOutcome` variant, and forwards both as `RemoteLeaseCallback { nonce, outcome }` to the originating lease via a plain `WasmMsg::Execute` — `add_message`, not `SubMsg::reply_*`. The dispatched payload is:

```json
{"remote_lease_callback": {"nonce": N, "outcome": <RemoteOperationOutcome>}}
```

mapping the IBC outcomes (the `nonce` is forwarded from the original packet's envelope in every case):

- `StdAck::Success(data)` → `RemoteOperationOutcome::OperationOk(OperationResponse)` — decoded from `data` as the **wire shape only** (#637): the controller validates that the payload is a well-formed response, while currency-registry validation belongs to the addressee lease, which absorbs content failures so the ack commits.
- `StdAck::Error(message)` → `RemoteOperationOutcome::OperationErr(RemoteErrorMessage)` (rejected if > 512 bytes).
- timeout → `RemoteOperationOutcome::OperationTimeout` (unit; the original `Operation` is recoverable from the lease's own pending-state).

The lease address travels with the packet (`envelope.lease`) — the controller keeps no per-packet correlation map. The per-emission `nonce` also rides the envelope, so the controller still stores no correlation state of its own; it simply reads the nonce back from its committed outbound packet and forwards it, giving the lease emission-level correlation (so a duplicate, stale, or heal-superseded callback is rejected at the lease — see "Correlation nonce (#636)"). The lease contract authorises the call by querying its leaser (`QueryMsg::CheckRemoteLeaseCallbackPermission { by: info.sender }`); the leaser compares the caller against its protocol-wide `Config.remote_lease_controller`, set at leaser instantiation. That address is immutable — no `ExecuteMsg` or `SudoMsg` variant updates `remote_lease_controller` — so the live-query semantic is equivalent to a pin set at lease open. The controller does not retry on the lease's behalf. See ADR 0001 §3.7 in `nolus-protocol/ibc-solray` for the atomicity model.

## Correlation nonce (#636)

`PacketEnvelope.nonce` is a per-emission `u64` set by the lease — the lease's identifier for *this* emission of an operation. The controller never invents or stores it: on `ibc_packet_ack` / `ibc_packet_timeout` it reads the nonce back from its own light-client-committed outbound packet (**never** from the counterparty's reply) and returns it in `RemoteLeaseCallback { nonce, outcome }`. The lease's dex `RemoteSwap` node records the in-flight leg's nonce and absorbs — via a dedicated `nonce-mismatch` event — any callback whose nonce differs, collapsing duplicate, out-of-order, and heal-re-emission-race callbacks into one rejected class and making the operator `Heal()` idempotent (a stale late ack from a superseded emission lands on a nonce that no longer matches). Non-swap operations (`OpenLease` / `CloseLease` / `TransferOut`) ride a zero nonce for now. The field is `#[serde(default)]` and last in the envelope, so a counterparty that omits it decodes as nonce `0` — no channel-version bump.

## Design principle

All policy lives on Nolus. Solana is a passive vault — see ADRs 0001 / 0002 in `nolus-protocol/ibc-solray`.
