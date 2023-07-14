use super::*;

pub(crate) fn close<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    contract_addr: Addr,
    expected_funds: &[CwCoin],
) -> AppResponse {
    let response: ResponseWithInterChainMsgs<'_, ()> =
        send_close(test_case, contract_addr.clone());

    expect_remote_ibc_transfer(response);

    do_remote_ibc_transfer(test_case, contract_addr, expected_funds)
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

fn expect_remote_ibc_transfer(mut response: ResponseWithInterChainMsgs<'_, ()>) {
    response.expect_submit_tx(TestCase::LEASER_CONNECTION_ID, "0", 1);

    () = response.unwrap_response()
}

fn do_remote_ibc_transfer<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    contract_addr: Addr,
    funds: &[CwCoin],
) -> AppResponse {
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
        .sudo(contract_addr, &construct_response(Binary::default()))
        .unwrap()
        .unwrap_response()
}
