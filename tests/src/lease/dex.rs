use currency::Currency;
use finance::coin::Coin;
use sdk::{
    cosmwasm_std::{Addr, Binary, Coin as CwCoin},
    cw_multi_test::AppResponse,
};

use crate::common::{
    self,
    test_case::{
        response::{RemoteChain, ResponseWithInterChainMsgs},
        TestCase,
    },
};

pub(super) fn expect_init_transfer_in(mut response: ResponseWithInterChainMsgs<'_, ()>) {
    response.expect_submit_tx(TestCase::LEASER_CONNECTION_ID, "0", 1);

    () = response.unwrap_response()
}

pub(super) fn do_transfer_in<
    Dispatcher,
    Treasury,
    Profit,
    Leaser,
    Lpp,
    Oracle,
    TimeAlarms,
    C,
    Asset,
>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    contract_addr: Addr,
    funds: Coin<C>,
    funds_at_ica_after: Option<Coin<Asset>>,
) -> AppResponse
where
    C: Currency,
    Asset: Currency,
{
    assert_eq!(
        test_case
            .app
            .query()
            .query_all_balances(contract_addr.clone())
            .unwrap(),
        vec![]
    );

    test_case
        .app
        .send_tokens(
            Addr::unchecked("ica0"),
            contract_addr.clone(),
            &[common::cwcoin(funds)],
        )
        .unwrap();

    let cw_funds_at_ica_after: Vec<CwCoin> =
        funds_at_ica_after.into_iter().map(common::cwcoin).collect();
    assert_eq!(
        test_case.app.query().query_all_balances("ica0").unwrap(),
        cw_funds_at_ica_after
    );

    let mut response = test_case
        .app
        .sudo(contract_addr, &super::construct_response(Binary::default()))
        .unwrap();
    response.expect_empty();
    response.unwrap_response()
}
