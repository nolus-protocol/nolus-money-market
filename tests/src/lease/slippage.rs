use currencies::{LeaseGroup, Lpn, Lpns, PaymentGroup};
use finance::{
    coin::CoinDTO,
    fraction::Fraction,
    percent::Percent,
    price::{self, Price},
};
use lease::api::query::{
    StateResponse,
    opened::{OngoingTrx, Status},
};
use sdk::{
    cosmwasm_std::{Addr, Event},
    testing,
};
use swap::testing::SwapRequest;

use crate::{
    common::{
        self, LEASE_ADMIN, USER,
        leaser::Instantiator as LeaserInstantiator,
        oracle as oracle_mod,
        test_case::{TestCase, response::RemoteChain},
    },
    lease::{self as lease_mod, heal},
};

use super::{DOWNPAYMENT, LeaseCoin, LeaseCurrency, LeaseTestCase, LpnCoin, PaymentCurrency};

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

    deliver_high_price(&mut test_case, LEASE_AMOUNT, BORROWED_AMOUNT);

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
fn full_liquidation_heal_full_liquidation() {
    let mut test_case = lease_mod::create_test_case::<PaymentCurrency>();

    let lease = lease_mod::open_lease(&mut test_case, DOWNPAYMENT, None);

    trigger_full_liquidation(&mut test_case, LEASE_AMOUNT, BORROWED_AMOUNT);
    simulate_min_out_not_satisfied(&mut test_case, lease.clone());

    //heal to full liquidation
    {
        let heal_response = heal::heal_ok(
            &mut test_case.app,
            lease.clone(),
            testing::user(LEASE_ADMIN),
        );

        let requests: Vec<SwapRequest<PaymentGroup, PaymentGroup>> = common::swap::expect_swap(
            heal_response,
            TestCase::DEX_CONNECTION_ID,
            TestCase::LEASE_ICA_ID,
            |_| {},
        );
        assert_min_out(&test_case, &requests, LEASE_AMOUNT);

        assert!(matches!(
            super::state_query(&test_case, lease),
            StateResponse::Opened {
                status: Status::InProgress(OngoingTrx::Liquidation { .. }),
                ..
            }
        ));
    }

    // assert_min_out(&requests, lease_amount, price);

    //     test_swap::expect_swap(
    //         swap_response_retry,
    //         TestCase::DEX_CONNECTION_ID,
    //         TestCase::LEASE_ICA_ID,
    //         |_| {},
    //     );

    // let mut response: ResponseWithInterChainMsgs<'_, ()> = common::swap::do_swap(
    //     &mut test_case.app,
    //     lease_addr.clone(),
    //     ica_addr.clone(),
    //     requests.into_iter(),
    //     |amount, _, _| {
    //         assert_eq!(amount, lease_amount);

    //         liq_outcome
    //     },
    // )
    // .ignore_response();
}

fn trigger_full_liquidation(
    test_case: &mut LeaseTestCase,
    lease_amount: LeaseCoin,
    borrowed_amount: LpnCoin,
) {
    // the base is chosen to be close to the position amount to trigger a full liquidation
    let response =
        lease_mod::deliver_new_price(test_case, lease_amount + 10.into(), borrowed_amount);
    let requests: Vec<SwapRequest<PaymentGroup, PaymentGroup>> = common::swap::expect_swap(
        response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
        |_| {},
    );
    //the `expect_swap` postconditions guarantee there is at least one item
    assert_eq!(
        Into::<CoinDTO<PaymentGroup>>::into(lease_amount),
        requests[0].token_in
    );
    assert_min_out(test_case, &requests, lease_amount);
}

fn simulate_min_out_not_satisfied(test_case: &mut LeaseTestCase, lease: Addr) {
    let mut swap_response = common::swap::do_swap_with_error(&mut test_case.app, lease.clone())
        .expect("on error should have gone into a protected state");
    swap_response.expect_empty();
    let app_response = swap_response.unwrap_response();
    app_response.assert_event(
        &Event::new("wasm-ls-slippage-anomaly")
            .add_attribute("lease", lease.clone().to_string())
            .add_attribute(
                "max_slippage",
                LeaserInstantiator::MAX_SLIPPAGE.units().to_string(),
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
    // far-better price
    let mut response = lease_mod::deliver_new_price(
        test_case,
        lease_amount.checked_div(2).unwrap(),
        borrowed_amount,
    );
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
    requests: &[SwapRequest<PaymentGroup, PaymentGroup>],
    lease_amount: LeaseCoin,
) {
    let price: Price<_, _> = oracle_mod::fetch_price::<LeaseCurrency, LeaseGroup, Lpn, Lpns>(
        test_case.app.query(),
        test_case.address_book.oracle().clone(),
    )
    .unwrap()
    .try_into()
    .unwrap();

    let position_in_lpn = price::total(lease_amount, price);
    assert_eq!(
        (Percent::HUNDRED - LeaserInstantiator::MAX_SLIPPAGE).of(position_in_lpn),
        requests[0].min_token_out.into()
    );
}
