use std::collections::HashMap;

use ::lease::api::{ExecuteMsg, StateResponse};
use currency::Currency;
use finance::{coin::Amount, duration::Duration};
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse};

use crate::{
    common::{
        ibc,
        leaser::Instantiator as LeaserInstantiator,
        test_case::{response::ResponseWithInterChainMsgs, TestCase},
        CwCoin,
    },
    lease::{self, LpnCurrency},
};

use super::{LeaseCoin, PaymentCoin, PaymentCurrency, DOWNPAYMENT};

fn liquidation_time_alarm(time_pass: Duration, liquidation_amount: Option<LeaseCoin>) {
    let mut test_case: TestCase<_, _, _, _, _, _, _> = lease::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = DOWNPAYMENT;
    let lease_addr: Addr = lease::open_lease(&mut test_case, downpayment, None);

    let StateResponse::Opened {
        amount: lease_amount,
        ..
    }: StateResponse = lease::state_query(&test_case, lease_addr.as_ref()) else {
        unreachable!()
    };
    let lease_amount: LeaseCoin = lease_amount.try_into().unwrap();

    test_case.app.time_shift(time_pass);

    lease::feed_price(&mut test_case);

    let mut response: ResponseWithInterChainMsgs<'_, ()> = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            lease_addr.clone(),
            &ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap()
        .ignore_response();

    let Some(liquidation_amount): Option<LeaseCoin> = liquidation_amount else {
        () = response.unwrap_response();

        return;
    };

    let requests: Vec<swap::trx::RequestMsg> = crate::common::swap::expect_swap(
        &mut response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );

    () = response.unwrap_response();

    let ica_addr: Addr = TestCase::ica_addr(lease_addr.as_str(), TestCase::LEASE_ICA_ID);

    let mut response: ResponseWithInterChainMsgs<'_, ()> = crate::common::swap::do_swap(
        &mut test_case.app,
        lease_addr.clone(),
        ica_addr.clone(),
        requests.into_iter(),
        |amount: u128, _: &str, _: &str| amount,
    )
    .ignore_response();

    let ibc_transfer_coin: CwCoin = ibc::expect_remote_transfer(
        &mut response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );

    () = response.unwrap_response();

    assert_eq!(ibc_transfer_coin.amount.u128(), liquidation_amount.into());
    assert_eq!(ibc_transfer_coin.denom, LpnCurrency::DEX_SYMBOL);

    let response: ResponseWithInterChainMsgs<'_, AppResponse> = ibc::do_transfer(
        &mut test_case.app,
        ica_addr,
        lease_addr.clone(),
        true,
        &ibc_transfer_coin,
    );

    let liquidation_end_response: AppResponse = response.unwrap_response();

    assert_eq!(
        test_case
            .app
            .query()
            .query_all_balances(lease_addr.clone())
            .unwrap(),
        &[],
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

    let query_result = lease::state_query(&test_case, lease_addr.as_str());

    let liquidated_amount: LeaseCoin = liquidation_attributes["amount-amount"]
        .parse::<Amount>()
        .unwrap()
        .into();

    assert_eq!(liquidated_amount, liquidation_amount);

    if let StateResponse::Opened {
        amount,
        previous_margin_due,
        previous_interest_due,
        ..
    } = query_result
    {
        assert_eq!(
            LeaseCoin::try_from(amount).unwrap(),
            lease_amount - liquidated_amount
        );

        assert!(previous_margin_due.is_zero());

        assert!(previous_interest_due.is_zero());
    }
}

#[test]
fn liquidation_time_alarm_0() {
    liquidation_time_alarm(
        LeaserInstantiator::REPAYMENT_PERIOD - Duration::from_nanos(1),
        None,
    );
}

#[test]
fn liquidation_time_alarm_1() {
    liquidation_time_alarm(
        LeaserInstantiator::REPAYMENT_PERIOD + LeaserInstantiator::GRACE_PERIOD
            - Duration::from_nanos(1),
        None,
    );
}

#[test]
fn liquidation_time_alarm_2() {
    liquidation_time_alarm(
        LeaserInstantiator::REPAYMENT_PERIOD + LeaserInstantiator::GRACE_PERIOD,
        Some(LeaseCoin::new(45792563600)),
    );
}
