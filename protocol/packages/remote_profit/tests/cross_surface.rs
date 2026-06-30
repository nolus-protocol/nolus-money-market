//! Cross-surface byte-equivalence test (GH #626 acceptance criterion).
//!
//! For every wire-bound type the Nolus controller emits, this test:
//!   1. Constructs the typed value (using `CurrencyDTO<PaymentGroup>` /
//!      `CoinDTO<PaymentGroup>`).
//!   2. Serialises it via `serde_json`.
//!   3. Deserialises the resulting JSON into the equivalent
//!      `remote_profit_wire::*` (stringly-typed) type.
//!   4. Re-serialises the wire value.
//!   5. Asserts byte-for-byte equality between the typed and wire JSON.
//!
//! This pins the contract that the wire-only crate, consumed from outside the
//! monorepo, can losslessly carry every packet the typed controller produces.

use currencies::testing::{PaymentC1, PaymentC2, PaymentC3};
use finance::coin::Coin;

use remote_profit::{
    callback::{RemoteErrorMessage, RemoteOperationOutcome, RemoteProfitCallback},
    envelope::PacketEnvelope,
    msg::{CloseProfitParams, OpenProfitParams, Operation, SwapParams, TransferOutParams},
    response::{
        CloseProfitResponse, OpenProfitResponse, OperationResponse, RemoteProfitId, SwapResponse,
        TransferOutResponse,
    },
    version::ProtocolVersion,
};

use remote_profit_wire::{
    callback::{RemoteOperationOutcome as WireOutcome, RemoteProfitCallback as WireCallback},
    envelope::PacketEnvelope as WireEnvelope,
    msg::Operation as WireOperation,
    nolus_receiver::NolusReceiver,
    response::OperationResponse as WireResponse,
};

#[test]
fn operation_open_profit_byte_identical() {
    assert_cross_surface_eq::<Operation, WireOperation>(&Operation::OpenProfit(open_profit()));
}

#[test]
fn operation_close_profit_byte_identical() {
    assert_cross_surface_eq::<Operation, WireOperation>(&Operation::CloseProfit(
        CloseProfitParams {},
    ));
}

#[test]
fn operation_swap_byte_identical() {
    assert_cross_surface_eq::<Operation, WireOperation>(&Operation::Swap(swap()));
}

#[test]
fn operation_transfer_out_byte_identical() {
    assert_cross_surface_eq::<Operation, WireOperation>(&Operation::TransferOut(transfer_out()));
}

#[test]
fn response_open_profit_byte_identical() {
    let typed = OperationResponse::OpenProfit(OpenProfitResponse {
        remote_profit_id: RemoteProfitId::new("So1RayProfit").expect("base58 profit id"),
    });
    assert_cross_surface_eq::<OperationResponse, WireResponse>(&typed);
}

#[test]
fn response_close_profit_byte_identical() {
    let typed = OperationResponse::CloseProfit(CloseProfitResponse {});
    assert_cross_surface_eq::<OperationResponse, WireResponse>(&typed);
}

#[test]
fn response_swap_byte_identical() {
    let typed = OperationResponse::Swap(SwapResponse {
        amount_out: Coin::<PaymentC2>::new(42).into(),
    });
    assert_cross_surface_eq::<OperationResponse, WireResponse>(&typed);
}

#[test]
fn response_transfer_out_byte_identical() {
    let typed = OperationResponse::TransferOut(TransferOutResponse {});
    assert_cross_surface_eq::<OperationResponse, WireResponse>(&typed);
}

#[test]
fn callback_operation_ok_byte_identical() {
    let typed =
        RemoteOperationOutcome::OperationOk(WireResponse::CloseProfit(CloseProfitResponse {}));
    assert_cross_surface_eq::<RemoteOperationOutcome, WireOutcome>(&typed);
}

#[test]
fn callback_operation_err_byte_identical() {
    let typed = RemoteOperationOutcome::OperationErr(
        RemoteErrorMessage::new("dex pool drained").expect("short message"),
    );
    assert_cross_surface_eq::<RemoteOperationOutcome, WireOutcome>(&typed);
}

#[test]
fn callback_operation_timeout_byte_identical() {
    let typed = RemoteOperationOutcome::OperationTimeout;
    assert_cross_surface_eq::<RemoteOperationOutcome, WireOutcome>(&typed);
}

// AC (#636): a NON-ZERO nonce on the envelope crosses the typed→JSON→wire→JSON
// surface byte-identically, so field-order drift between the typed and wire
// `PacketEnvelope` (nonce must be the last field, after `version`) is caught.
#[test]
fn packet_envelope_byte_identical() {
    let typed = PacketEnvelope {
        operation: Operation::Swap(swap()),
        version: ProtocolVersion,
        nonce: 7,
    };
    assert_cross_surface_eq::<PacketEnvelope, WireEnvelope>(&typed);
}

// AC (#636): a NON-ZERO nonce on the callback crosses the typed→wire surface
// byte-identically, so the `{ nonce, outcome }` field order stays in lockstep
// between the typed and wire `RemoteProfitCallback`.
#[test]
fn callback_carries_nonce_byte_identical() {
    let typed = RemoteProfitCallback {
        nonce: 7,
        outcome: RemoteOperationOutcome::OperationOk(WireResponse::CloseProfit(
            CloseProfitResponse {},
        )),
    };
    assert_cross_surface_eq::<RemoteProfitCallback, WireCallback>(&typed);
}

fn assert_cross_surface_eq<T, W>(typed: &T)
where
    T: serde::Serialize,
    W: serde::Serialize + serde::de::DeserializeOwned,
{
    let typed_json = serde_json::to_string(typed).expect("typed serialization");
    let wire: W =
        serde_json::from_str(&typed_json).expect("wire crate must accept typed-emitted JSON");
    let wire_json = serde_json::to_string(&wire).expect("wire serialization");
    assert_eq!(
        typed_json, wire_json,
        "wire round-trip must be byte-identical"
    );
}

fn open_profit() -> OpenProfitParams {
    OpenProfitParams::new(
        7,
        NolusReceiver::new("nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu")
            .expect("sample address is a valid bech32 Nolus addr"),
    )
}

fn swap() -> SwapParams {
    SwapParams::new(
        Coin::<PaymentC1>::new(1000).into(),
        Coin::<PaymentC2>::new(42).into(),
    )
    .expect("distinct non-zero amounts")
}

fn transfer_out() -> TransferOutParams {
    TransferOutParams::new(Coin::<PaymentC3>::new(1000).into()).expect("non-zero amount")
}
