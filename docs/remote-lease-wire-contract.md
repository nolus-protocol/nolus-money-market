# Remote Lease Wire Contract

The `remote_lease` crate defines the IBC packet types exchanged between the Nolus CosmWasm controller and the Solana Remote Lease App. Both sides deserialise the same Rust types via `serde`; the canonical definition lives in `protocol/packages/remote_lease/`.

## Pinned constants

- Protocol version: `nls-remote-lease.v1` (`remote_lease::VERSION`). Encoded on every packet as the `ProtocolVersion` ZST; mismatches are rejected at deserialisation, not in business code.
- IBC port: `nls-remote-lease.<dex>` — built via `remote_lease::port_id_for`.
- Callback error payload: max 512 bytes (`OPERATION_ERR_MAX_BYTES`); enforced in the `RemoteErrorMessage` visitor before allocation.

## Envelope

`PacketEnvelope { lease: LeaseAddrOnWire, operation: Operation, version: ProtocolVersion }`. `deny_unknown_fields` everywhere. The lease address is wrapped in `LeaseAddrOnWire`; receivers must call `into_validated(api)` (CosmWasm) before treating it as an `Addr`.

## Operations

- `OpenLease { expected_instance_ordinal: u16, downpayment_currency, lpn_currency, asset_currency }` — currencies must be pairwise distinct.
- `CloseLease {}`
- `Swap { coin_in, min_out }` — both amounts non-zero, currencies distinct.
- `TransferOut { amount }` — amount non-zero.

Invariants are enforced both in constructors (`new`) and on the deserialiser path via `try_from` raw shadows.

## Callback

`RemoteLeaseCallback::{OperationOk(OperationResponse), OperationErr(RemoteErrorMessage), OperationTimeout}`. Timeout is structurally separate from error — recovery paths differ.

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

## Controller → Lease callback dispatch

On `ibc_packet_ack` and `ibc_packet_timeout` the controller decodes the original packet's `PacketEnvelope`, builds the appropriate `RemoteLeaseCallback` variant, and forwards it to the originating lease via a plain `WasmMsg::Execute` — `add_message`, not `SubMsg::reply_*`. The dispatched payload is:

```json
{"remote_lease_callback": <RemoteLeaseCallback>}
```

mapping the IBC outcomes:

- `StdAck::Success(data)` → `RemoteLeaseCallback::OperationOk(OperationResponse)` (decoded from `data`).
- `StdAck::Error(message)` → `RemoteLeaseCallback::OperationErr(RemoteErrorMessage)` (rejected if > 512 bytes).
- timeout → `RemoteLeaseCallback::OperationTimeout` (unit; the original `Operation` is recoverable from the lease's own pending-state).

The lease address travels with the packet (`envelope.lease`) — the controller keeps no per-packet correlation map. The lease contract authorises the call with `info.sender == controller_addr` (set in its `Config` at instantiate); the controller does not retry on the lease's behalf. See ADR 0001 §3.7 in `nolus-protocol/ibc-solray` for the atomicity model.

## Design principle

All policy lives on Nolus. Solana is a passive vault — see ADRs 0001 / 0002 in `nolus-protocol/ibc-solray`.
