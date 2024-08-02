use std::marker::PhantomData;

use currency::{
    AnyVisitor, AnyVisitorResult, Currency, CurrencyDTO, DexSymbols, Group, GroupVisit as _,
    MemberOf,
};
use dex::swap::Error;
use finance::coin::{Amount, Coin, CoinDTO, NonZeroAmount};
use oracle::api::swap::SwapPath;
use sdk::cosmos_sdk_proto::Any;
#[cfg(test)]
pub(crate) use tests::validate_a_response;

#[cfg(test)]
mod tests;

pub trait ExactAmountInSkel {
    fn parse_request<GIn, GSwap>(request: Any) -> SwapRequest<GIn, GSwap>
    where
        GIn: Group + MemberOf<GSwap>,
        GSwap: Group;

    fn build_response(amount_out: Amount) -> Any;
}

pub struct SwapRequest<GIn, GSwap>
where
    GIn: Group,
    GSwap: Group,
{
    pub token_in: CoinDTO<GIn>,
    pub swap_path: SwapPath<GSwap>,
}

pub(crate) fn parse_dex_token<G>(amount: &str, denom: &str) -> finance::coin::CoinDTO<G>
where
    G: currency::Group,
{
    currency::DexSymbols::visit_any(
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

pub(crate) fn from_dex_symbol<G>(ticker: &str) -> dex::swap::Result<CurrencyDTO<G>>
where
    G: Group,
{
    struct TypeToCurrency<G>(PhantomData<G>);
    impl<G> AnyVisitor<G> for TypeToCurrency<G>
    where
        G: Group,
    {
        type VisitorG = G;
        type Output = CurrencyDTO<G>;

        type Error = Error;

        fn on<C>(self) -> AnyVisitorResult<G, Self>
        where
            C: Currency + MemberOf<G>,
        {
            Ok(currency::dto::<C, G>())
        }
    }
    DexSymbols::visit_any(ticker, TypeToCurrency(PhantomData)).map_err(Error::from)
}

#[cold]
pub(crate) fn pattern_match_else(message_name: &str) -> ! {
    unimplemented!(
        r#"Expected "{message_name}" message symmetric to the one built by the "build_request" method!"#
    );
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
    type VisitorG = G;

    type Output = CoinDTO<G>;
    type Error = currency::error::Error;

    fn on<C>(self) -> AnyVisitorResult<G, Self>
    where
        C: Currency + MemberOf<G>,
    {
        Ok(Coin::<C>::new(self.amount).into())
    }
}
