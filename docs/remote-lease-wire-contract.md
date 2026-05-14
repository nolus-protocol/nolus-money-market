# Remote Lease Wire Contract

The `remote_lease` crate defines the IBC packet types exchanged between the Nolus CosmWasm controller and the Solana Remote Lease App. Both sides deserialise the same Rust types via `serde`; the canonical definition lives in `protocol/packages/remote_lease/`.

## Pinned constants

- Protocol version: `nls-remote-lease.v1` (`remote_lease::VERSION`). Encoded on every packet as the `ProtocolVersion` ZST; mismatches are rejected at deserialisation, not in business code.
- IBC port: `nls-remote-lease.<dex>` — built via `remote_lease::port_id_for`.
- Callback error payload: max 512 bytes (`OPERATION_ERR_MAX_BYTES`); enforced in the `RemoteErrorMessage` visitor before allocation.

## Envelope

`PacketEnvelope { lease: LeaseAddrOnWire, operation: LeaseOperationsMsg, version: ProtocolVersion }`. `deny_unknown_fields` everywhere. The lease address is wrapped in `LeaseAddrOnWire`; receivers must call `into_validated(api)` (CosmWasm) before treating it as an `Addr`.

## Operations

- `OpenLease { expected_instance_ordinal: u16, downpayment_currency, lpn_currency, asset_currency }` — currencies must be pairwise distinct.
- `CloseLease {}`
- `Swap { coin_in, min_out }` — both amounts non-zero, currencies distinct.
- `TransferOut { amount }` — amount non-zero.

Invariants are enforced both in constructors (`new`) and on the deserialiser path via `try_from` raw shadows.

## Callback

`RemoteLeaseCallback::{OperationOk(OperationResponse), OperationErr(RemoteErrorMessage), OperationTimeout}`. Timeout is structurally separate from error — recovery paths differ.

## Design principle

All policy lives on Nolus. Solana is a passive vault — see ADRs 0001 / 0002 in `nolus-protocol/ibc-solray`.
