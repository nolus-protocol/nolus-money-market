use finance::price;
use lease::api::{ExecuteMsg, StateResponse};
use sdk::{
    cosmos_sdk_proto::{ibc::applications::transfer::v1::MsgTransfer, traits::TypeUrl as _},
    cosmwasm_std::{Addr, Binary, Coin as CwCoin},
    cw_multi_test::AppResponse,
};

use crate::{
    common::{
        cwcoin,
        test_case::{
            app::Wasm as WasmTrait,
            response::{RemoteChain as _, ResponseWithInterChainMsgs},
            TestCase,
        },
        USER,
    },
    lease::repay,
};

use super::{heal, LeaseCoin, LeaseCurrency, PaymentCoin, PaymentCurrency, DOWNPAYMENT};

#[test]
fn state_closed() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = super::create_payment_coin(DOWNPAYMENT);
    let lease_address = super::open_lease(&mut test_case, downpayment, None);
    let borrowed: PaymentCoin = price::total(
        super::quote_borrow(&test_case, downpayment),
        super::price_lpn_of::<PaymentCurrency>().inv(),
    );
    let lease_amount: LeaseCoin = price::total(
        price::total(downpayment, super::price_lpn_of())
            + super::quote_borrow(&test_case, downpayment),
        super::price_lpn_of::<LeaseCurrency>().inv(),
    );
    repay::repay(&mut test_case, lease_address.clone(), borrowed);

    let user_balance: LeaseCoin =
        platform::bank::balance(&Addr::unchecked(USER), &test_case.app.query()).unwrap();

    close(
        &mut test_case,
        lease_address.clone(),
        &[cwcoin(lease_amount)],
    );

    let query_result = super::state_query(&test_case, lease_address.as_str());
    let expected_result = StateResponse::Closed();

    assert_eq!(query_result, expected_result);

    assert_eq!(
        platform::bank::balance(&Addr::unchecked(USER), &test_case.app.query()).unwrap(),
        user_balance + lease_amount
    );

    heal::heal_unsupported(&mut test_case, lease_address);
}

fn close<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &mut TestCase<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    contract_addr: Addr,
    expected_funds: &[CwCoin],
) -> AppResponse
where
    Wasm: WasmTrait,
{
    let response: ResponseWithInterChainMsgs<'_, ()> = send_close(test_case, contract_addr.clone());

    expect_remote_ibc_transfer(response);

    do_remote_ibc_transfer(test_case, contract_addr, expected_funds)
}

fn send_close<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &mut TestCase<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    contract_addr: Addr,
) -> ResponseWithInterChainMsgs<'_, ()>
where
    Wasm: WasmTrait,
{
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

fn expect_remote_ibc_transfer(mut response: ResponseWithInterChainMsgs<'_, ()>) {
    response.expect_submit_tx(
        TestCase::LEASER_CONNECTION_ID,
        "0",
        &[MsgTransfer::TYPE_URL],
    );

    () = response.unwrap_response()
}

fn do_remote_ibc_transfer<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &mut TestCase<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    contract_addr: Addr,
    funds: &[CwCoin],
) -> AppResponse
where
    Wasm: WasmTrait,
{
    assert_eq!(
        test_case
            .app
            .query()
            .query_all_balances(contract_addr.clone())
            .unwrap(),
        &[] as &[CwCoin]
    );

    test_case
        .app
        .send_tokens(Addr::unchecked("ica0"), contract_addr.clone(), funds)
        .unwrap();

    assert_eq!(
        test_case.app.query().query_all_balances("ica0").unwrap(),
        &[] as &[CwCoin]
    );

    /* Confirm transfer */
    test_case
        .app
        .sudo(contract_addr, &super::construct_response(Binary::default()))
        .unwrap()
        .unwrap_response()
}
