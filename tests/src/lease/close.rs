use std::slice;

use finance::price;
use lease::api::{ExecuteMsg, query::StateResponse};
use platform::coin_legacy::to_cosmwasm_on_dex;
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse, testing};

use crate::common::{
    CwCoin, USER, ibc, leaser as leaser_mod,
    test_case::{TestCase, response::ResponseWithInterChainMsgs},
};

use super::{
    DOWNPAYMENT, LeaseCoin, LeaseCurrency, LeaseTestCase, LpnCoin, PaymentCoin, PaymentCurrency,
    heal, repay,
};

#[test]
fn state_closed() {
    let mut test_case: LeaseTestCase = super::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = DOWNPAYMENT;
    let lease: Addr = super::open_lease(&mut test_case, downpayment, None);

    let borrowed_lpn: LpnCoin = super::quote_borrow(&test_case, downpayment);
    let borrowed: PaymentCoin =
        price::total(borrowed_lpn, super::price_lpn_of::<PaymentCurrency>().inv());
    let lease_amount: LeaseCoin = price::total(
        price::total(downpayment, super::price_lpn_of()) + borrowed_lpn,
        super::price_lpn_of::<LeaseCurrency>().inv(),
    );
    repay::repay(&mut test_case, lease.clone(), borrowed);

    let customer_addr: Addr = testing::user(USER);
    let user_balance: LeaseCoin =
        platform::bank::balance(&customer_addr, test_case.app.query()).unwrap();

    close(&mut test_case, lease.clone(), lease_amount);

    let query_result: StateResponse = super::state_query(&test_case, lease.clone());
    let expected_result: StateResponse = StateResponse::Closed();

    assert_eq!(query_result, expected_result);

    assert_eq!(
        platform::bank::balance(&customer_addr, test_case.app.query()).unwrap(),
        user_balance + lease_amount
    );

    leaser_mod::assert_no_leases(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        customer_addr,
    );
    heal::heal_no_inconsistency(&mut test_case.app, lease, testing::user(USER));
}

fn close<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    lease_addr: Addr,
    expected_funds: LeaseCoin,
) -> AppResponse {
    let ica_addr: Addr = TestCase::ica_addr(&lease_addr, TestCase::LEASE_ICA_ID);

    let mut response: ResponseWithInterChainMsgs<'_, ()> =
        send_close(test_case, lease_addr.clone());

    let transfer_amount: CwCoin = ibc::expect_remote_transfer(
        &mut response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );

    assert_eq!(transfer_amount, to_cosmwasm_on_dex(expected_funds));

    () = response.unwrap_response();

    ibc::do_transfer(
        &mut test_case.app,
        ica_addr,
        lease_addr,
        true,
        slice::from_ref(&transfer_amount),
    )
    .unwrap_response()
}

fn send_close<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    contract_addr: Addr,
) -> ResponseWithInterChainMsgs<'_, ()> {
    test_case
        .app
        .execute(
            testing::user(USER),
            contract_addr,
            &ExecuteMsg::Close {},
            &[],
        )
        .unwrap()
        .ignore_response()
}
