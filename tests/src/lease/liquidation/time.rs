use std::collections::HashMap;

use currencies::PaymentGroup;
use finance::{coin::Amount, duration::Duration, price, zero::Zero};
use lease::api::{ExecuteMsg, query::StateResponse};
use platform::coin_legacy;
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse};
use swap::testing::SwapRequest;

use crate::{
    common::{
        self, CwCoin, ibc, lease as common_lease,
        leaser::Instantiator as LeaserInstantiator,
        test_case::{TestCase, response::ResponseWithInterChainMsgs},
    },
    lease::{self as lease_test, LeaseCoin, PaymentCurrency},
};

use super::super::LeaseTestCase;

fn liquidation_time_alarm(
    downpayment: Amount,
    time_pass: Duration,
    liquidation_amount: Option<LeaseCoin>,
) {
    let mut test_case: LeaseTestCase = lease_test::create_test_case::<PaymentCurrency>();
    let lease_addr: Addr = lease_test::open_lease(
        &mut test_case,
        common::coin::<PaymentCurrency>(downpayment),
        None,
    );

    let StateResponse::Opened {
        amount: lease_amount,
        ..
    }: StateResponse = lease_test::state_query(&test_case, lease_addr.clone())
    else {
        unreachable!()
    };
    let lease_amount: LeaseCoin = lease_amount.try_into().unwrap();

    test_case.app.time_shift(time_pass);

    lease_test::feed_price(&mut test_case);

    let response = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            lease_addr.clone(),
            &ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();

    let Some(liquidation_amount): Option<LeaseCoin> = liquidation_amount else {
        return;
    };

    let requests: Vec<SwapRequest<PaymentGroup, PaymentGroup>> = crate::common::swap::expect_swap(
        response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
        |_| {},
    );

    let ica_addr: Addr = TestCase::ica_addr(&lease_addr, TestCase::LEASE_ICA_ID);

    let mut response: ResponseWithInterChainMsgs<'_, ()> = crate::common::swap::do_swap(
        &mut test_case.app,
        lease_addr.clone(),
        ica_addr.clone(),
        requests.into_iter(),
        |amount: u128, _, _| amount,
    )
    .ignore_response();

    let transfer_amount: CwCoin = ibc::expect_remote_transfer(
        &mut response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );

    () = response.unwrap_response();

    assert_eq!(
        transfer_amount,
        coin_legacy::to_cosmwasm_on_dex(
            price::total(liquidation_amount, lease_test::price_lpn_of()).unwrap()
        )
    );

    let response: ResponseWithInterChainMsgs<'_, AppResponse> = ibc::do_transfer(
        &mut test_case.app,
        ica_addr,
        lease_addr.clone(),
        true,
        &transfer_amount,
    );

    let liquidation_end_response: AppResponse = response.unwrap_response();

    common_lease::assert_lease_balance_eq(
        &test_case.app,
        &lease_addr,
        common::cwcoin(LeaseCoin::ZERO),
    );

    let liquidation_attributes: HashMap<String, String> = liquidation_end_response
        .events
        .into_iter()
        .find(|event| event.ty == "wasm-ls-liquidation")
        .expect("No liquidation emitted!")
        .attributes
        .into_iter()
        .map(|attribute| (attribute.key, attribute.value))
        .collect();

    let query_result: StateResponse = lease_test::state_query(&test_case, lease_addr);

    let liquidated_amount: LeaseCoin = common::coin(
        liquidation_attributes["amount-amount"]
            .parse::<Amount>()
            .unwrap(),
    );

    assert_eq!(liquidated_amount, liquidation_amount);

    if let StateResponse::Opened {
        amount,
        due_interest,
        due_margin,
        overdue_interest,
        overdue_margin,
        ..
    } = query_result
    {
        assert_eq!(
            LeaseCoin::try_from(amount).unwrap(),
            lease_amount - liquidated_amount
        );
        assert!(due_interest.is_zero());
        assert!(due_margin.is_zero());

        assert!(overdue_interest.is_zero());
        assert!(overdue_margin.is_zero());
    }
}

#[test]
fn liquidation_by_time_due_more_than_min_no_overdue() {
    liquidation_time_alarm(
        1_000_000,
        LeaserInstantiator::REPAYMENT_PERIOD - Duration::from_nanos(1),
        None,
    );
}

#[test]
fn liquidation_by_time_overdue_less_than_min() {
    liquidation_time_alarm(
        100,
        LeaserInstantiator::REPAYMENT_PERIOD + Duration::from_days(1),
        None,
    );
}

#[test]
fn liquidation_by_time_overdue_more_than_min() {
    liquidation_time_alarm(
        1_000_000_000,
        LeaserInstantiator::REPAYMENT_PERIOD,
        Some(LeaseCoin::new(45792562)), //the total interest due for the LeaserInstantiator::REPAYMENT_PERIOD = (7% + 3%) * 65/(100-65)*Downpayment * LeaserInstantiator::REPAYMENT_PERIOD/365
    );
}
