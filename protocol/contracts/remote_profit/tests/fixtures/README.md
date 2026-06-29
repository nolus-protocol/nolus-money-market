# Inbound ack fixtures

Canonical byte sequences the controller must accept on `ibc_packet_ack` for the
two `StdAck` variants the Solana-side Remote Profit App emits per
**ADR 0001 §3.5**.

## Files

- `stdack_success_open_profit.bin` — `StdAck::Success(<JSON of OperationResponse::OpenProfit>)` wrapped by `to_binary()`. The inner snake-case-tagged `OperationResponse::OpenProfit` carries `remote_profit_id = "CkymGXksQYqyYZrdvFTWwFhvMNBqENvKfKQN4e7CwBxF"` — a representative **ProfitAuthority** address, the funding target the Cosmos side sends ICS-20 to (ADR 0008), the program-derived singleton authority, **not** a per-customer PDA. It is a canonical base58 encoding of a 32-byte PDA pubkey (44 chars), in the `remote_profit_id` range documented in the wire crate. Outer JSON is the cosmwasm-std `StdAck` wire shape (`{"result":"<base64>"}`).
- `stdack_error.bin` — an error ack `{"error":"ibc-solray: dex pool drained"}`. The Solana side prefixes every error-ack content with `ibc-solray: ` (ibc-solray `src/app/remote_profit/ack.rs`), so the controller lifts the full prefixed string into `RemoteErrorMessage`.

## Cross-side contract

These bytes are the Solana emitter's output, reproduced here byte-for-byte. The consumer tests assert two things: that this controller decodes each fixture into the expected `RemoteProfitCallback` (the consume path), and that re-encoding the same value through the shared `remote_profit_wire` crate reproduces the fixture (so a drift in our own serialiser is caught too). If the Solana side's emission ever diverges from these bytes, regenerate from its new output and reconcile — that divergence is the schema drift this fixture exists to surface.

## Emission path (source of truth)

The bytes are what ibc-solray's `src/app/remote_profit/ack.rs` produces:

- success: `{"result":"<base64-standard of serde_json(OperationResponse)>"}` (`ack::success`);
- error: `{"error":"ibc-solray: <message>"}` (`ack::error`, `PREFIX = "ibc-solray: "`).

The inner `OperationResponse` JSON is the shared `remote_profit_wire` shape, byte-identical across both stacks — pinned by the cross-surface test in `protocol/packages/remote_profit/tests/cross_surface.rs` and the literal-string tests in `protocol/packages/remote_profit/src/tests.rs`.

## Regenerating

Compute each, **with no trailing newline**:

```text
inner   = serde_json::to_string(&OperationResponse::OpenProfit(OpenProfitResponse { remote_profit_id }))
success = r#"{"result":"<base64-standard(inner)>"}"#     // == StdAck::Success(Binary::from(inner)).to_binary()
error   = r#"{"error":"ibc-solray: <message>"}"#          // == StdAck::error("ibc-solray: <message>").to_binary()
```

The example `remote_profit_id` is a fixed stand-in — a 32-byte PDA pubkey in canonical base58 (44 chars), not a live address. Any valid 32-byte base58 value works; if you change it, regenerate the `.bin` with the formula above.

The consumer tests live in `protocol/contracts/remote_profit/src/ibc/tests/packets.rs` under names prefixed `fixture_…`; each asserts the fixture equals the value re-encoded through the wire crate, then drives `ibc_packet_ack` over it.
