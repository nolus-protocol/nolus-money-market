use currencies::PaymentGroup;
use finance::{
    coin::{Amount, Coin, CoinDTO},
    percent::Percent,
};
use lease::api::query::{StateResponse, opened::Status};
use sdk::{
    cosmwasm_std::{Addr, Event},
    testing,
};
use swap::testing::SwapRequest;

use crate::{
    common::{
        self, LEASE_ADMIN,
        test_case::{TestCase, response::RemoteChain},
    },
    lease::{self as lease_mod, heal},
};

use super::{DOWNPAYMENT, LeaseCurrency, PaymentCurrency};

#[test]
fn full_liquidation() {
    let mut test_case = lease_mod::create_test_case::<PaymentCurrency>();

    let lease_addr: Addr = lease_mod::open_lease(&mut test_case, DOWNPAYMENT, None);

    // let ica_addr: Addr = TestCase::ica_addr(&lease_addr, TestCase::LEASE_ICA_ID);

    let lease_amount: Amount = 2857142857142;
    let borrowed_amount: Amount = 1857142857142;

    // the base is chosen to be close to the position amount to trigger a full liquidation
    let response = lease_mod::deliver_new_price(
        &mut test_case,
        (lease_amount + 10).into(),
        borrowed_amount.into(),
    );

    let requests: Vec<SwapRequest<PaymentGroup, PaymentGroup>> = common::swap::expect_swap(
        response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
        |_| {},
    );
    //the `expect_swap` postconditions guarantee there is at least one item
    assert_eq!(
        Into::<CoinDTO<PaymentGroup>>::into(Coin::<LeaseCurrency>::from(lease_amount)),
        requests[0].token_in
    );
    assert_min_out(&requests, lease_amount);

    let mut swap_response =
        common::swap::do_swap_with_error(&mut test_case.app, lease_addr.clone())
            .expect("on error should have gone into a protected state");
    swap_response.expect_empty();
    let app_response = swap_response.unwrap_response();
    app_response.assert_event(
        &Event::new("wasm-ls-slippage-anomaly")
            .add_attribute("lease", lease_addr.to_string())
            .add_attribute(
                "max_slippage",
                Percent::from_percent(15).units().to_string(),
            ), //TODO obtain it from the leaser
    );

    let query_result = super::state_query(&test_case, lease_addr.clone());
    assert!(matches!(
        query_result,
        StateResponse::Opened {
            status: Status::SlippageProtectionActivated,
            ..
        }
    ));

    let heal_response = heal::heal_ok(&mut test_case.app, lease_addr, testing::user(LEASE_ADMIN));
    let requests: Vec<SwapRequest<PaymentGroup, PaymentGroup>> = common::swap::expect_swap(
        heal_response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
        |_| {},
    );
    assert_min_out(&requests, lease_amount);

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

fn assert_min_out(requests: &[SwapRequest<PaymentGroup, PaymentGroup>], lease_amount: Amount) {
    //TODO query the slippage tolerance from the leaser and calculate the expected
    assert!(dbg!(requests[0].min_token_out) > 0);
    assert!(requests[0].min_token_out < lease_amount);
}
