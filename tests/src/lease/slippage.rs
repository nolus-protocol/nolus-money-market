use currencies::{LeaseGroup, Lpn, Lpns, PaymentGroup};
use dex::MaxSlippage;
use finance::{
    coin::CoinDTO,
    fraction::Fraction,
    price::{self, Price},
};
use lease::api::{
    ExecuteMsg,
    position::ChangeCmd,
    query::{
        StateResponse,
        opened::{OngoingTrx, Status},
    },
};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback},
    swap::SwapParams,
};
use sdk::{
    cosmwasm_std::{Addr, Event},
    testing,
};

use crate::{
    common::{
        self, LEASE_ADMIN, USER,
        leaser::Instantiator as LeaserInstantiator,
        oracle as oracle_mod,
        remote_lease_controller_stub::{self as stub, ResponseMode, op_tag},
        swap,
        test_case::response::RemoteChain,
    },
    lease::{self as lease_mod, heal},
};

use super::{
    DOWNPAYMENT, LeaseCoin, LeaseCurrency, LeaseTestCase, LpnCoin, PaymentCurrency, close_policy,
};

const LEASE_AMOUNT: LeaseCoin = LeaseCoin::new(2857142857142);
const BORROWED_AMOUNT: LpnCoin = LpnCoin::new(1857142857142);

#[test]
fn full_liquidation_heal_no_rights() {
    let mut test_case = lease_mod::create_test_case::<PaymentCurrency>();

    let lease = lease_mod::open_lease(&mut test_case, DOWNPAYMENT, None);

    trigger_full_liquidation(&mut test_case, LEASE_AMOUNT, BORROWED_AMOUNT);
    simulate_min_out_not_satisfied(&mut test_case, lease.clone());

    heal::heal_no_rights(&mut test_case.app, lease.clone(), testing::user(USER));
}

#[test]
fn full_liquidation_heal_no_close() {
    let mut test_case = lease_mod::create_test_case::<PaymentCurrency>();

    let lease = lease_mod::open_lease(&mut test_case, DOWNPAYMENT, None);

    trigger_full_liquidation(&mut test_case, LEASE_AMOUNT, BORROWED_AMOUNT);
    simulate_min_out_not_satisfied(&mut test_case, lease.clone());

    // far-better price Lease = 2* Borrowed, i.e. LTV = 50%
    deliver_high_price(
        &mut test_case,
        LEASE_AMOUNT.checked_div(2).unwrap(),
        BORROWED_AMOUNT,
    );

    //heal to idle
    {
        let mut heal_response = heal::heal_ok(
            &mut test_case.app,
            lease.clone(),
            testing::user(LEASE_ADMIN),
        )
        .ignore_response();
        heal_response.expect_empty();
        assert!(matches!(
            super::state_query(&test_case, lease),
            StateResponse::Opened {
                status: Status::Idle,
                ..
            }
        ));
    }
}

#[test]
fn full_liquidation_heal_sl_close() {
    let mut test_case = lease_mod::create_test_case::<PaymentCurrency>();

    let lease = lease_mod::open_lease(&mut test_case, DOWNPAYMENT, None);

    let sl = LeaserInstantiator::THIRD_LIQ_WARN;
    close_policy::change_ok(
        &mut test_case,
        lease.clone(),
        Some(ChangeCmd::Reset),
        Some(ChangeCmd::Set(sl)),
    );

    trigger_full_liquidation(&mut test_case, LEASE_AMOUNT, BORROWED_AMOUNT);
    simulate_min_out_not_satisfied(&mut test_case, lease.clone());

    // price that triggers SL: SL% of Lease = Borrowed, i.e. LTV = SL
    deliver_high_price(&mut test_case, sl.of(LEASE_AMOUNT), BORROWED_AMOUNT);

    //heal to SL close
    {
        let controller = test_case.address_book.remote_lease_controller().clone();
        let heal_response = heal::heal_ok(
            &mut test_case.app,
            lease.clone(),
            testing::user(LEASE_ADMIN),
        );
        // The heal re-emits the sell-asset swap, held pending by the stand-in,
        // so no interchain messages accompany it.
        let _ = heal_response.unwrap_response();

        let captured = swap::captured(&test_case.app, &controller);
        assert_any_min_out(&captured);

        assert!(matches!(
            super::state_query(&test_case, lease),
            StateResponse::Opened {
                status: Status::InProgress(OngoingTrx::Close { .. }),
                ..
            }
        ));
    }
}

#[test]
fn full_liquidation_heal_full_liquidation() {
    let mut test_case = lease_mod::create_test_case::<PaymentCurrency>();

    let lease = lease_mod::open_lease(&mut test_case, DOWNPAYMENT, None);

    trigger_full_liquidation(&mut test_case, LEASE_AMOUNT, BORROWED_AMOUNT);
    simulate_min_out_not_satisfied(&mut test_case, lease.clone());

    //heal to full liquidation
    {
        let controller = test_case.address_book.remote_lease_controller().clone();
        let heal_response = heal::heal_ok(
            &mut test_case.app,
            lease.clone(),
            testing::user(LEASE_ADMIN),
        );
        // The heal re-emits the sell-asset swap, held pending by the stand-in,
        // so no interchain messages accompany it.
        let _ = heal_response.unwrap_response();

        let captured = swap::captured(&test_case.app, &controller);
        assert_min_out(&test_case, &captured, LEASE_AMOUNT);

        assert!(matches!(
            super::state_query(&test_case, lease),
            StateResponse::Opened {
                status: Status::InProgress(OngoingTrx::Liquidation { .. }),
                ..
            }
        ));
    }
}

fn trigger_full_liquidation(
    test_case: &mut LeaseTestCase,
    lease_amount: LeaseCoin,
    borrowed_amount: LpnCoin,
) {
    let controller = test_case.address_book.remote_lease_controller().clone();
    // Hold the sell-asset swap pending so `simulate_min_out_not_satisfied` can
    // reject it by hand.
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    // the base is chosen to be close to the position amount to trigger a full liquidation
    let response =
        lease_mod::deliver_new_price(test_case, lease_amount + common::coin(10), borrowed_amount);
    // The swap is emitted (a `WasmMsg`, held pending), so the price-delivery
    // response carries no interchain messages.
    let _ = response.unwrap_response();

    let captured = swap::captured(&test_case.app, &controller);
    assert_eq!(
        Into::<CoinDTO<PaymentGroup>>::into(lease_amount),
        swap::token_in(&captured),
    );
    assert_min_out(test_case, &captured, lease_amount);
}

fn simulate_min_out_not_satisfied(test_case: &mut LeaseTestCase, lease: Addr) {
    let controller = test_case.address_book.remote_lease_controller().clone();
    let reason = RemoteErrorMessage::new("min output amount not fulfilled").expect("within cap");
    // The counterparty rejects the held swap; a sell-asset anomaly drives the
    // lease into the slippage-protected state (no retry).
    let mut swap_response = test_case
        .app
        .execute(
            controller,
            lease.clone(),
            &ExecuteMsg::RemoteLeaseCallback(RemoteLeaseCallback::OperationErr(reason)),
            &[],
        )
        .expect("on error should have gone into a protected state");
    swap_response.expect_empty();
    let app_response = swap_response.unwrap_response();
    app_response.assert_event(
        &Event::new("wasm-ls-slippage-anomaly")
            .add_attribute("lease", lease.clone().to_string())
            .add_attribute(
                "max_slippage",
                LeaserInstantiator::MAX_SLIPPAGE.display_primitive(),
            ),
    );
    assert!(matches!(
        super::state_query(test_case, lease),
        StateResponse::Opened {
            status: Status::SlippageProtectionActivated,
            ..
        }
    ));
}

fn deliver_high_price(
    test_case: &mut LeaseTestCase,
    lease_amount: LeaseCoin,
    borrowed_amount: LpnCoin,
) {
    let mut response = lease_mod::deliver_new_price(test_case, lease_amount, borrowed_amount);
    response.expect_empty();
    let app_resp = response.unwrap_response();
    assert_eq!(
        app_resp
            .events
            .iter()
            .find(|event| event.ty == "wasm-pricealarm-delivery"),
        None,
        "{:?}",
        app_resp.events
    );
}

fn assert_min_out(
    test_case: &LeaseTestCase,
    params: &SwapParams<PaymentGroup, PaymentGroup>,
    lease_amount: LeaseCoin,
) {
    let price: Price<_, _> = oracle_mod::fetch_price::<LeaseCurrency, LeaseGroup, Lpn, Lpns>(
        test_case.app.query(),
        test_case.address_book.oracle().clone(),
    )
    .unwrap()
    .try_into()
    .unwrap();

    let position_in_lpn = price::total(lease_amount, price).unwrap();
    assert_eq!(
        MaxSlippage::unchecked(LeaserInstantiator::MAX_SLIPPAGE).min_out(position_in_lpn),
        common::coin(swap::min_out(params))
    );
}

fn assert_any_min_out(params: &SwapParams<PaymentGroup, PaymentGroup>) {
    assert_eq!(LeaseCoin::new(1), common::coin(swap::min_out(params)));
}
