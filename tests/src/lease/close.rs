use finance::price;
use lease::api::{ExecuteMsg, StateResponse};
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse};

use crate::{
    common::{
        leaser as leaser_mod,
        test_case::{response::ResponseWithInterChainMsgs, TestCase},
        USER,
    },
    lease::repay,
};

use super::{dex, heal, LeaseCoin, LeaseCurrency, PaymentCoin, PaymentCurrency, DOWNPAYMENT};

#[test]
fn state_closed() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = DOWNPAYMENT;
    let lease_address = super::open_lease(&mut test_case, downpayment, None);
    let borrowed_lpn = super::quote_borrow(&test_case, downpayment);
    let borrowed: PaymentCoin =
        price::total(borrowed_lpn, super::price_lpn_of::<PaymentCurrency>().inv());
    let lease_amount: LeaseCoin = price::total(
        price::total(downpayment, super::price_lpn_of()) + borrowed_lpn,
        super::price_lpn_of::<LeaseCurrency>().inv(),
    );
    repay::repay(
        &mut test_case,
        lease_address.clone(),
        borrowed,
        lease_amount,
    );

    let customer = Addr::unchecked(USER);
    let user_balance: LeaseCoin =
        platform::bank::balance(&customer, &test_case.app.query()).unwrap();

    close(&mut test_case, lease_address.clone(), lease_amount);

    let query_result = super::state_query(&test_case, lease_address.as_str());
    let expected_result = StateResponse::Closed();

    assert_eq!(query_result, expected_result);

    assert_eq!(
        platform::bank::balance(&customer, &test_case.app.query()).unwrap(),
        user_balance + lease_amount
    );

    leaser_mod::assert_no_leases(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        customer,
    );
    heal::heal_unsupported(&mut test_case, lease_address);
}

fn close<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    contract_addr: Addr,
    expected_funds: LeaseCoin,
) -> AppResponse {
    dex::expect_init_transfer_in(send_close(test_case, contract_addr.clone()));
    dex::do_transfer_in(
        test_case,
        contract_addr,
        expected_funds,
        Option::<LeaseCoin>::None,
    )
}

fn send_close<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    contract_addr: Addr,
) -> ResponseWithInterChainMsgs<'_, ()> {
    test_case
        .app
        .execute(
            Addr::unchecked(USER),
            contract_addr,
            &ExecuteMsg::Close {},
            &[],
        )
        .unwrap()
        .ignore_response()
}
