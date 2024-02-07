use std::marker::PhantomData;

use serde::{de::DeserializeOwned, ser::Serialize};

use currency::{AnyVisitor, AnyVisitorResult, Currency, Group, GroupVisit as _};
use dex::swap::ExactAmountIn;
use finance::coin::{Amount, Coin, CoinDTO, NonZeroAmount};
use oracle::api::swap::SwapPath;
use sdk::cosmos_sdk_proto::Any;

#[cfg(test)]
mod tests;

pub trait ExactAmountInExt: ExactAmountIn {
    fn parse_request<GIn, GSwap>(request: Any) -> SwapRequest<GIn>
    where
        GIn: Group,
        GSwap: Group;

    fn build_response(amount_out: Amount) -> Any;
}

pub struct SwapRequest<GIn>
where
    GIn: Group,
{
    pub token_in: CoinDTO<GIn>,
    pub swap_path: SwapPath,
}

pub(crate) fn parse_dex_token<G>(amount: &str, denom: &str) -> finance::coin::CoinDTO<G>
where
    G: currency::Group,
{
    currency::DexSymbols
        .visit_any::<G, _>(
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

impl<G> AnyVisitor for ConstructDto<G>
where
    G: Group,
{
    type Output = CoinDTO<G>;
    type Error = currency::error::Error;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency + Serialize + DeserializeOwned,
    {
        Ok(Coin::<C>::new(self.amount).into())
    }
}
