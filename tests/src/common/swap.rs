use std::ops::Deref;

use currencies::PaymentGroup;
use currency::{DexSymbols, Group, SymbolStatic};
use finance::coin::Amount;
use sdk::{
    api::ProtobufAny,
    cosmwasm_std::{Addr, Binary, Coin as CwCoin},
    cw_multi_test::AppResponse,
    testing,
};
use swap::{
    Impl,
    testing::{ExactAmountInSkel, SwapRequest},
};

use super::{
    ADMIN, ibc,
    test_case::{
        app::App,
        response::{RemoteChain as _, ResponseWithInterChainMsgs},
    },
};

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

pub(crate) fn expect_swap<InspectFn>(
    mut response: ResponseWithInterChainMsgs<'_, AppResponse>,
    connection_id: &str,
    ica_id: &str,
    inspect_fn: InspectFn,
) -> Vec<SwapRequest<PaymentGroup>>
where
    InspectFn: FnOnce(&AppResponse),
{
    let requests: Vec<SwapRequest<PaymentGroup>> = response
        .expect_submit_tx(connection_id, ica_id)
        .into_iter()
        .map(|msg: ProtobufAny| <Impl as ExactAmountInSkel>::parse_request(msg))
        .collect();

    assert!(!requests.is_empty());
    inspect_fn(&response.unwrap_response());
    requests
}

pub(crate) fn do_swap<I, F>(
    app: &mut App,
    initiator_contract_addr: Addr,
    ica_addr: Addr,
    requests: I,
    out_denom: SymbolStatic,
    price_f: F,
) -> ResponseWithInterChainMsgs<'_, AppResponse>
where
    I: Iterator<Item = SwapRequest<PaymentGroup>>,
    F: for<'r, 't> FnMut(Amount, DexDenom<'r>, DexDenom<'t>) -> Amount,
{
    do_swap_with::<PaymentGroup, I, F>(
        app,
        initiator_contract_addr,
        ica_addr,
        requests,
        out_denom,
        price_f,
    )
}

pub(crate) fn do_swap_with<GIn, I, F>(
    app: &mut App,
    initiator_contract_addr: Addr,
    ica_addr: Addr,
    requests: I,
    out_denom: SymbolStatic,
    mut price_f: F,
) -> ResponseWithInterChainMsgs<'_, AppResponse>
where
    GIn: Group,
    I: Iterator<Item = SwapRequest<GIn>>,
    F: for<'r, 't> FnMut(Amount, DexDenom<'r>, DexDenom<'t>) -> Amount,
{
    let amounts: Vec<Amount> = requests
        .map(|request: SwapRequest<GIn>| {
            do_swap_internal::<GIn, _>(app, ica_addr.clone(), request, out_denom, &mut price_f)
        })
        .collect();

    send_response(app, initiator_contract_addr, &amounts)
}

pub(crate) fn do_swap_with_error(
    app: &mut App,
    requester_contract: Addr,
) -> sdk::cosmwasm_std::StdResult<ResponseWithInterChainMsgs<'_, AppResponse>> {
    send_error_response(app, requester_contract)
}

fn do_swap_internal<GIn, F>(
    app: &mut App,
    ica_addr: Addr,
    request: SwapRequest<GIn>,
    out_denom: SymbolStatic,
    price_f: &mut F,
) -> Amount
where
    GIn: Group,
    F: for<'r, 't> FnMut(Amount, DexDenom<'r>, DexDenom<'t>) -> Amount,
{
    let dex_denom_in: SymbolStatic = request.token_in.currency().into_symbol::<DexSymbols<GIn>>();
    let amount_in = request.token_in.amount();

    app.send_tokens(
        ica_addr.clone(),
        testing::user(ADMIN),
        &[CwCoin::new(amount_in, dex_denom_in)],
    )
    .unwrap();

    let amount_out = price_f(amount_in, DexDenom(dex_denom_in), DexDenom(out_denom));

    app.send_tokens(
        testing::user(ADMIN),
        ica_addr,
        &[CwCoin::new(amount_out, out_denom)],
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
) -> sdk::cosmwasm_std::StdResult<ResponseWithInterChainMsgs<'_, AppResponse>> {
    ibc::send_error(app, requester_contract.clone())
}
