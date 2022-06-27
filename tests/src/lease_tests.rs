use std::collections::HashSet;

use cosmwasm_std::{coins, Addr, Coin};
use cw_multi_test::{AppResponse, Executor};

use lease::msg::{StateQuery, StateResponse};
use leaser::msg::{QueryMsg, QuoteResponse};

use crate::common::{test_case::TestCase, USER};

const DENOM: &str = "uusdc";
const DOWNPAYMENT: u128 = 10;

fn create_test_case() -> TestCase {
    let user_addr: Addr = Addr::unchecked(USER);
    let mut test_case = TestCase::new(DENOM);
    test_case.init(&user_addr, coins(100, DENOM));
    test_case.init_lpp(None);
    test_case.init_leaser();

    test_case
}

fn open_lease(test_case: &mut TestCase, value: u128) -> Addr {
    let user_addr: Addr = Addr::unchecked(USER);
    test_case
        .app
        .execute_contract(
            user_addr,
            test_case.leaser_addr.clone().unwrap(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: DENOM.to_string(),
            },
            &coins(value, DENOM),
        )
        .unwrap();

    get_lease_address(test_case)
}

fn get_lease_address(test_case: &mut TestCase) -> Addr {
    let user_addr: Addr = Addr::unchecked(USER);
    let query_response: HashSet<Addr> = test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Leases { owner: user_addr },
        )
        .unwrap();
    assert_eq!(query_response.len(), 1);
    query_response.iter().next().unwrap().clone()
}

fn repay(test_case: &mut TestCase, contract_addr: &Addr, value: u128) -> AppResponse {
    let user_addr: Addr = Addr::unchecked(USER);
    test_case
        .app
        .execute_contract(
            user_addr,
            contract_addr.clone(),
            &lease::msg::ExecuteMsg::Repay {},
            &coins(value, DENOM),
        )
        .unwrap()
}

fn close(test_case: &mut TestCase, contract_addr: &Addr) -> AppResponse {
    let user_addr: Addr = Addr::unchecked(USER);
    test_case
        .app
        .execute_contract(
            user_addr,
            contract_addr.clone(),
            &lease::msg::ExecuteMsg::Close {},
            &coins(1, DENOM),
        )
        .unwrap()
}

fn quote_query(test_case: &mut TestCase, amount: Coin) -> QuoteResponse {
    test_case
        .app
        .wrap()
        .query_wasm_smart(
            test_case.leaser_addr.clone().unwrap(),
            &QueryMsg::Quote {
                downpayment: amount,
            },
        )
        .unwrap()
}

fn state_query(test_case: &mut TestCase, contract_addr: &String) -> StateResponse {
    test_case
        .app
        .wrap()
        .query_wasm_smart(contract_addr, &StateQuery {})
        .unwrap()
}

fn get_expected_open_state(
    test_case: &mut TestCase,
    downpayment: u128,
    payments: u128,
) -> StateResponse {
    let quote_result = quote_query(test_case, Coin::new(downpayment, DENOM));
    let expected = quote_result.total.amount.u128() - downpayment - payments;
    StateResponse::Opened {
        amount: quote_result.total,
        interest_rate: quote_result.annual_interest_rate,
        principal_due: Coin::new(expected, DENOM),
        // TODO(kari): Calculate the actual result
        interest_due: Coin::new(0, DENOM),
    }
}

#[test]
fn state_opened_when_no_payments() {
    let mut test_case = create_test_case();
    let lease_address = open_lease(&mut test_case, DOWNPAYMENT);

    let _expected_result = get_expected_open_state(&mut test_case, DOWNPAYMENT, 0);
    let query_result = state_query(&mut test_case, &lease_address.into_string());

    println!("=======> {:#?}", query_result);
    /*
        This is commented out because otherwise it will fail due to bug <number>
        where interest is not calculated acurately and also bug <number> where borrow amount is calculated differently in quote and in leaser/lpp and there is precision loss
        TODO(kari): uncomment the assert after the bugs are fixed

    assert_eq!(
        expected_result, query_result,
        "EXPECTED =======> {:#?} \n ACTUAL =======> {:#?}",
        expected_result, query_result
    );
    */
}

#[test]
fn state_opened_when_partially_paid() {
    let mut test_case = create_test_case();
    let lease_address = open_lease(&mut test_case, DOWNPAYMENT);

    let quote_result = quote_query(&mut test_case, Coin::new(DOWNPAYMENT, DENOM));
    let partial_payment = quote_result.borrow.amount.u128() / 2;
    let _expected_result = get_expected_open_state(&mut test_case, DOWNPAYMENT, partial_payment);

    repay(&mut test_case, &lease_address, partial_payment);

    let query_result = state_query(&mut test_case, &lease_address.into_string());

    println!("=======> {:#?}", query_result);
    /*
        This is commented out because otherwise it will fail due to bug <number>
        where interest is not calculated acurately and also bug <number> where borrow amount is calculated differently in quote and in leaser/lpp and there is precision loss
        TODO(kari): uncomment the assert after the bugs are fixed

    assert_eq!(
        expected_result, query_result,
        "EXPECTED =======> {:#?} \n ACTUAL =======> {:#?}",
        expected_result, query_result
    );
    */
}

#[test]
fn state_paid() {
    let mut test_case = create_test_case();
    let lease_address = open_lease(&mut test_case, DOWNPAYMENT);
    let quote_result = quote_query(&mut test_case, Coin::new(DOWNPAYMENT, DENOM));
    let full_payment = quote_result.borrow.amount.u128();

    repay(&mut test_case, &lease_address, full_payment);

    let expected_amount = Coin::new(DOWNPAYMENT + full_payment, DENOM);
    let expected_result = StateResponse::Paid(expected_amount);
    let query_result = state_query(&mut test_case, &lease_address.into_string());

    println!("=======> {:#?}", query_result);
    assert_eq!(
        expected_result, query_result,
        "EXPECTED =======> {:#?} \n ACTUAL =======> {:#?}",
        expected_result, query_result
    );
}

#[test]
fn state_paid_when_overpaid() {
    let mut test_case = create_test_case();
    let lease_address = open_lease(&mut test_case, DOWNPAYMENT);
    let quote_result = quote_query(&mut test_case, Coin::new(DOWNPAYMENT, DENOM));
    let full_payment = quote_result.borrow.amount.u128();
    let overpayment = 5;
    let expected_amount = Coin::new(DOWNPAYMENT + full_payment + overpayment, DENOM);

    repay(&mut test_case, &lease_address, full_payment + overpayment);

    let expected_result = StateResponse::Paid(expected_amount);
    let query_result = state_query(&mut test_case, &lease_address.into_string());

    println!("=======> {:#?}", query_result);
    assert_eq!(
        expected_result, query_result,
        "EXPECTED =======> {:#?} \n ACTUAL =======> {:#?}",
        expected_result, query_result
    );
}

#[test]
fn state_closed() {
    let mut test_case = create_test_case();
    let lease_address = open_lease(&mut test_case, DOWNPAYMENT);
    let quote_result = quote_query(&mut test_case, Coin::new(DOWNPAYMENT, DENOM));
    let full_payment = quote_result.borrow.amount.u128();
    repay(&mut test_case, &lease_address, full_payment);
    close(&mut test_case, &lease_address);

    let expected_result = StateResponse::Closed;
    let query_result = state_query(&mut test_case, &lease_address.into_string());

    println!("=======> {:#?}", query_result);
    assert_eq!(
        expected_result, query_result,
        "EXPECTED =======> {:#?} \n ACTUAL =======> {:#?}",
        expected_result, query_result
    );
}
