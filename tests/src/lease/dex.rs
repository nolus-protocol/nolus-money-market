use currency::Currency;
use finance::coin::Coin;
use sdk::{
    cosmwasm_std::{Addr, Binary},
    cw_multi_test::AppResponse,
};

use crate::common::{
    self,
    test_case::{
        response::{RemoteChain, ResponseWithInterChainMsgs},
        TestCase,
    },
    ADMIN,
};

pub(super) fn expect_swap(mut response: ResponseWithInterChainMsgs<'_, ()>) {
    response.expect_submit_tx(TestCase::LEASER_CONNECTION_ID, "0", 1);

    () = response.unwrap_response()
}

pub(super) fn do_swap<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, In, Out>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>,
    contract_addr: Addr,
    swap_in: Coin<In>,
    swap_out: Coin<Out>,
) -> ResponseWithInterChainMsgs<'_, ()>
where
    In: Currency,
    Out: Currency,
{
    test_case
        .app
        .send_tokens(
            Addr::unchecked("ica0"),
            Addr::unchecked(ADMIN),
            &[common::cwcoin(swap_in)],
        )
        .unwrap();

    test_case.send_funds_from_admin(Addr::unchecked("ica0"), &[common::cwcoin(swap_out)]);

    test_case
        .app
        .sudo(
            contract_addr,
            &super::construct_response(Binary(platform::trx::encode_msg_responses(
                [swap::trx::build_exact_amount_in_resp(swap_out.into())].into_iter(),
            ))),
        )
        .unwrap()
        .ignore_response()
}

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
    funds_at_ica_after: Coin<Asset>,
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

    assert_eq!(
        test_case.app.query().query_all_balances("ica0").unwrap(),
        common::cwcoin_as_balance(funds_at_ica_after)
    );

    let mut response = test_case
        .app
        .sudo(contract_addr, &super::construct_response(Binary::default()))
        .unwrap();
    response.expect_empty();
    response.unwrap_response()
}
