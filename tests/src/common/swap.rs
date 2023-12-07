use std::ops::Deref;

use finance::coin::Amount;
use sdk::{
    cosmos_sdk_proto::traits::Message,
    cosmwasm_std::{Addr, Binary, Coin as CwCoin},
    cw_multi_test::AppResponse,
    neutron_sdk::bindings::types::ProtobufAny,
};
use swap::trx::{ExactAmountIn, RequestMsg, TypeUrl};

use super::{
    ibc,
    test_case::{
        app::App,
        response::{RemoteChain as _, ResponseWithInterChainMsgs},
    },
    ADMIN,
};

#[repr(transparent)]
#[derive(Debug, Eq)]
pub struct DexDenom<'r>(&'r str);

impl<'r, 't> PartialEq<DexDenom<'t>> for DexDenom<'r> {
    #[inline]
    fn eq(&self, other: &DexDenom<'t>) -> bool {
        self.0 == other.0
    }
}

impl<'r, Rhs> PartialEq<Rhs> for DexDenom<'r>
where
    str: PartialEq<Rhs::Target>,
    Rhs: Deref + ?Sized,
{
    #[inline]
    fn eq(&self, other: &Rhs) -> bool {
        *self.0 == **other
    }
}

pub(crate) fn expect_swap(
    response: &mut ResponseWithInterChainMsgs<'_, ()>,
    connection_id: &str,
    ica_id: &str,
) -> Vec<RequestMsg> {
    let requests: Vec<RequestMsg> = response
        .expect_submit_tx(connection_id, ica_id)
        .into_iter()
        .map(|message: ProtobufAny| {
            if message.type_url == <RequestMsg as TypeUrl>::TYPE_URL {
                Message::decode(message.value.as_slice()).unwrap()
            } else {
                panic!(
                    "Expected message with type URL equal to \"{expected}\"! Got \"{actual}\" instead!",
                    expected = <RequestMsg as TypeUrl>::TYPE_URL,
                    actual = message.type_url
                );
            }
        })
        .collect();

    assert!(!requests.is_empty());

    requests
}

pub(crate) fn do_swap<I, F>(
    app: &mut App,
    inituator_contract_addr: Addr,
    ica_addr: Addr,
    requests: I,
    mut price_f: F,
) -> ResponseWithInterChainMsgs<'_, AppResponse>
where
    I: Iterator<Item = RequestMsg>,
    F: for<'r, 't> FnMut(Amount, DexDenom<'r>, DexDenom<'t>) -> Amount,
{
    let amounts: Vec<Amount> = requests
        .map(|request: RequestMsg| do_swap_internal(app, ica_addr.clone(), request, &mut price_f))
        .collect();

    send_response(app, inituator_contract_addr, &amounts)
}

fn do_swap_internal<F>(app: &mut App, ica_addr: Addr, request: RequestMsg, price_f: F) -> Amount
where
    F: for<'r, 't> FnMut(Amount, DexDenom<'r>, DexDenom<'t>) -> Amount,
{
    {
        #[cfg(feature = "astroport")]
        do_swap_internal_astroport(app, ica_addr, request, price_f)
    }
    #[cfg(feature = "osmosis")]
    do_swap_internal_osmosis(app, ica_addr, request, price_f)
}

#[cfg(feature = "astroport")]
fn do_swap_internal_astroport<F>(
    app: &mut App,
    ica_addr: Addr,
    mut request: RequestMsg,
    mut price_f: F,
) -> Amount
where
    F: for<'r, 't> FnMut(Amount, DexDenom<'r>, DexDenom<'t>) -> Amount,
{
    use sdk::{
        cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin, cosmwasm_std::from_json,
    };

    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum AstroportMsg {
        ExecuteSwapOperations { operations: Vec<SwapOperation> },
    }

    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    enum SwapOperation {
        AstroSwap {
            offer_asset_info: AssetInfo,
            ask_asset_info: AssetInfo,
        },
    }

    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    enum AssetInfo {
        NativeToken { denom: String },
    }

    let AstroportMsg::ExecuteSwapOperations { operations } = from_json(request.msg).unwrap();

    let sent_token = {
        let ProtoCoin { denom, amount } = request.funds.pop().unwrap();

        assert!({ request.funds }.is_empty(), "More than one token sent!");

        CwCoin {
            denom,
            amount: amount.parse::<Amount>().unwrap().into(),
        }
    };

    app.send_tokens(
        ica_addr.clone(),
        Addr::unchecked(ADMIN),
        std::slice::from_ref(&sent_token),
    )
    .unwrap();

    let (amount_out, denom_out) = operations.into_iter().fold(
        (sent_token.amount.u128(), sent_token.denom),
        |(amount_in, denom_in),
         SwapOperation::AstroSwap {
             offer_asset_info:
                 AssetInfo::NativeToken {
                     denom: swap_denom_in,
                 },
             ask_asset_info:
                 AssetInfo::NativeToken {
                     denom: swap_denom_out,
                 },
         }| {
            assert_eq!(denom_in, swap_denom_in);

            (
                price_f(amount_in, &swap_denom_in, &swap_denom_out),
                swap_denom_out,
            )
        },
    );

    app.send_tokens(
        Addr::unchecked(ADMIN),
        ica_addr,
        &[CwCoin::new(amount_out, denom_out)],
    )
    .unwrap();

    amount_out
}

#[cfg(feature = "osmosis")]
fn do_swap_internal_osmosis<F>(
    app: &mut App,
    ica_addr: Addr,
    request: RequestMsg,
    price_f: F,
) -> Amount
where
    F: for<'r, 't> FnOnce(Amount, DexDenom<'r>, DexDenom<'t>) -> Amount,
{
    let token_in = request.token_in.unwrap();
    let amount_in: u128 = token_in.amount.parse().unwrap();

    app.send_tokens(
        ica_addr.clone(),
        Addr::unchecked(ADMIN),
        &[CwCoin::new(amount_in, token_in.denom.clone())],
    )
    .unwrap();

    let denom_out: &String = &request.routes.last().unwrap().token_out_denom;
    let amount_out: Amount = price_f(amount_in, DexDenom(&token_in.denom), DexDenom(denom_out));

    app.send_tokens(
        Addr::unchecked(ADMIN),
        ica_addr,
        &[CwCoin::new(amount_out, denom_out)],
    )
    .unwrap();

    amount_out
}

fn send_response<'r>(
    app: &'r mut App,
    inituator_contract_addr: Addr,
    amounts: &[Amount],
) -> ResponseWithInterChainMsgs<'r, AppResponse> {
    ibc::send_response(
        app,
        inituator_contract_addr.clone(),
        Binary(platform::trx::encode_msg_responses(
            amounts
                .iter()
                .copied()
                .map(|amount| swap::trx::exact_amount_in().build_resp(amount)),
        )),
    )
}
