use std::{cell::Cell, slice};

use anyhow::anyhow;
use currency::Currency as _;
use finance::coin::Amount;
use lease::api::ExecuteMsg;
use osmosis_std::types::osmosis::gamm::v1beta1::{
    MsgSwapExactAmountIn, MsgSwapExactAmountInResponse,
};
use platform::trx;
use sdk::{
    cosmos_sdk_proto::{ibc::applications::transfer::v1::MsgTransfer, traits::TypeUrl as _},
    cosmwasm_std::{Addr, Binary, Coin as CwCoin, WasmMsg},
    cw_multi_test::AppResponse,
    neutron_sdk::sudo::msg::{RequestPacket, SudoMsg as NeutronSudoMsg},
};

use crate::common::{
    cwcoin,
    test_case::{
        response::{RemoteChain, ResponseWithInterChainMsgs},
        wasm::{Action, ConfigurableWasmBuilder, Request, Wasm as WasmTrait},
        TestCase,
    },
    ADMIN, USER,
};

use super::{create_test_case_with_wasm, open_lease, Lpn, PaymentCoin, PaymentCurrency};

#[test]
fn safe_delivery() {
    const ERROR_MESSAGE: &str = "Simulated error";

    let forward_specific_execute: Cell<bool> = Cell::new(true);
    let lease_addr_cell: Cell<&str> = Cell::new("DEADCODE");

    let mut test_case: TestCase<_, _, _, _, _, _, _, _> =
        create_test_case_with_wasm::<_, _, Lpn>(|| {
            ConfigurableWasmBuilder::new(|request: Request<'_>| {
                if let Request::Execute(WasmMsg::Execute {
                    contract_addr,
                    msg: _,
                    funds: _,
                }) = request
                {
                    if contract_addr == lease_addr_cell.get()
                        && !forward_specific_execute.replace(true)
                    {
                        return Action::Error(anyhow!(ERROR_MESSAGE));
                    }
                }

                Action::Forward
            })
            .build()
        });

    test_case.send_funds_from_admin(Addr::unchecked(USER), &[cwcoin::<PaymentCurrency, _>(200)]);

    let lease: Addr = open_lease(&mut test_case, PaymentCoin::new(100), None);

    lease_addr_cell.set(lease.as_str());

    let lpp_balance_after_lease: CwCoin = test_case
        .app
        .query()
        .query_balance(
            test_case.address_book.lpp().clone(),
            String::from(Lpn::BANK_SYMBOL),
        )
        .unwrap();

    let repay_cw_coin: CwCoin = cwcoin::<PaymentCurrency, _>(100);

    let mut response: ResponseWithInterChainMsgs<'_, ()> = test_case
        .app
        .execute(
            Addr::unchecked(USER),
            lease.clone(),
            &ExecuteMsg::Repay(),
            slice::from_ref(&repay_cw_coin),
        )
        .unwrap()
        .ignore_response();

    response.expect_ibc_transfer(
        TestCase::LEASER_IBC_CHANNEL,
        repay_cw_coin.clone(),
        lease.as_str(),
        "ica0",
    );

    () = response.unwrap_response();

    test_case
        .app
        .send_tokens(
            lease.clone(),
            Addr::unchecked("ica0"),
            slice::from_ref(&repay_cw_coin),
        )
        .unwrap();

    forward_specific_execute.set(false);

    let response: AppResponse = test_case
        .app
        .sudo(lease.clone(), &ibc_transfer_response())
        .unwrap()
        .unwrap_response();

    expect_failure_and_reschedule(
        response,
        lease.as_str(),
        test_case.address_book.time_alarms().as_str(),
    );

    forward_specific_execute.set(false);

    assert!(execute_time_alarm_raw(&mut test_case, lease.clone())
        .unwrap_err()
        .to_string()
        .contains(ERROR_MESSAGE));

    let mut response: ResponseWithInterChainMsgs<'_, ()> =
        execute_time_alarm(&mut test_case, lease.clone());

    response.expect_submit_tx(
        TestCase::LEASER_CONNECTION_ID,
        "0",
        &[MsgSwapExactAmountIn::TYPE_URL],
    );

    () = response.unwrap_response();

    test_case
        .app
        .send_tokens(
            Addr::unchecked("ica0"),
            Addr::unchecked(ADMIN),
            slice::from_ref(&repay_cw_coin),
        )
        .unwrap();

    let swap_out_amount: Amount = 100;

    test_case.send_funds_from_admin(
        Addr::unchecked("ica0"),
        &[cwcoin::<Lpn, _>(swap_out_amount)],
    );

    forward_specific_execute.set(false);

    let response: AppResponse = test_case
        .app
        .sudo(lease.clone(), &osmosis_swap_response(swap_out_amount))
        .unwrap()
        .unwrap_response();

    expect_failure_and_reschedule(
        response,
        lease.as_str(),
        test_case.address_book.time_alarms().as_str(),
    );

    let mut response: ResponseWithInterChainMsgs<'_, ()> =
        execute_time_alarm(&mut test_case, lease.clone());

    response.expect_submit_tx(
        TestCase::LEASER_CONNECTION_ID,
        "0",
        &[MsgTransfer::TYPE_URL],
    );

    () = response.unwrap_response();

    test_case
        .app
        .send_tokens(
            Addr::unchecked("ica0"),
            lease.clone(),
            &[cwcoin::<Lpn, _>(swap_out_amount)],
        )
        .unwrap();

    forward_specific_execute.set(false);

    let response: AppResponse = test_case
        .app
        .sudo(lease.clone(), &ibc_transfer_response())
        .unwrap()
        .unwrap_response();

    expect_failure_and_reschedule(
        response,
        lease.as_str(),
        test_case.address_book.time_alarms().as_str(),
    );

    () = execute_time_alarm(&mut test_case, lease.clone()).unwrap_response();

    assert_eq!(
        test_case
            .app
            .query()
            .query_balance(test_case.address_book.lpp().clone(), Lpn::BANK_SYMBOL)
            .unwrap()
            .amount
            .u128(),
        lpp_balance_after_lease.amount.u128() + swap_out_amount
    );
}

const fn neutron_response(data: Vec<u8>) -> NeutronSudoMsg {
    NeutronSudoMsg::Response {
        request: RequestPacket {
            sequence: None,
            source_port: None,
            source_channel: None,
            destination_port: None,
            destination_channel: None,
            data: None,
            timeout_height: None,
            timeout_timestamp: None,
        },
        data: Binary(data),
    }
}

const fn ibc_transfer_response() -> NeutronSudoMsg {
    neutron_response(Vec::new())
}

fn osmosis_swap_response(amount: Amount) -> NeutronSudoMsg {
    neutron_response(trx::encode_msg_responses(
        [trx::encode_msg_response(
            MsgSwapExactAmountInResponse {
                token_out_amount: amount.to_string(),
            },
            MsgSwapExactAmountIn::TYPE_URL,
        )]
        .into_iter(),
    ))
}

fn execute_time_alarm_raw<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>,
    lease: Addr,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>>
where
    Wasm: WasmTrait,
{
    test_case.app.execute(
        test_case.address_book.time_alarms().clone(),
        lease,
        &ExecuteMsg::TimeAlarm {},
        &[],
    )
}

fn execute_time_alarm<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>,
    lease: Addr,
) -> ResponseWithInterChainMsgs<'_, ()>
where
    Wasm: WasmTrait,
{
    execute_time_alarm_raw(test_case, lease)
        .unwrap()
        .ignore_response()
}

fn expect_failure_and_reschedule(response: AppResponse, lease: &str, time_alarms: &str) {
    assert_eq!(response.events[0].ty, "sudo");
    assert_eq!(response.events[0].attributes, [("_contract_addr", lease)]);

    assert_eq!(response.events[1].ty, "reply");
    assert_eq!(
        response.events[1].attributes,
        [("_contract_addr", lease), ("mode", "handle_failure")]
    );

    assert_eq!(response.events[2].ty, "wasm-next-delivery");
    assert_eq!(
        response.events[2].attributes,
        [("_contract_addr", lease), ("what", "dex-response")]
    );

    assert_eq!(response.events[3].ty, "execute");
    assert_eq!(
        response.events[3].attributes,
        [("_contract_addr", time_alarms)]
    );
}
