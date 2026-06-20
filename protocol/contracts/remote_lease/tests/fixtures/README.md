# Inbound ack fixtures

Canonical byte sequences the controller must accept on `ibc_packet_ack` for the
two `StdAck` variants the Solana-side Remote Lease App emits per
**ADR 0001 §3.5**, pinned to `nolus-protocol/ibc-solray` main `73a8b163`.

## Files

- `stdack_success_open_lease.bin` — `StdAck::Success(<JSON of OperationResponse::OpenLease>)` wrapped by `to_binary()`. The inner snake-case-tagged `OperationResponse::OpenLease` carries `remote_lease_id = "LeaseAuthFundingTarget1"` — a representative **LeaseAuthority** address, the funding target the Cosmos side sends ICS-20 to (ibc-solray #486, ADR 0002 §3.4 step 9), **not** the Lease PDA. Outer JSON is the cosmwasm-std `StdAck` wire shape (`{"result":"<base64>"}`); the id is a base58 string per the `RemoteLeaseId` wire invariant.
- `stdack_error.bin` — an error ack `{"error":"ibc-solray: dex pool drained"}`. The Solana side prefixes every error-ack content with `ibc-solray: ` (ibc-solray `src/app/remote_lease/ack.rs`), so the controller lifts the full prefixed string into `RemoteErrorMessage`.

## Cross-side contract

These bytes are the Solana emitter's output, reproduced here byte-for-byte. The consumer tests assert two things: that this controller decodes each fixture into the expected `RemoteLeaseCallback` (the consume path), and that re-encoding the same value through the shared `remote_lease_wire` crate reproduces the fixture (so a drift in our own serialiser is caught too). If the Solana side's emission ever diverges from these bytes, regenerate from its new output and reconcile — that divergence is the schema drift this fixture exists to surface.

## Emission path (source of truth)

The bytes are what ibc-solray's `src/app/remote_lease/ack.rs` produces:

- success: `{"result":"<base64-standard of serde_json(OperationResponse)>"}` (`ack::success`);
- error: `{"error":"ibc-solray: <message>"}` (`ack::error`, `PREFIX = "ibc-solray: "`).

The inner `OperationResponse` JSON is the shared `remote_lease_wire` shape, byte-identical across both stacks — pinned by the cross-surface test in `protocol/packages/remote_lease/tests/cross_surface.rs` and the literal-string tests in `protocol/packages/remote_lease/src/tests.rs`.

## Regenerating

Compute each, **with no trailing newline**:

```text
inner   = serde_json::to_string(&OperationResponse::OpenLease(OpenLeaseResponse { remote_lease_id }))
success = r#"{"result":"<base64-standard(inner)>"}"#     // == StdAck::Success(Binary::from(inner)).to_binary()
error   = r#"{"error":"ibc-solray: <message>"}"#          // == StdAck::error("ibc-solray: <message>").to_binary()
```

The consumer tests live in `protocol/contracts/remote_lease/src/ibc/tests/packets.rs` under names prefixed `fixture_…`; each asserts the fixture equals the value re-encoded through the wire crate, then drives `ibc_packet_ack` over it.
