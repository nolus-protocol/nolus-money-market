use std::slice;

use currencies::PaymentGroup;
use currency::{BankSymbols, DexSymbols, Group, SymbolStatic};
use sdk::{
    cosmos_sdk_proto::{
        cosmos::base::v1beta1::Coin as ProtobufCoin,
        ibc::applications::transfer::v1::MsgTransfer,
        traits::{Message, TypeUrl},
    },
    cosmwasm_std::{Addr, Binary},
    cw_multi_test::AppResponse,
    neutron_sdk::{
        bindings::types::ProtobufAny,
        sudo::msg::{RequestPacket, SudoMsg},
    },
};

use crate::common::ADMIN;

use super::{
    test_case::{
        app::App,
        response::{RemoteChain as _, ResponseWithInterChainMsgs},
    },
    CwCoin,
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

pub(crate) fn expect_remote_transfer<T>(
    response: &mut ResponseWithInterChainMsgs<'_, T>,
    connection_id: &str,
    ica_id: &str,
) -> CwCoin where
{
    let messages: Vec<ProtobufAny> = response.expect_submit_tx(connection_id, ica_id);

    let message: MsgTransfer = match messages.as_slice() {
        [message] if message.type_url == MsgTransfer::TYPE_URL => {
            Message::decode(message.value.as_slice()).unwrap()
        }
        _ => unimplemented!(),
    };

    let token: ProtobufCoin = message.token.unwrap();

    CwCoin::new(token.amount.parse().unwrap(), token.denom)
}

pub(crate) fn do_transfer<'r>(
    app: &'r mut App,
    sender: Addr,
    recipient: Addr,
    on_remote_chain: bool,
    cw_coin: &CwCoin,
) -> ResponseWithInterChainMsgs<'r, AppResponse> {
    let contract_addr: Addr = if on_remote_chain { &recipient } else { &sender }.clone();

    do_transfer_no_response(app, sender, recipient, on_remote_chain, cw_coin);

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
        PaymentGroup::maybe_visit(&DexSymbols, &cw_coin.denom, BankSymbols).ok()
    } else {
        PaymentGroup::maybe_visit(&BankSymbols, &cw_coin.denom, DexSymbols).ok()
    }
    .unwrap()
    .unwrap();

    app.send_tokens(
        sender.clone(),
        Addr::unchecked(ADMIN),
        slice::from_ref(cw_coin),
    )
    .unwrap();

    app.send_tokens(
        Addr::unchecked(ADMIN),
        recipient.clone(),
        &[CwCoin::new(cw_coin.amount.u128(), new_symbol)],
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
    send_response(app, addr, Binary(Vec::new()))
}
