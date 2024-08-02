use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use currency::{Currency, Group, MemberOf};
use sdk::schemars::{self, JsonSchema};
use with_price::WithPrice;

use crate::{
    coin::{Coin, CoinDTO},
    error::{Error, Result as FinanceResult},
};

use super::{dto::PriceDTO, Price};

mod unchecked;
pub mod with_price;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(
    try_from = "unchecked::BasePrice<BaseG, QuoteG>",
    into = "unchecked::BasePrice<BaseG, QuoteG>",
    bound(serialize = "", deserialize = "")
)]
pub struct BasePrice<BaseG, QuoteC, QuoteG>
where
    BaseG: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    amount: CoinDTO<BaseG>,
    amount_quote: Coin<QuoteC>,
    #[serde(skip)]
    _quote_group: PhantomData<QuoteG>,
}

impl<BaseG, QuoteC, QuoteG> BasePrice<BaseG, QuoteC, QuoteG>
where
    BaseG: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    #[cfg(any(test, feature = "testing"))]
    pub fn new(amount: CoinDTO<BaseG>, amount_quote: Coin<QuoteC>) -> Self {
        Self::new_unchecked(amount, amount_quote)
    }

    fn new_checked(amount: CoinDTO<BaseG>, amount_quote: Coin<QuoteC>) -> FinanceResult<Self> {
        let res = Self::new_raw(amount, amount_quote);
        res.invariant_held().map(|()| res)
    }

    fn new_unchecked(amount: CoinDTO<BaseG>, amount_quote: Coin<QuoteC>) -> Self {
        let res = Self::new_raw(amount, amount_quote);

        debug_assert_eq!(Ok(()), res.invariant_held());
        res
    }

    fn new_raw(amount: CoinDTO<BaseG>, amount_quote: Coin<QuoteC>) -> Self {
        Self {
            amount,
            amount_quote,
            _quote_group: PhantomData,
        }
    }

    fn invariant_held(&self) -> FinanceResult<()> {
        struct InvariantCheck<PriceG> {
            price_g: PhantomData<PriceG>,
        }

        impl<PriceG, QuoteC> WithPrice<QuoteC> for InvariantCheck<PriceG>
        where
            PriceG: Group,
            QuoteC: Currency,
        {
            type PriceG = PriceG;

            type Output = ();

            type Error = Error;

            fn exec<C>(self, converted: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
            where
                C: 'static,
            {
                converted.invariant_held()
            }
        }

        with_price::execute(
            self,
            InvariantCheck {
                price_g: PhantomData::<BaseG>,
            },
        )
    }
}

//
// Price related transformations
//
impl<C, G, QuoteC, QuoteG> From<Price<C, QuoteC>> for BasePrice<G, QuoteC, QuoteG>
where
    C: Currency + MemberOf<G>,
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn from(price: Price<C, QuoteC>) -> Self {
        Self::new_unchecked(price.amount.into(), price.amount_quote)
    }
}

impl<C, G, QuoteC, QuoteG> TryFrom<BasePrice<G, QuoteC, QuoteG>> for Price<C, QuoteC>
where
    C: Currency + MemberOf<G>,
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    type Error = Error;

    fn try_from(base: BasePrice<G, QuoteC, QuoteG>) -> Result<Self, Self::Error> {
        Self::try_from(&base)
    }
}

impl<C, G, QuoteC, QuoteG> TryFrom<&BasePrice<G, QuoteC, QuoteG>> for Price<C, QuoteC>
where
    C: Currency + MemberOf<G>,
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    type Error = Error;

    fn try_from(base: &BasePrice<G, QuoteC, QuoteG>) -> Result<Self, Self::Error> {
        base.amount
            .try_into()
            .map_err(Into::into)
            .map(|amount| super::total_of(amount).is(base.amount_quote))
    }
}

//
// PriceDTO related transformations
//
impl<BaseG, QuoteC, QuoteG> From<BasePrice<BaseG, QuoteC, QuoteG>> for PriceDTO<BaseG, QuoteG>
where
    BaseG: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn from(base: BasePrice<BaseG, QuoteC, QuoteG>) -> Self {
        Self::new_unchecked(base.amount, base.amount_quote.into())
    }
}
impl<BaseG, QuoteC, QuoteG> TryFrom<PriceDTO<BaseG, QuoteG>> for BasePrice<BaseG, QuoteC, QuoteG>
where
    BaseG: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    type Error = Error;

    fn try_from(price: PriceDTO<BaseG, QuoteG>) -> Result<Self, Self::Error> {
        Coin::<QuoteC>::try_from(*(price.quote()))
            .and_then(|amount_quote| Self::new_checked(*price.base(), amount_quote))
    }
}
#[cfg(test)]
mod test_invariant {
    use currency::{
        test::{SubGroup, SubGroupTestC1, SuperGroup, SuperGroupTestC1, SuperGroupTestC2},
        Currency, Definition, Group, MemberOf,
    };
    use sdk::cosmwasm_std::{from_json, to_json_string, StdResult};

    use crate::coin::Coin;

    use super::BasePrice;

    #[test]
    #[should_panic = "zero"]
    fn base_zero() {
        new_invalid(
            Coin::<SuperGroupTestC1>::new(0),
            Coin::<SuperGroupTestC2>::new(3),
        )
    }

    #[test]
    #[should_panic = "zero"]
    fn base_zero_json() {
        let json = format!(
            r#"{{"amount": {{"amount": "0", "ticker": "{}"}}, "amount_quote": {{"amount": "3", "ticker": "{}"}}}}"#,
            SuperGroupTestC1::TICKER,
            SuperGroupTestC2::TICKER
        );

        let _loaded = load::<SuperGroup, SuperGroupTestC2, SuperGroup>(&json.into_bytes());

        #[cfg(not(debug_assertions))]
        {
            _loaded.expect("should have returned an error");
        }
    }

    #[test]
    #[should_panic = "zero"]
    fn quote_zero() {
        new_invalid(
            Coin::<SuperGroupTestC1>::new(6),
            Coin::<SuperGroupTestC2>::new(0),
        );
    }

    #[test]
    #[should_panic = "zero"]
    fn quote_zero_json() {
        let json = format!(
            r#"{{"amount": {{"amount": "6", "ticker": "{}"}}, "amount_quote": {{"amount": "0", "ticker": "{}"}}}}"#,
            SuperGroupTestC1::TICKER,
            SuperGroupTestC2::TICKER
        );

        let _loaded = load::<SuperGroup, SuperGroupTestC2, SuperGroup>(&json.into_bytes());

        #[cfg(not(debug_assertions))]
        {
            _loaded.expect("should have returned an error");
        }
    }

    #[test]
    fn test_serialize_deserialize() {
        let base_price = BasePrice::<SuperGroup, SubGroupTestC1, SubGroup>::new(
            Coin::<SuperGroupTestC2>::new(2).into(),
            Coin::<SubGroupTestC1>::new(10),
        );

        let serialized = to_json_string(&base_price).expect("Failed to serialize");
        let loaded = load::<SuperGroup, SubGroupTestC1, SubGroup>(&serialized.into_bytes())
            .expect("Failed to deserialize");
        assert_eq!(base_price, loaded);
    }

    fn new_invalid<BaseC, QuoteC>(amount: Coin<BaseC>, amount_quote: Coin<QuoteC>)
    where
        BaseC: Currency + MemberOf<SuperGroup>,
        QuoteC: Currency + MemberOf<SuperGroup>,
    {
        let _base_price: BasePrice<SuperGroup, QuoteC, SuperGroup> =
            BasePrice::new_unchecked(amount.into(), amount_quote);

        #[cfg(not(debug_assertions))]
        {
            _base_price
                .invariant_held()
                .expect("should have returned an error");
        }
    }

    fn load<G, QuoteC, QuoteG>(json: &[u8]) -> StdResult<BasePrice<G, QuoteC, QuoteG>>
    where
        G: Group,
        QuoteC: Currency + MemberOf<QuoteG>,
        QuoteG: Group,
    {
        load_with_group::<G, QuoteC, QuoteG>(json)
    }

    fn load_with_group<G, QuoteC, QuoteG>(json: &[u8]) -> StdResult<BasePrice<G, QuoteC, QuoteG>>
    where
        G: Group,
        QuoteC: Currency + MemberOf<QuoteG>,
        QuoteG: Group,
    {
        from_json::<BasePrice<G, QuoteC, QuoteG>>(json)
    }
}
