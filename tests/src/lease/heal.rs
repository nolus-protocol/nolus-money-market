use lease::{
    api::{
        ExecuteMsg,
        query::{
            StateResponse,
            opened::{OngoingTrx, RepayTrx, Status},
        },
    },
    error::ContractError,
};
use remote_lease::{
    callback::RemoteLeaseCallback,
    response::{TransferOutResponse, WireOperationResponse},
};
use sdk::{
    cosmwasm_std::{Addr, Event, StdResult},
    cw_multi_test::AppResponse,
    testing,
};

use crate::{
    common::{
        self, USER,
        remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
        test_case::{
            app::App,
            response::{RemoteChain, ResponseWithInterChainMsgs},
        },
    },
    lease::{LeaseCoin, LeaseCurrency, LpnCoin, LpnCurrency, repay},
};

const REPAY_SWAP_EVENT: &str = "wasm-ls-repay-swap";

#[test]
fn active_state() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(10_000);
    let lease = super::open_lease(&mut test_case, downpayment, None);

    let query_result = super::state_query(&test_case, lease.clone());
    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, super::create_payment_coin(0));
    assert_eq!(query_result, expected_result);

    let unutilized_amount: LpnCoin = common::coin(100);

    test_case.send_funds_from_admin(lease.clone(), &[common::cwcoin(unutilized_amount)]);
    heal_ok(&mut test_case.app, lease.clone(), testing::user(USER))
        .ignore_response()
        .expect_empty();
    assert!(
        platform::bank::balance::<LpnCurrency>(&lease, test_case.app.query())
            .unwrap()
            .is_zero()
    );

    let query_result = super::state_query(&test_case, lease.clone());
    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, unutilized_amount);
    assert_eq!(query_result, expected_result);

    heal_no_inconsistency(&mut test_case.app, lease, testing::user(USER));
}

/// A bad acknowledgment of the repay swap is absorbed without mutating
/// state, and `Heal` re-emits the in-flight leg so the repay completes.
///
/// The repay swap rides the merged remote-lease controller, so this is the
/// controller-path analogue of the old ICA swap-error retry. It mirrors the
/// opening-leg driver
/// `remote_lease_swap::wrong_variant_callback_absorbed_then_heal_recovers`
/// on the repay leg (`ls-repay-swap` instead of `ls-open-swap`).
#[test]
fn swap_on_repay() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(10_000);
    let lease = super::open_lease(&mut test_case, downpayment, None);
    let controller = test_case.address_book.remote_lease_controller().clone();

    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, super::create_payment_coin(0));
    assert_eq!(super::state_query(&test_case, lease.clone()), expected_result);

    let payment = super::create_payment_coin(1_000);
    test_case.send_funds_from_admin(testing::user(USER), &[common::cwcoin(payment)]);

    // Hold the repay swap in flight, then feed it a decodable-but-wrong
    // success payload. The lease absorbs it without advancing.
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );
    let _repay = repay::send_repay(&mut test_case, lease.clone(), payment);
    repay::consume_repay_swap_input(&mut test_case, &lease, payment);
    assert_repay_swap_in_flight(&test_case, &lease);

    let wrong_variant = RemoteLeaseCallback::OperationOk(WireOperationResponse::TransferOut(
        TransferOutResponse {},
    ));
    let absorbed = stub::inject_callback(&mut test_case.app, &controller, &lease, wrong_variant);
    expect_attribute(
        &absorbed.events,
        REPAY_SWAP_EVENT,
        "absorbed",
        "unexpected-response-variant",
    );
    assert_repay_swap_in_flight(&test_case, &lease);

    // `Heal` re-emits the in-flight swap; with the stand-in back to `Ok` the
    // re-emit acks inline and the drain transfers the proceeds home.
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Ok,
    );
    let healed = super::heal(&mut test_case, lease.clone());
    expect_attribute(&healed.events, REPAY_SWAP_EVENT, "heal", "re-emit");

    let proceeds = repay::proceeds_recorded(&test_case, &lease);
    repay::deposit_lpn_proceeds(&mut test_case, &lease, proceeds);
    let _arrival = repay::deliver_funds_arrival_alarm(&mut test_case, lease.clone());

    let expected_result = super::expected_newly_opened_state(&test_case, downpayment, payment);
    assert_eq!(super::state_query(&test_case, lease.clone()), expected_result);

    heal_no_inconsistency(&mut test_case.app, lease, testing::user(USER));
}

#[track_caller]
fn assert_repay_swap_in_flight(test_case: &super::LeaseTestCase, lease: &Addr) {
    match super::state_query(test_case, lease.clone()) {
        StateResponse::Opened {
            status: Status::InProgress(OngoingTrx::Repayment { in_progress, .. }),
            ..
        } => assert_eq!(RepayTrx::Swap, in_progress),
        other => panic!("expected the repay swap in flight, got {other:?}"),
    }
}

fn expect_attribute(events: &[Event], event_type: &str, key: &str, value: &str) {
    assert!(
        events.iter().any(|event| {
            event.ty == event_type
                && event
                    .attributes
                    .iter()
                    .any(|attr| attr.key == key && attr.value == value)
        }),
        "expected event `{event_type}` with `{key} = {value}`, got {events:?}",
    );
}

pub(super) fn heal_no_inconsistency(app: &mut App, lease: Addr, caller: Addr) {
    let err = try_heal(app, lease, caller).unwrap_err();
    assert!(matches!(
        err.downcast_ref::<ContractError>().unwrap(),
        &ContractError::InconsistencyNotDetected()
    ));
}

pub(super) fn heal_no_rights(app: &mut App, lease: Addr, caller: Addr) {
    let err = try_heal(app, lease, caller).unwrap_err();
    assert!(matches!(
        err.downcast_ref::<ContractError>().unwrap(),
        &ContractError::Unauthorized(access_control::error::Error::Unauthorized {})
    ));
}

// pub(super) fn heal_unsupported(app: &mut App, lease: Addr) {
//     let err = try_heal(app, lease).unwrap_err();
//     let heal_err = err.downcast_ref::<ContractError>();
//     assert_eq!(
//         Some(&ContractError::unsupported_operation("heal")),
//         heal_err
//     );
// }

pub(super) fn heal_ok(
    app: &mut App,
    lease: Addr,
    caller: Addr,
) -> ResponseWithInterChainMsgs<'_, AppResponse> {
    try_heal(app, lease, caller).unwrap()
}

fn try_heal(
    app: &mut App,
    lease: Addr,
    caller: Addr,
) -> StdResult<ResponseWithInterChainMsgs<'_, AppResponse>> {
    app.execute(caller, lease, &ExecuteMsg::Heal(), &[])
}
