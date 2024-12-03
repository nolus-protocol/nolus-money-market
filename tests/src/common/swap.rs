use std::ops::Deref;

use currencies::PaymentGroup;
use currency::{DexSymbols, Group, MemberOf, SymbolStatic};
use finance::coin::Amount;
use sdk::{
    cosmos_sdk_proto::Any as CosmosAny,
    cosmwasm_std::{Addr, Binary, Coin as CwCoin},
    cw_multi_test::AppResponse,
    neutron_sdk::bindings::types::ProtobufAny as NeutronAny,
    testing,
};
use swap::{
    testing::{ExactAmountInSkel, SwapRequest},
    Impl,
};

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

impl PartialEq<DexDenom<'_>> for DexDenom<'_> {
    #[inline]
    fn eq(&self, other: &DexDenom<'_>) -> bool {
        self.0 == other.0
    }
}

impl<Rhs> PartialEq<Rhs> for DexDenom<'_>
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
) -> Vec<SwapRequest<PaymentGroup, PaymentGroup>> {
    expect_swap_with::<PaymentGroup, PaymentGroup>(response, connection_id, ica_id)
}

pub(crate) fn expect_swap_with<GIn, GSwap>(
    response: &mut ResponseWithInterChainMsgs<'_, ()>,
    connection_id: &str,
    ica_id: &str,
) -> Vec<SwapRequest<GIn, GSwap>>
where
    GIn: Group + MemberOf<GSwap>,
    GSwap: Group,
{
    let requests: Vec<SwapRequest<GIn, GSwap>> = response
        .expect_submit_tx(connection_id, ica_id)
        .into_iter()
        .map(|NeutronAny { type_url, value }| {
            <Impl as ExactAmountInSkel>::parse_request::<GIn, GSwap>(CosmosAny {
                type_url,
                value: value.into(),
            })
        })
        .collect();

    assert!(!requests.is_empty());

    requests
}

pub(crate) fn do_swap<I, F>(
    app: &mut App,
    initiator_contract_addr: Addr,
    ica_addr: Addr,
    requests: I,
    price_f: F,
) -> ResponseWithInterChainMsgs<'_, AppResponse>
where
    I: Iterator<Item = SwapRequest<PaymentGroup, PaymentGroup>>,
    F: for<'r, 't> FnMut(Amount, DexDenom<'r>, DexDenom<'t>) -> Amount,
{
    do_swap_with::<PaymentGroup, PaymentGroup, I, F>(
        app,
        initiator_contract_addr,
        ica_addr,
        requests,
        price_f,
    )
}

pub(crate) fn do_swap_with<GIn, GSwap, I, F>(
    app: &mut App,
    initiator_contract_addr: Addr,
    ica_addr: Addr,
    requests: I,
    mut price_f: F,
) -> ResponseWithInterChainMsgs<'_, AppResponse>
where
    GIn: Group,
    GSwap: Group,
    I: Iterator<Item = SwapRequest<GIn, GSwap>>,
    F: for<'r, 't> FnMut(Amount, DexDenom<'r>, DexDenom<'t>) -> Amount,
{
    let amounts: Vec<Amount> = requests
        .map(|request: SwapRequest<GIn, GSwap>| {
            do_swap_internal::<GIn, GSwap, _>(app, ica_addr.clone(), request, &mut price_f)
        })
        .collect();

    send_response(app, initiator_contract_addr, &amounts)
}

pub(crate) fn do_swap_with_error(
    app: &mut App,
    requester_contract: Addr,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>> {
    send_error_response(app, requester_contract)
}

fn do_swap_internal<GIn, GSwap, F>(
    app: &mut App,
    ica_addr: Addr,
    request: SwapRequest<GIn, GSwap>,
    price_f: &mut F,
) -> Amount
where
    GIn: Group,
    GSwap: Group,
    F: for<'r, 't> FnMut(Amount, DexDenom<'r>, DexDenom<'t>) -> Amount,
{
    assert!(!request.swap_path.is_empty());

    let dex_denom_in: SymbolStatic = request.token_in.currency().into_symbol::<DexSymbols<GIn>>();
    let amount_in: u128 = request.token_in.amount();

    app.send_tokens(
        ica_addr.clone(),
        testing::user(ADMIN),
        &[CwCoin::new(amount_in, dex_denom_in)],
    )
    .unwrap();

    let (amount_out, dex_denom_out) = request.swap_path.iter().fold(
        (amount_in, dex_denom_in),
        |(amount_in, dex_denom_in), swap_target| {
            let dex_denom_out = swap_target.target.into_symbol::<DexSymbols<GSwap>>();

            let amount_out = price_f(amount_in, DexDenom(dex_denom_in), DexDenom(dex_denom_out));

            (amount_out, dex_denom_out)
        },
    );

    app.send_tokens(
        testing::user(ADMIN),
        ica_addr,
        &[CwCoin::new(amount_out, dex_denom_out)],
    )
    .unwrap();

    amount_out
}

fn send_response<'r>(
    app: &'r mut App,
    initiator_contract_addr: Addr,
    amounts: &[Amount],
) -> ResponseWithInterChainMsgs<'r, AppResponse> {
    use swap::testing::ExactAmountInSkel as _;

    ibc::send_response(
        app,
        initiator_contract_addr.clone(),
        Binary::new(platform::trx::encode_msg_responses(
            amounts.iter().copied().map(Impl::build_response),
        )),
    )
}

fn send_error_response(
    app: &mut App,
    requester_contract: Addr,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>> {
    ibc::send_error(app, requester_contract.clone())
}
