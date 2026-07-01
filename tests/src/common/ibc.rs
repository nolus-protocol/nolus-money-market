use std::slice;

use super::testing;
use currencies::PaymentGroup;
use currency::{BankSymbols, CurrencyDTO, DexSymbols, Symbol, SymbolStatic};
use sdk::{
    cosmwasm_std::{Addr, Binary},
    cw_multi_test::AppResponse,
    ica::{RequestPacket, SudoMsg},
};

use crate::common::ADMIN;

use super::{
    CwCoin,
    test_case::{
        app::App,
        response::{RemoteChain as _, ResponseWithInterChainMsgs},
    },
};

pub(crate) fn expect_transfer<T>(
    response: &mut ResponseWithInterChainMsgs<'_, T>,
    channel: &str,
    addr: &str,
    ica_addr: &str,
) -> CwCoin where
{
    response.expect_ibc_transfer(channel, addr, ica_addr)
}

/// Consume the next IBC transfer asserting only its `channel`, returning its
/// token. Unlike [`expect_transfer`] the receiver is not pinned: a funding
/// transfer is addressed to the per-lease `LeaseAuthority`, which the test
/// decouples from the holdings stand-in (`ica_addr`) it lands the funds on.
pub(crate) fn take_transfer<T>(
    response: &mut ResponseWithInterChainMsgs<'_, T>,
    channel: &str,
) -> CwCoin {
    let (_sender, _receiver, token) = response.take_ibc_transfer(channel);
    token
}

pub(crate) fn do_transfer<'r>(
    app: &'r mut App,
    sender: Addr,
    recipient: Addr,
    on_remote_chain: bool,
    cw_coin: &CwCoin,
) -> ResponseWithInterChainMsgs<'r, AppResponse> {
    let contract_addr: Addr = if on_remote_chain { &recipient } else { &sender }.clone();

    do_transfer_no_response(
        app,
        sender.clone(),
        recipient.clone(),
        on_remote_chain,
        cw_coin,
    );
    send_blank_response(app, contract_addr)
}

fn do_transfer_no_response(
    app: &mut App,
    sender: Addr,
    recipient: Addr,
    on_remote_chain: bool,
    cw_coin: &CwCoin,
) {
    let new_symbol: SymbolStatic = if on_remote_chain {
        dex_to_bank(&cw_coin.denom)
    } else {
        bank_to_dex(&cw_coin.denom)
    };

    app.send_tokens(sender, testing::user(ADMIN), slice::from_ref(cw_coin))
        .unwrap();

    app.send_tokens(
        testing::user(ADMIN),
        recipient,
        &[CwCoin::new(cw_coin.amount, new_symbol)],
    )
    .unwrap();
}

pub(super) fn send_response(
    app: &mut App,
    contract_addr: Addr,
    resp: Binary,
) -> ResponseWithInterChainMsgs<'_, AppResponse> {
    app.sudo(
        contract_addr,
        &SudoMsg::Response {
            // TODO fill-in with real/valid response data
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
            data: resp,
        },
    )
    .unwrap()
}

fn send_blank_response(app: &mut App, addr: Addr) -> ResponseWithInterChainMsgs<'_, AppResponse> {
    send_response(app, addr, Binary::new(vec![]))
}

/// Deliver an ICS-20 transfer timeout to `contract_addr` over the sudo path -
/// the signal the funding leg re-emits its single in-flight coin on.
pub(crate) fn timeout_transfer(
    app: &mut App,
    contract_addr: Addr,
) -> ResponseWithInterChainMsgs<'_, AppResponse> {
    app.sudo(
        contract_addr,
        &SudoMsg::Timeout {
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
        },
    )
    .unwrap()
}

fn dex_to_bank(symbol: &str) -> SymbolStatic {
    symbol_net_to_net::<DexSymbols<PaymentGroup>, BankSymbols<PaymentGroup>>(symbol)
}

fn bank_to_dex(symbol: &str) -> SymbolStatic {
    symbol_net_to_net::<BankSymbols<PaymentGroup>, DexSymbols<PaymentGroup>>(symbol)
}

fn symbol_net_to_net<FromS, IntoS>(symbol: &str) -> SymbolStatic
where
    FromS: Symbol<Group = PaymentGroup>,
    IntoS: Symbol<Group = PaymentGroup>,
{
    CurrencyDTO::<PaymentGroup>::from_symbol_testing::<FromS>(symbol)
        .unwrap()
        .into_symbol::<IntoS>()
}
