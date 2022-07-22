use std::collections::HashSet;

use cosmwasm_std::{Addr, Timestamp};
use cw_multi_test::{AppResponse, Executor};

use lease::msg::{StateQuery, StateResponse};
use leaser::msg::{QueryMsg, QuoteResponse};
use platform::coin_legacy::to_cosmwasm;

use crate::common::test_case::TestCase;
use finance::{
    coin::Coin, currency::Usdc, duration::Duration,
    interest::InterestPeriod, percent::Percent,
};

type Currency = Usdc;
type TheCoin = Coin<Currency>;
const DENOM: &str = <Usdc as finance::currency::Currency>::SYMBOL;
const DOWNPAYMENT: u128 = 10;

fn create_coin(amount: u128) -> TheCoin {
    Coin::<Currency>::new(amount)
}

fn create_test_case() -> TestCase {
    let mut test_case = TestCase::new(DENOM);
    test_case.init(
        &Addr::unchecked("user"),
        vec![to_cosmwasm(create_coin(100))],
    );
    test_case.init_lpp(None);
    test_case.init_leaser();

    test_case
}

fn calculate_interest(principal: TheCoin, interest_rate: Percent, duration: u64) -> TheCoin {
    InterestPeriod::with_interest(interest_rate)
        .from(Timestamp::from_nanos(0))
        .spanning(Duration::from_nanos(duration))
        .interest(principal)
}

fn open_lease(test_case: &mut TestCase, value: TheCoin) -> Addr {
    test_case
        .app
        .execute_contract(
            Addr::unchecked("user"),
            test_case.leaser_addr.clone().unwrap(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: DENOM.to_string(),
            },
            &[to_cosmwasm(value)],
        )
        .unwrap();

    get_lease_address(test_case)
}

fn get_lease_address(test_case: &TestCase) -> Addr {
    let query_response: HashSet<Addr> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Leases {
                owner: Addr::unchecked("user"),
            },
        )
        .unwrap();
    assert_eq!(query_response.len(), 1);
    query_response.iter().next().unwrap().clone()
}

fn repay(test_case: &mut TestCase, contract_addr: &Addr, value: TheCoin) -> AppResponse {
    test_case
        .app
        .execute_contract(
            Addr::unchecked("user"),
            contract_addr.clone(),
            &lease::msg::ExecuteMsg::Repay {},
            &[to_cosmwasm(value)],
        )
        .unwrap()
}

fn close(test_case: &mut TestCase, contract_addr: &Addr) -> AppResponse {
    test_case
        .app
        .execute_contract(
            Addr::unchecked("user"),
            contract_addr.clone(),
            &lease::msg::ExecuteMsg::Close {},
            &[],
        )
        .unwrap()
}

fn quote_borrow(test_case: &TestCase, amount: TheCoin) -> TheCoin {
    quote_query(test_case, amount).borrow.try_into().unwrap()
}

fn quote_query(test_case: &TestCase, amount: TheCoin) -> QuoteResponse {
    test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Quote {
                downpayment: amount.into(),
            },
        )
        .unwrap()
}

fn state_query(test_case: &TestCase, contract_addr: &String) -> StateResponse<Currency, Currency> {
    test_case
        .app
        .wrap()
        .query_wasm_smart(contract_addr, &StateQuery {})
        .unwrap()
}

fn expected_open_state(
    test_case: &TestCase,
    downpayment: TheCoin,
    payments: TheCoin,
    duration: u64,
) -> StateResponse<Currency, Currency> {
    let quote_result = quote_query(test_case, downpayment);
    let total = quote_result.total.try_into().unwrap();
    let expected = total - downpayment - payments;
    StateResponse::Opened {
        amount: total,
        interest_rate: quote_result.annual_interest_rate,
        principal_due: expected,
        interest_due: calculate_interest(expected, quote_result.annual_interest_rate, duration),
    }
}

#[test]
fn state_opened_when_no_payments() {
    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let expected_result = expected_open_state(&test_case, downpayment, create_coin(0), 0);
    let lease_address = open_lease(&mut test_case, downpayment);

    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(expected_result, query_result);
}

#[test]
fn state_opened_when_partially_paid() {
    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);

    let quote_result = quote_query(&test_case, downpayment);
    let amount: Coin<Usdc> = quote_result.borrow.try_into().unwrap();
    let partial_payment = create_coin(u128::from(amount) / 2);
    let expected_result = expected_open_state(&test_case, downpayment, partial_payment, 0);

    let lease_address = open_lease(&mut test_case, downpayment);
    repay(&mut test_case, &lease_address, partial_payment);

    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(expected_result, query_result);
}

#[test]
fn state_paid() {
    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment);
    let borrowed = quote_borrow(&test_case, downpayment);

    repay(&mut test_case, &lease_address, borrowed);

    let expected_amount = downpayment + borrowed;
    let expected_result = StateResponse::Paid(expected_amount);
    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(expected_result, query_result);
}

#[test]
fn state_paid_when_overpaid() {
    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment);
    let borrowed = quote_borrow(&test_case, downpayment);

    let overpayment = create_coin(5);
    let payment = borrowed + overpayment;
    let expected_amount = downpayment + payment;

    repay(&mut test_case, &lease_address, payment);

    let expected_result = StateResponse::Paid(expected_amount);
    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(expected_result, query_result);
}

#[test]
fn state_closed() {
    let mut test_case = create_test_case();
    let downpayment = create_coin(DOWNPAYMENT);
    let lease_address = open_lease(&mut test_case, downpayment);
    let borrowed = quote_borrow(&test_case, downpayment);
    repay(&mut test_case, &lease_address, borrowed);
    close(&mut test_case, &lease_address);

    let expected_result = StateResponse::Closed();
    let query_result = state_query(&test_case, &lease_address.into_string());

    assert_eq!(expected_result, query_result);
}
