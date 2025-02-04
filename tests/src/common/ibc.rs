use std::slice;

use currencies::PaymentGroup;
use currency::{BankSymbols, CurrencyDTO, DexSymbols, Symbol, SymbolStatic};
use finance::coin::Amount;
use sdk::{
    cosmos_sdk_proto::traits::{Message, Name},
    cosmwasm_std::{Addr, Binary},
    cw_multi_test::AppResponse,
    ibc_proto::{
        cosmos::base::v1beta1::Coin as ProtobufCoin, ibc::applications::transfer::v1::MsgTransfer,
    },
    neutron_sdk::{
        bindings::types::ProtobufAny,
        sudo::msg::{RequestPacket, SudoMsg},
    },
    testing,
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
        [message] if message.type_url == MsgTransfer::type_url() => {
            Message::decode(message.value.as_slice()).unwrap()
        }
        _ => unimplemented!(),
    };

    let token: ProtobufCoin = message.token.unwrap();

    CwCoin::new(token.amount.parse::<Amount>().unwrap(), token.denom)
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
        dex_to_bank(&cw_coin.denom)
    } else {
        bank_to_dex(&cw_coin.denom)
    };

    app.send_tokens(
        sender.clone(),
        testing::user(ADMIN),
        slice::from_ref(cw_coin),
    )
    .unwrap();

    app.send_tokens(
        testing::user(ADMIN),
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

pub(super) fn send_error(
    app: &mut App,
    contract: Addr,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>> {
    app.sudo(
        contract,
        &SudoMsg::Error {
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
            details: "min output amount not fulfilled!".to_string(),
        },
    )
}

fn send_blank_response(app: &mut App, addr: Addr) -> ResponseWithInterChainMsgs<'_, AppResponse> {
    send_response(app, addr, Binary::new(vec![]))
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
