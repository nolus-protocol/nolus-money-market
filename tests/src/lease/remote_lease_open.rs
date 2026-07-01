//! Open-lifecycle E2E for the remote-lease lifecycle: happy path,
//! `OperationErr` auto-refund, and late-ack absorber on the `OpenFailed`
//! terminal. Targets the post-refactor query surface (`StateResponse::
//! OpenFailed`, `OngoingTrx::OpenLease`) and the
//! `wasm-remote-lease-open-failed` / `wasm-remote-lease-late-ack` events.

use crate::common::testing;
use crate::common::{
    self, USER,
    remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
};
use currencies::Lpn;
use finance::coin::Coin;
use lease::api::{
    ExecuteMsg,
    query::{StateResponse, opening::OngoingTrx as OpeningOngoingTrx},
};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback, RemoteOperationOutcome},
    response::{CloseLeaseResponse, OpenLeaseResponse, RemoteLeaseId, WireOperationResponse},
};
use sdk::cosmwasm_std::Addr;

use super::{LeaseCoin, LeaseCurrency};

const DOWNPAYMENT: LeaseCoin = LeaseCoin::new(10_000);

#[test]
fn open_lifecycle_happy_path() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let lease = super::try_init_lease(&mut test_case, DOWNPAYMENT, None);

    let state = super::state_query(&test_case, lease);

    // The synchronous `OpenLease` ack progresses the opening straight to the
    // funding leg, so the observable sub-state is `Funding`; its `receiver` is
    // the `LeaseAuthority` the ack returned, named so a future variant is not
    // silently caught by a wildcard.
    let receiver = match state {
        StateResponse::Opening { in_progress, .. } => match in_progress {
            OpeningOngoingTrx::Funding { receiver } => receiver,
            in_progress @ (OpeningOngoingTrx::RequestingOpenLease
            | OpeningOngoingTrx::OpenLease { .. }
            | OpeningOngoingTrx::BuyAsset { .. }
            | OpeningOngoingTrx::SlippageProtectionActivated
            | OpeningOngoingTrx::Unwinding) => {
                panic!("expected OngoingTrx::Funding, got {in_progress:?}")
            }
        },
        other => panic!("expected StateResponse::Opening, got {other:?}"),
    };

    assert!(
        receiver.starts_with("StubPda"),
        "expected stand-in PDA prefix, got {receiver:?}",
    );
}

#[test]
fn open_failed_on_error_refunds_and_finalises() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    let leaser = test_case.address_book.leaser().clone();
    let reason = RemoteErrorMessage::new("solana side rejected").expect("within length cap");
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::OPEN_LEASE,
        ResponseMode::Err(reason.clone()),
    );

    let customer = testing::user(USER);
    let downpayment_before = balance::<LeaseCurrency>(&test_case, &customer);

    let response = test_case
        .app
        .execute(
            customer.clone(),
            leaser.clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: currency::dto::<LeaseCurrency, _>(),
                max_ltd: None,
            },
            &[common::cwcoin::<LeaseCurrency>(DOWNPAYMENT)],
        )
        .unwrap()
        .unwrap_response();

    let event = response
        .events
        .iter()
        .find(|event| event.ty == "wasm-ls-remote-lease-open-failed")
        .expect("auto-refund must emit the open-failed event");
    let event_reason = event
        .attributes
        .iter()
        .find(|attr| attr.key == "reason")
        .map(|attr| attr.value.as_str())
        .expect("event must carry the counterparty reason");
    assert_eq!(reason.as_str(), event_reason);

    let downpayment_after = balance::<LeaseCurrency>(&test_case, &customer);
    assert_eq!(
        downpayment_before, downpayment_after,
        "downpayment must be refunded in full",
    );

    let leases = leases_of(&test_case, &leaser, &customer);
    assert!(leases.is_empty(), "leaser must show no live lease");

    let lease = lease_address_from(&response.events).expect("instantiate event carries the addr");
    let state = super::state_query(&test_case, lease);
    match state {
        StateResponse::OpenFailed {
            reason: ref state_reason,
        } => assert_eq!(reason.as_str(), state_reason.as_str()),
        other => panic!("expected StateResponse::OpenFailed, got {other:?}"),
    }
}

#[test]
fn open_failed_on_unexpected_operation_refunds_and_finalises() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    let leaser = test_case.address_book.leaser().clone();
    // Hold the lease in the pre-ack `OpenLease` state by deferring the
    // controller's callback, so the wrong-operation ack below reaches the
    // `OperationOk(other)` arm rather than the auto-synthesised happy path.
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::OPEN_LEASE,
        ResponseMode::Delayed,
    );

    let customer = testing::user(USER);
    let balance_before = balance::<LeaseCurrency>(&test_case, &customer);

    let response = test_case
        .app
        .execute(
            customer.clone(),
            leaser.clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: currency::dto::<LeaseCurrency, _>(),
                max_ltd: None,
            },
            &[common::cwcoin::<LeaseCurrency>(DOWNPAYMENT)],
        )
        .unwrap()
        .unwrap_response();

    let lease = lease_address_from(&response.events).expect("instantiate event carries the addr");

    // A `CloseLease` success against an in-flight `OpenLease` can only come
    // from a buggy or hostile counterparty. The lease must treat it as an
    // open failure — refund and move to terminal — rather than returning
    // `Err`, which would revert the controller's ack and strand the relayer.
    let unexpected = RemoteLeaseCallback {
        nonce: 0,
        outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::CloseLease(
            CloseLeaseResponse {},
        )),
    };
    let failed = test_case
        .app
        .execute(
            controller.clone(),
            lease.clone(),
            &ExecuteMsg::RemoteLeaseCallback(unexpected),
            &[],
        )
        .expect("unexpected operation must be absorbed as an open failure")
        .unwrap_response();

    let event_reason = failed
        .events
        .iter()
        .find(|event| event.ty == "wasm-ls-remote-lease-open-failed")
        .and_then(|event| event.attributes.iter().find(|attr| attr.key == "reason"))
        .map(|attr| attr.value.as_str())
        .expect("unexpected operation must emit the open-failed event with a reason");
    assert!(
        event_reason.starts_with("unexpected operation response"),
        "reason must name the unexpected variant, got {event_reason:?}",
    );

    let balance_after = balance::<LeaseCurrency>(&test_case, &customer);
    assert_eq!(
        balance_before, balance_after,
        "downpayment must be refunded in full",
    );

    let leases = leases_of(&test_case, &leaser, &customer);
    assert!(leases.is_empty(), "leaser must show no live lease");

    match super::state_query(&test_case, lease) {
        StateResponse::OpenFailed { reason } => assert!(
            reason.as_str().starts_with("unexpected operation response"),
            "terminal reason must name the unexpected variant, got {reason:?}",
        ),
        other => panic!("expected StateResponse::OpenFailed, got {other:?}"),
    }
}

#[test]
fn late_open_lease_ack_after_open_failed_is_absorbed() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let controller = test_case.address_book.remote_lease_controller().clone();
    let leaser = test_case.address_book.leaser().clone();
    let reason = RemoteErrorMessage::new("solana side rejected").expect("within length cap");
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::OPEN_LEASE,
        ResponseMode::Err(reason.clone()),
    );

    let customer = testing::user(USER);

    let response = test_case
        .app
        .execute(
            customer.clone(),
            leaser.clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: currency::dto::<LeaseCurrency, _>(),
                max_ltd: None,
            },
            &[common::cwcoin::<LeaseCurrency>(DOWNPAYMENT)],
        )
        .unwrap()
        .unwrap_response();

    let lease = lease_address_from(&response.events).expect("instantiate event carries the addr");
    let balance_before = balance::<LeaseCurrency>(&test_case, &customer);

    // Synthesise the late OK ack that ibc-go would deliver against the
    // already-terminal lease on an UNORDERED channel.
    let late_callback = RemoteLeaseCallback {
        nonce: 0,
        outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::OpenLease(
            OpenLeaseResponse {
                remote_lease_id: RemoteLeaseId::new(
                    "StubPdaLate111111111111111111111111111".to_owned(),
                )
                .expect("base58 sample"),
            },
        )),
    };
    let late = test_case
        .app
        .execute(
            controller.clone(),
            lease.clone(),
            &ExecuteMsg::RemoteLeaseCallback(late_callback),
            &[],
        )
        .expect("late ack must be absorbed by the OpenFailed terminal")
        .unwrap_response();

    let absorbed = late
        .events
        .iter()
        .any(|event| event.ty == "wasm-ls-remote-lease-late-ack");
    assert!(absorbed, "OpenFailed must emit the late-ack event");

    let balance_after = balance::<LeaseCurrency>(&test_case, &customer);
    assert_eq!(balance_before, balance_after, "absorber must be idempotent");

    match super::state_query(&test_case, lease) {
        StateResponse::OpenFailed {
            reason: state_reason,
        } => assert_eq!(reason.as_str(), state_reason.as_str()),
        other => panic!("expected StateResponse::OpenFailed, got {other:?}"),
    }
}

fn balance<C>(test_case: &super::LeaseTestCase, account: &Addr) -> Coin<C>
where
    C: currency::CurrencyDef,
{
    use platform::bank::{self, BankAccountView};
    bank::account_view(account, test_case.app.query())
        .balance::<C>()
        .expect("balance query must succeed")
}

fn leases_of(
    test_case: &super::LeaseTestCase,
    leaser: &Addr,
    customer: &Addr,
) -> std::collections::HashSet<Addr> {
    test_case
        .app
        .query()
        .query_wasm_smart(
            leaser.clone(),
            &leaser::msg::QueryMsg::Leases {
                owner: customer.clone(),
            },
        )
        .unwrap()
}

fn lease_address_from(events: &[sdk::cosmwasm_std::Event]) -> Option<Addr> {
    events
        .iter()
        .filter(|event| event.ty == "instantiate" || event.ty.starts_with("wasm-"))
        .flat_map(|event| event.attributes.iter())
        .find(|attr| attr.key == "_contract_address" || attr.key == "lease_address")
        .map(|attr| Addr::unchecked(attr.value.clone()))
}

// Pull `Lpn` into the build graph so its currency definitions are loaded by
// the tests using LeaseCurrency-side balance queries.
#[allow(dead_code)]
fn _lpn_anchor() -> currency::CurrencyDTO<currencies::Lpns> {
    currency::dto::<Lpn, _>()
}
