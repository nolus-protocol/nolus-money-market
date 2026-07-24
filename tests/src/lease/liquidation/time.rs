use std::collections::HashMap;

use currencies::PaymentGroup;
use finance::{
    coin::{Amount, CoinDTO},
    duration::Duration,
    price,
    zero::Zero,
};
use lease::api::{ExecuteMsg, query::StateResponse};
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse};

use crate::{
    common::{
        self, lease as common_lease, leaser::Instantiator as LeaserInstantiator,
        remote_lease_controller_stub::SwapFill, swap,
        test_case::response::ResponseWithInterChainMsgs,
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

    let controller = test_case.address_book.remote_lease_controller().clone();
    // Identity DEX fill: the liquidated collateral yields its LPN value.
    swap::set_fill(&mut test_case.app, &controller, SwapFill::InputAmount);

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

    // The liquidation sell-asset swap and the proceeds transfer-out fired inline
    // on the time-alarm; the lease (local-output) is now in TransferInFinish,
    // polling its bank balance for the returning LPN proceeds.
    let _ = response.unwrap_response();

    let proceeds = price::total(liquidation_amount, lease_test::price_lpn_of()).unwrap();

    // Fidelity: the swap input is exactly the liquidated amount.
    let captured = swap::captured(&test_case.app, &controller);
    assert_eq!(
        Into::<CoinDTO<PaymentGroup>>::into(liquidation_amount),
        swap::token_in(&captured),
    );

    // Fidelity: the emitted transfer-out returns exactly the LPN proceeds.
    assert_eq!(
        Into::<CoinDTO<PaymentGroup>>::into(proceeds),
        swap::captured_transfer_out(&test_case.app, &controller),
    );

    let time_alarms = test_case.address_book.time_alarms().clone();
    let response: ResponseWithInterChainMsgs<'_, AppResponse> = swap::deliver_transfer_in(
        &mut test_case.app,
        time_alarms,
        lease_addr.clone(),
        &common::cwcoin(proceeds),
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
