use std::marker::PhantomData;

use currency::{
    AnyVisitor, AnyVisitorResult, CurrencyDTO, CurrencyDef, DexSymbols, Group, GroupVisit as _,
    MemberOf,
};
use finance::coin::{Amount, Coin, CoinDTO, NonZeroAmount};
use oracle::api::swap::SwapTarget;
use sdk::cosmos_sdk_proto::Any as CosmosAny;

#[cfg(test)]
mod tests;

pub trait ExactAmountInSkel {
    fn parse_request<GIn, GSwap>(request: CosmosAny) -> SwapRequest<GIn, GSwap>
    where
        GIn: Group + MemberOf<GSwap>,
        GSwap: Group;

    fn build_response(amount_out: Amount) -> CosmosAny;
}

pub struct SwapRequest<GIn, GSwap>
where
    GIn: Group,
    GSwap: Group,
{
    pub token_in: CoinDTO<GIn>,
    pub min_token_out: Amount,
    pub swap_path: Vec<SwapTarget<GSwap>>,
}

pub(crate) fn parse_dex_token<G>(amount: &str, denom: &str) -> CoinDTO<G>
where
    G: Group,
{
    DexSymbols::visit_any(
        denom,
        ConstructDto {
            amount: amount
                .parse::<NonZeroAmount>()
                .expect("Expected swap-in amount to be a non-zero unsigned integer!")
                .get(),
            _group: PhantomData,
        },
    )
    .expect("Expected swap-in token to be part of selected group!")
}

pub fn from_dex_symbol<G>(symbol: &str) -> dex::swap::Result<CurrencyDTO<G>>
where
    G: Group,
{
    CurrencyDTO::<G>::from_symbol_testing::<DexSymbols<G>>(symbol).map_err(Into::into)
}

#[cold]
pub(crate) fn pattern_match_else(message_name: &str) -> ! {
    unimplemented!(
        r#"Expected "{message_name}" message symmetric to the one built by the "build_request" method!"#
    );
}

#[cfg(test)]
pub(crate) fn validate_a_response(resp_base64: &str, exp_amount1: Amount, exp_amount2: Amount) {
    use base64::{Engine, engine::general_purpose};

    use dex::swap::ExactAmountIn;
    use platform::trx;

    use crate::Impl;

    let resp = general_purpose::STANDARD
        .decode(resp_base64)
        .expect("Response string should be valid Base-64 encoded data!");

    let mut resp_messages = trx::decode_msg_responses(&resp)
        .expect("Response data should contain valid response messages!")
        .fuse();

    for (expected_amount, error) in IntoIterator::into_iter([
        (exp_amount1, "Expected response for first swapped amount!"),
        (exp_amount2, "Expected response for second swapped amount!"),
    ]) {
        assert_eq!(
            <Impl as ExactAmountIn>::parse_response(&mut resp_messages).expect(error),
            expected_amount,
        );
    }

    assert_eq!(resp_messages.next(), None);
}

struct ConstructDto<G>
where
    G: Group,
{
    amount: Amount,
    _group: PhantomData<G>,
}

impl<G> AnyVisitor<G> for ConstructDto<G>
where
    G: Group,
{
    type Output = CoinDTO<G>;
    type Error = currency::error::Error;

    fn on<C>(self, _def: &CurrencyDTO<C::Group>) -> AnyVisitorResult<G, Self>
    where
        C: CurrencyDef,
        C::Group: MemberOf<G>,
    {
        Ok(Coin::<C>::new(self.amount).into())
    }
}
