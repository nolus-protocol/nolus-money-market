use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, result::Result as StdResult};

use currency::{Currency, Group, MemberOf};
use sdk::schemars::{self, JsonSchema};

use crate::{
    coin::CoinDTO,
    error::{Error, Result},
    price::Price,
};

mod unchecked;
pub mod with_price;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    try_from = "unchecked::PriceDTO<G, QuoteG>",
    bound(serialize = "", deserialize = "")
)]
pub struct PriceDTO<G, QuoteG>
where
    G: Group,
    QuoteG: Group,
{
    amount: CoinDTO<G>,
    amount_quote: CoinDTO<QuoteG>,
}

impl<G, QuoteG> PriceDTO<G, QuoteG>
where
    G: Group,
    QuoteG: Group,
{
    pub(super) fn new_unchecked(base: CoinDTO<G>, quote: CoinDTO<QuoteG>) -> Self {
        let res = Self {
            amount: base,
            amount_quote: quote,
        };
        debug_assert!(res.invariant_held().is_ok());
        res
    }

    fn try_new(base: CoinDTO<G>, quote: CoinDTO<QuoteG>) -> Result<Self> {
        Self {
            amount: base,
            amount_quote: quote,
        }
        .invariant_held()
    }

    pub const fn base(&self) -> &CoinDTO<G> {
        &self.amount
    }

    pub const fn quote(&self) -> &CoinDTO<QuoteG> {
        &self.amount_quote
    }

    fn invariant_held(self) -> Result<Self> {
        struct InvariantCheck<G, QuoteG> {
            g: PhantomData<G>,
            quote_g: PhantomData<QuoteG>,
        }

        impl<G, QuoteG> WithPrice for InvariantCheck<G, QuoteG>
        where
            G: Group,
            QuoteG: Group,
        {
            type G = G;
            type QuoteG = QuoteG;

            type Output = ();

            type Error = Error;

            fn exec<C, QuoteC>(self, converted: Price<C, QuoteC>) -> Result<Self::Output>
            where
                C: Currency + MemberOf<G>,
                QuoteC: Currency + MemberOf<QuoteG>,
            {
                converted.invariant_held()
            }
        }

        with_price::execute_with_coins(
            self.amount,
            self.amount_quote,
            InvariantCheck {
                g: PhantomData::<G>,
                quote_g: PhantomData::<QuoteG>,
            },
        )
        .map(|()| self)
    }

    /// Intended in scenarios when the currencies are known in advance.
    fn as_specific<C, QuoteC>(&self) -> Price<C, QuoteC>
    where
        C: Currency + MemberOf<G>,
        QuoteC: Currency + MemberOf<QuoteG>,
    {
        super::total_of(self.amount.as_specific()).is(self.amount_quote.as_specific())
    }

    fn of_currencies<C, QuoteC>(&self) -> Result<()>
    where
        C: Currency + MemberOf<G>,
        QuoteC: Currency + MemberOf<QuoteG>,
    {
        self.amount
            .of_currency::<C>()
            .and_then(|()| self.amount_quote.of_currency::<QuoteC>())
            .map_err(Into::into)
    }
}

impl<G, QuoteG, C, QuoteC> From<Price<C, QuoteC>> for PriceDTO<G, QuoteG>
where
    G: Group,
    QuoteG: Group,
    C: Currency + MemberOf<G>,
    QuoteC: Currency + MemberOf<QuoteG>,
{
    fn from(price: Price<C, QuoteC>) -> Self {
        Self::new_unchecked(price.amount.into(), price.amount_quote.into())
    }
}

impl<G, QuoteG, C, QuoteC> TryFrom<PriceDTO<G, QuoteG>> for Price<C, QuoteC>
where
    G: Group,
    QuoteG: Group,
    C: Currency + MemberOf<G>,
    QuoteC: Currency + MemberOf<QuoteG>,
{
    type Error = Error;

    fn try_from(price: PriceDTO<G, QuoteG>) -> StdResult<Self, Self::Error> {
        Self::try_from(&price)
    }
}

impl<G, QuoteG, C, QuoteC> TryFrom<&PriceDTO<G, QuoteG>> for Price<C, QuoteC>
where
    G: Group,
    QuoteG: Group,
    C: Currency + MemberOf<G>,
    QuoteC: Currency + MemberOf<QuoteG>,
{
    type Error = Error;

    fn try_from(price: &PriceDTO<G, QuoteG>) -> StdResult<Self, Self::Error> {
        price
            .of_currencies::<C, QuoteC>()
            .map(|()| price.as_specific())
    }
}

pub trait WithPrice {
    type G: Group;
    type QuoteG: Group;

    type Output;
    type Error;

    fn exec<C, QuoteC>(self, _: Price<C, QuoteC>) -> StdResult<Self::Output, Self::Error>
    where
        C: Currency + MemberOf<Self::G>,
        QuoteC: Currency + MemberOf<Self::QuoteG>;
}

pub trait WithBase<C>
where
    C: Currency,
{
    type Output;
    type Error;

    fn exec<QuoteC>(self, _: Price<C, QuoteC>) -> StdResult<Self::Output, Self::Error>
    where
        QuoteC: Currency;
}

#[cfg(test)]
mod test_invariant {

    use currency::test::{SubGroup, SuperGroup, SuperGroupTestC1, SuperGroupTestC2};
    use currency::{Currency, Definition, Group, MemberOf};
    use sdk::cosmwasm_std::{from_json, StdError as CWError, StdResult as CWResult};

    use crate::{
        coin::{Coin, CoinDTO},
        error::Result,
    };

    use super::PriceDTO;

    type TC = SuperGroup;

    #[test]
    fn base_zero() {
        assert_err(
            new_invalid(
                Coin::<SuperGroupTestC1>::new(0),
                Coin::<SuperGroupTestC2>::new(5),
            ),
            "zero",
        );
    }

    #[test]
    fn base_zero_json() {
        let json = format!(
            r#"{{"amount": {{"amount": "0", "ticker": "{}"}}, "amount_quote": {{"amount": "5", "ticker": "{}"}}}}"#,
            SuperGroupTestC1::TICKER,
            SuperGroupTestC2::TICKER
        );
        assert_load_err(load(&json.into_bytes()), "not be zero");
    }

    #[test]
    fn quote_zero() {
        assert_err(
            new_invalid(
                Coin::<SuperGroupTestC1>::new(10),
                Coin::<SuperGroupTestC2>::new(0),
            ),
            "zero",
        )
    }

    #[test]
    fn quote_zero_json() {
        let json = format!(
            r#"{{"amount": {{"amount": "10", "ticker": "{}"}}, "amount_quote": {{"amount": "0", "ticker": "{}"}}}}"#,
            SuperGroupTestC1::TICKER,
            SuperGroupTestC2::TICKER
        );
        assert_load_err(load(&json.into_bytes()), "not be zero");
    }

    #[test]
    fn currencies_match() {
        assert_err(
            new_invalid(
                Coin::<SuperGroupTestC2>::new(4),
                Coin::<SuperGroupTestC2>::new(5),
            ),
            "should be equal to the identity if the currencies match",
        );
    }

    #[test]
    fn currencies_match_json() {
        let json = format!(
            r#"{{"amount": {{"amount": "10", "ticker": "{}"}}, "amount_quote": {{"amount": "5", "ticker": "{}"}}}}"#,
            SuperGroupTestC1::TICKER,
            SuperGroupTestC1::TICKER
        );
        assert_load_err(
            load(&json.into_bytes()),
            "should be equal to the identity if the currencies match",
        );
    }

    #[test]
    fn currencies_match_ok() {
        let p = PriceDTO::<TC, TC>::new_unchecked(
            Coin::<SuperGroupTestC2>::new(4).into(),
            Coin::<SuperGroupTestC2>::new(4).into(),
        );
        assert_eq!(
            &CoinDTO::<TC>::from(Coin::<SuperGroupTestC2>::new(4)),
            p.base()
        );
    }

    #[test]
    fn currencies_match_ok_json() {
        let json = format!(
            r#"{{"amount": {{"amount": "4", "ticker": "{}"}}, "amount_quote": {{"amount": "4", "ticker": "{}"}}}}"#,
            SuperGroupTestC1::TICKER,
            SuperGroupTestC1::TICKER
        );
        assert_eq!(
            load(&json.into_bytes()).unwrap().base(),
            &CoinDTO::<TC>::from(Coin::<SuperGroupTestC1>::new(4)),
        );
    }

    #[test]
    fn group_mismatch_json() {
        let r = load_with_groups::<TC, SubGroup>(&format!(
            r#"{{"amount": {{"amount": "4", "ticker": "{}"}}, "amount_quote": {{"amount": "5", "ticker": "{}"}}}}"#,
            SuperGroupTestC1::TICKER,
            SuperGroupTestC2::TICKER
        ).into_bytes());
        assert_load_err(r, "pretending to be ticker of a currency pertaining to");
    }

    fn new_invalid<C, QuoteC>(base: Coin<C>, quote: Coin<QuoteC>) -> Result<PriceDTO<TC, TC>>
    where
        C: Currency + MemberOf<TC>,
        QuoteC: Currency + MemberOf<TC>,
    {
        PriceDTO::<TC, TC>::try_new(base.into(), quote.into())
    }

    fn load(json: &[u8]) -> CWResult<PriceDTO<TC, TC>> {
        load_with_groups::<TC, TC>(json)
    }

    fn load_with_groups<G, QuoteG>(json: &[u8]) -> CWResult<PriceDTO<G, QuoteG>>
    where
        G: Group,
        QuoteG: Group,
    {
        from_json::<PriceDTO<G, QuoteG>>(json)
    }

    fn assert_load_err<G, QuoteG>(r: CWResult<PriceDTO<G, QuoteG>>, msg: &str)
    where
        G: Group,
        QuoteG: Group,
    {
        assert!(matches!(
            r,
            Err(CWError::ParseErr {
                target_type,
                msg: real_msg
            }) if target_type.contains("PriceDTO") && real_msg.contains(msg)
        ));
    }

    fn assert_err<G, QuoteG>(r: Result<PriceDTO<G, QuoteG>>, msg: &str)
    where
        G: Group,
        QuoteG: Group,
    {
        assert!(r.expect_err("expected an error").to_string().contains(msg));
    }
}
