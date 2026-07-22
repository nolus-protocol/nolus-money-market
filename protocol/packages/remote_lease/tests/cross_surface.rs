//! Cross-surface byte-equivalence test (GH #626 acceptance criterion).
//!
//! For every wire-bound type the Nolus controller emits, this test:
//!   1. Constructs the typed value (using `CurrencyDTO<PaymentGroup>` /
//!      `CoinDTO<PaymentGroup>`).
//!   2. Serialises it via `serde_json`.
//!   3. Deserialises the resulting JSON into the equivalent
//!      `remote_lease_wire::*` (stringly-typed) type.
//!   4. Re-serialises the wire value.
//!   5. Asserts byte-for-byte equality between the typed and wire JSON.
//!
//! This pins the contract that the wire-only crate, consumed from outside the
//! monorepo, can losslessly carry every packet the typed controller produces.

use currencies::{
    PaymentGroup,
    testing::{PaymentC1, PaymentC2, PaymentC3},
};
use finance::coin::Coin;

use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback},
    envelope::{LeaseAddrOnWire, PacketEnvelope},
    msg::{CloseLeaseParams, OpenLeaseParams, Operation, TransferOutParams},
    response::{
        CloseLeaseResponse, OpenLeaseResponse, OperationResponse, RemoteLeaseId, SwapResponse,
        TransferOutResponse,
    },
    swap::SwapParams,
    version::ProtocolVersion,
};

use remote_lease_wire::{
    callback::RemoteLeaseCallback as WireCallback, envelope::PacketEnvelope as WireEnvelope,
    msg::Operation as WireOperation, response::OperationResponse as WireResponse,
};

type OperationP = Operation<PaymentGroup, PaymentGroup, PaymentGroup>;
type SwapParamsP = SwapParams<PaymentGroup, PaymentGroup>;
type OpenLeaseParamsP = OpenLeaseParams<PaymentGroup, PaymentGroup, PaymentGroup>;
type TransferOutParamsP = TransferOutParams<PaymentGroup>;
type PacketEnvelopeP = PacketEnvelope<PaymentGroup, PaymentGroup, PaymentGroup>;

#[test]
fn operation_open_lease_byte_identical() {
    assert_cross_surface_eq::<OperationP, WireOperation>(&OperationP::OpenLease(open_lease()));
}

#[test]
fn operation_close_lease_byte_identical() {
    assert_cross_surface_eq::<OperationP, WireOperation>(&OperationP::CloseLease(
        CloseLeaseParams {},
    ));
}

#[test]
fn operation_swap_byte_identical() {
    assert_cross_surface_eq::<OperationP, WireOperation>(&OperationP::Swap(swap()));
}

#[test]
fn operation_swap_two_byte_identical() {
    let params = SwapParamsP::two(
        Coin::<PaymentC1>::new(1000).into(),
        Coin::<PaymentC3>::new(500).into(),
        Coin::<PaymentC2>::new(42).into(),
    )
    .expect("three distinct non-zero amounts");
    assert_cross_surface_eq::<OperationP, WireOperation>(&OperationP::Swap(params));
}

#[test]
fn operation_transfer_out_byte_identical() {
    assert_cross_surface_eq::<OperationP, WireOperation>(&OperationP::TransferOut(transfer_out()));
}

#[test]
fn response_open_lease_byte_identical() {
    let typed = OperationResponse::OpenLease(OpenLeaseResponse {
        remote_lease_id: RemoteLeaseId::new("So1RayLease1").expect("base58 lease id"),
    });
    assert_cross_surface_eq::<OperationResponse<PaymentGroup>, WireResponse>(&typed);
}

#[test]
fn response_close_lease_byte_identical() {
    let typed = OperationResponse::CloseLease(CloseLeaseResponse {});
    assert_cross_surface_eq::<OperationResponse<PaymentGroup>, WireResponse>(&typed);
}

#[test]
fn response_swap_byte_identical() {
    let typed = OperationResponse::Swap(SwapResponse {
        amount_out: Coin::<PaymentC2>::new(42).into(),
    });
    assert_cross_surface_eq::<OperationResponse<PaymentGroup>, WireResponse>(&typed);
}

#[test]
fn response_transfer_out_byte_identical() {
    let typed = OperationResponse::TransferOut(TransferOutResponse {});
    assert_cross_surface_eq::<OperationResponse<PaymentGroup>, WireResponse>(&typed);
}

#[test]
fn callback_operation_ok_byte_identical() {
    let typed =
        RemoteLeaseCallback::OperationOk(OperationResponse::CloseLease(CloseLeaseResponse {}));
    assert_cross_surface_eq::<RemoteLeaseCallback<PaymentGroup>, WireCallback>(&typed);
}

#[test]
fn callback_operation_err_byte_identical() {
    let typed = RemoteLeaseCallback::OperationErr(
        RemoteErrorMessage::new("dex pool drained").expect("short message"),
    );
    assert_cross_surface_eq::<RemoteLeaseCallback<PaymentGroup>, WireCallback>(&typed);
}

#[test]
fn callback_operation_timeout_byte_identical() {
    let typed = RemoteLeaseCallback::OperationTimeout;
    assert_cross_surface_eq::<RemoteLeaseCallback<PaymentGroup>, WireCallback>(&typed);
}

#[test]
fn packet_envelope_byte_identical() {
    let typed = PacketEnvelopeP {
        lease: LeaseAddrOnWire::new("nolus1leaseaddr"),
        operation: OperationP::Swap(swap()),
        version: ProtocolVersion,
    };
    assert_cross_surface_eq::<PacketEnvelopeP, WireEnvelope>(&typed);
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

fn open_lease() -> OpenLeaseParamsP {
    OpenLeaseParams::new(
        7,
        currency::dto::<PaymentC1, PaymentGroup>(),
        currency::dto::<PaymentC2, PaymentGroup>(),
        currency::dto::<PaymentC3, PaymentGroup>(),
    )
    .expect("three distinct currencies")
}

fn swap() -> SwapParamsP {
    SwapParams::one(
        Coin::<PaymentC1>::new(1000).into(),
        Coin::<PaymentC2>::new(42).into(),
    )
    .expect("distinct non-zero amounts")
}

fn transfer_out() -> TransferOutParamsP {
    TransferOutParams::new(Coin::<PaymentC3>::new(1000).into()).expect("non-zero amount")
}
