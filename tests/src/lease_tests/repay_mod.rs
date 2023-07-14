use super::*;

pub(crate) fn repay<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>,
    contract_addr: Addr,
    payment: PaymentCoin,
) -> AppResponse {
    let cw_payment: CwCoin = cwcoin(payment);

    let response: ResponseWithInterChainMsgs<'_, ()> =
        send_payment_and_transfer(test_case, contract_addr.clone(), cw_payment.clone());

    expect_swap(response);

    let swap_out_lpn: LpnCoin = price::total(payment, price_lpn_of());

    let response: ResponseWithInterChainMsgs<'_, ()> =
        do_swap(test_case, contract_addr.clone(), &cw_payment, swap_out_lpn);

    expect_remote_ibc_transfer(response);

    do_remote_ibc_transfer(test_case, contract_addr, &cwcoin(swap_out_lpn))
}

fn send_payment_and_transfer<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>,
    contract_addr: Addr,
    cw_payment: CwCoin,
) -> ResponseWithInterChainMsgs<'_, ()> {
    let mut response: ResponseWithInterChainMsgs<'_, ()> = test_case
        .app
        .execute(
            Addr::unchecked(USER),
            contract_addr.clone(),
            &ExecuteMsg::Repay {},
            std::slice::from_ref(&cw_payment),
        )
        .unwrap()
        .ignore_response();

    response.expect_ibc_transfer(
        "channel-0",
        cw_payment.clone(),
        contract_addr.as_str(),
        "ica0",
    );

    () = response.unwrap_response();

    test_case
        .app
        .send_tokens(
            contract_addr.clone(),
            Addr::unchecked("ica0"),
            &[cw_payment],
        )
        .unwrap();

    test_case
        .app
        .sudo(contract_addr, &construct_response(Binary::default()))
        .unwrap()
        .ignore_response()
}

fn expect_swap(mut response: ResponseWithInterChainMsgs<'_, ()>) {
    response.expect_submit_tx(TestCase::LEASER_CONNECTION_ID, "0", 1);

    response.unwrap_response()
}

fn do_swap<'r, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>(
    test_case: &'r mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>,
    contract_addr: Addr,
    cw_payment: &CwCoin,
    swap_out_lpn: LpnCoin,
) -> ResponseWithInterChainMsgs<'r, ()> {
    test_case
        .app
        .send_tokens(
            Addr::unchecked("ica0"),
            Addr::unchecked(ADMIN),
            std::slice::from_ref(cw_payment),
        )
        .unwrap();

    test_case.send_funds_from_admin(Addr::unchecked("ica0"), &[cwcoin(swap_out_lpn)]);

    test_case
        .app
        .sudo(
            contract_addr,
            &construct_response(Binary(platform::trx::encode_msg_responses(
                [platform::trx::encode_msg_response(
                    MsgSwapExactAmountInResponse {
                        token_out_amount: Amount::from(swap_out_lpn).to_string(),
                    },
                    MsgSwapExactAmountIn::TYPE_URL,
                )]
                .into_iter(),
            ))),
        )
        .unwrap()
        .ignore_response()
}

fn expect_remote_ibc_transfer(mut response: ResponseWithInterChainMsgs<'_, ()>) {
    response.expect_submit_tx(TestCase::LEASER_CONNECTION_ID, "0", 1);

    response.unwrap_response()
}

fn do_remote_ibc_transfer<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>,
    contract_addr: Addr,
    cw_swap_out_lpn: &CwCoin,
) -> AppResponse {
    test_case
        .app
        .send_tokens(
            Addr::unchecked("ica0"),
            contract_addr.clone(),
            std::slice::from_ref(cw_swap_out_lpn),
        )
        .unwrap();

    test_case
        .app
        .sudo(contract_addr, &construct_response(Binary::default()))
        .unwrap()
        .unwrap_response()
}
