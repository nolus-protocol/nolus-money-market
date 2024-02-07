use std::ops::Deref;

use finance::coin::Amount;
use sdk::{
    cosmos_sdk_proto::traits::Message,
    cosmwasm_std::{Addr, Binary, Coin as CwCoin},
    cw_multi_test::AppResponse,
    neutron_sdk::bindings::types::ProtobufAny,
};
use swap::{Impl, RequestMsg};

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
            if message.type_url == RequestMsg::TYPE_URL {
                Message::decode(message.value.as_slice()).unwrap()
            } else {
                panic!(
                    "Expected message with type URL equal to \"{expected}\"! Got \"{actual}\" instead!",
                    expected = RequestMsg::TYPE_URL,
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
    use swap::testing::ExactAmountInSkel as _;

    ibc::send_response(
        app,
        inituator_contract_addr.clone(),
        Binary(platform::trx::encode_msg_responses(
            amounts.iter().copied().map(Impl::build_response),
        )),
    )
}
