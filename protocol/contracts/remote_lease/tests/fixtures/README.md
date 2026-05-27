# Inbound ack fixtures

Canonical byte sequences the controller must accept on `ibc_packet_ack` for the
two `StdAck` variants emitted by the Solana-side Remote Lease App per
**ADR 0001 §3.5** (`nolus-protocol/ibc-solray` at commit `179ca2382`).

## Files

- `stdack_success_open_lease.bin` — `StdAck::Success(<JSON of OperationResponse::OpenLease(remote_lease_id = "So1RayF1xtureLease1")>)` wrapped by `to_binary()`. Outer JSON is the cosmwasm-std `StdAck` wire shape (`{"result":"<base64>"}`); inner is the snake-case-tagged `OperationResponse` enum. The id is a base58 string per the `RemoteLeaseId` wire invariant.
- `stdack_error.bin` — `StdAck::Error("dex pool drained")` wrapped by `to_binary()`. Outer JSON is `{"error":"<msg>"}`.

## Status

**Placeholder.** These bytes are generated on the Nolus side by the same wire-types crate the contract consumes (`protocol/packages/remote_lease/`). They prove our own serialiser/deserialiser is stable, but cross-language drift between Solana and Nolus is NOT yet detected by this fixture.

The Solana Remote Lease App is expected to emit the same byte sequences once its PoC ships (per ADR 0001 §3.5). At that point these files are to be **regenerated from Solana's output** and the consumer test must continue to pass against the new bytes. If Solana's output diverges, that is the schema drift this fixture is designed to surface.

## Regenerating

The inner JSON shapes are pinned by literal-string tests in `protocol/packages/remote_lease/src/tests.rs`. The outer `StdAck` wrapper is defined entirely by `cosmwasm_std::StdAck::to_binary()`. To recompute:

```text
inner = serde_json::to_string(&OperationResponse::OpenLease(OpenLeaseResponse { remote_lease_id }))
outer = serde_json::to_string(&StdAck::Success(Binary::from(inner)))
     == r#"{"result":"<base64(inner)>"}"#
```

The consumer tests live in `protocol/contracts/remote_lease/src/ibc/tests.rs` under names prefixed `fixture_…`.
