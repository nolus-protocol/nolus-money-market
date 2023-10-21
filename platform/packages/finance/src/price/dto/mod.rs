use std::cmp::Ordering;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    coin::CoinDTO,
    error::{Error, Result as FinanceResult},
    price::Price,
};
use currency::{Currency, Group};

mod unchecked;
pub mod with_price;
pub mod with_quote;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "unchecked::PriceDTO<G, QuoteG>")]
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
    pub fn new(base: CoinDTO<G>, quote: CoinDTO<QuoteG>) -> Self {
        let res = Self {
            amount: base,
            amount_quote: quote,
        };
        debug_assert_eq!(Ok(()), res.invariant_held());
        res
    }

    pub const fn base(&self) -> &CoinDTO<G> {
        &self.amount
    }

    pub const fn quote(&self) -> &CoinDTO<QuoteG> {
        &self.amount_quote
    }

    fn invariant_held(&self) -> FinanceResult<()> {
        Self::check(!self.base().is_zero(), "The amount should not be zero")
            .and_then(|_| {
                Self::check(
                    !self.quote().is_zero(),
                    "The quote amount should not be zero",
                )
            })
            .and_then(|_| {
                Self::check(
                    self.amount.ticker() != self.amount_quote.ticker()
                        || self.amount.amount() == self.amount_quote.amount(),
                    "The price should be equal to the identity if the currencies match",
                )
            })
    }

    fn check(invariant: bool, msg: &str) -> FinanceResult<()> {
        Error::broken_invariant_if::<Self>(!invariant, msg)
    }
}

impl<G, QuoteG, C, QuoteC> From<Price<C, QuoteC>> for PriceDTO<G, QuoteG>
where
    G: Group,
    QuoteG: Group,
    C: Currency,
    QuoteC: Currency,
{
    fn from(price: Price<C, QuoteC>) -> Self {
        Self::new(price.amount.into(), price.amount_quote.into())
    }
}

impl<G, QuoteG, C, QuoteC> TryFrom<&PriceDTO<G, QuoteG>> for Price<C, QuoteC>
where
    G: Group,
    QuoteG: Group,
    C: Currency,
    QuoteC: Currency,
{
    type Error = Error;

    fn try_from(value: &PriceDTO<G, QuoteG>) -> Result<Self, Self::Error> {
        Ok(super::total_of((&value.amount).try_into()?).is((&value.amount_quote).try_into()?))
    }
}

impl<G, QuoteG, C, QuoteC> TryFrom<PriceDTO<G, QuoteG>> for Price<C, QuoteC>
where
    G: Group,
    QuoteG: Group,
    C: Currency,
    QuoteC: Currency,
{
    type Error = Error;

    fn try_from(value: PriceDTO<G, QuoteG>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl<G, QuoteG> PartialOrd for PriceDTO<G, QuoteG>
where
    G: Group,
    QuoteG: Group,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        struct Comparator<'a, G, QuoteG>
        where
            G: Group,
            QuoteG: Group,
        {
            other: &'a PriceDTO<G, QuoteG>,
        }

        impl<'a, G, QuoteG> WithPrice for Comparator<'a, G, QuoteG>
        where
            G: PartialEq + Group,
            QuoteG: Group,
        {
            type Output = Option<Ordering>;
            type Error = Error;

            fn exec<C, QuoteC>(self, lhs: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
            where
                C: Currency,
                QuoteC: Currency,
            {
                Price::<C, QuoteC>::try_from(self.other).map(|rhs| lhs.partial_cmp(&rhs))
            }
        }
        with_price::execute(self, Comparator { other })
            .expect("The currencies of both prices should match")
    }
}

pub trait WithPrice {
    type Output;
    type Error;

    fn exec<C, QuoteC>(self, _: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
    where
        C: Currency + Serialize + DeserializeOwned,
        QuoteC: Currency + Serialize + DeserializeOwned;
}

pub trait WithBase<C>
where
    C: Currency,
{
    type Output;
    type Error;

    fn exec<QuoteC>(self, _: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
    where
        QuoteC: Currency;
}

pub trait WithQuote<C>
where
    C: Currency,
{
    type Output;
    type Error;

    fn exec<Base>(self, _: Price<Base, C>) -> Result<Self::Output, Self::Error>
    where
        Base: Currency;
}

#[cfg(test)]
mod test {
    use std::cmp::Ordering;

    use crate::{
        coin::Coin,
        price::{self, dto::PriceDTO, Price},
    };
    use currency::test::{
        SubGroup, SubGroupTestC1, SuperGroup, SuperGroupTestC1, SuperGroupTestC2,
    };

    type TestPriceDTO = PriceDTO<SubGroup, SuperGroup>;

    #[test]
    fn test_cmp() {
        let p1: TestPriceDTO = price::total_of(Coin::<SubGroupTestC1>::new(20))
            .is(Coin::<SuperGroupTestC1>::new(5000))
            .into();
        assert!(p1 == p1);
        assert_eq!(Some(Ordering::Equal), p1.partial_cmp(&p1));

        let p2 = price::total_of(Coin::<SubGroupTestC1>::new(20))
            .is(Coin::<SuperGroupTestC1>::new(5001))
            .into();
        assert!(p1 < p2);

        let p3: TestPriceDTO = price::total_of(Coin::<SubGroupTestC1>::new(1000000))
            .is(Coin::<SuperGroupTestC1>::new(789456))
            .into();
        let p4 = price::total_of(Coin::<SubGroupTestC1>::new(1000000))
            .is(Coin::<SuperGroupTestC1>::new(123456))
            .into();
        assert!(p3 >= p4);

        let p5 = price::total_of(Coin::<SubGroupTestC1>::new(1000000))
            .is(Coin::<SuperGroupTestC1>::new(3456))
            .into();
        assert!(p3 >= p5);

        let p6 = price::total_of(Coin::<SubGroupTestC1>::new(1000000))
            .is(Coin::<SuperGroupTestC1>::new(3456))
            .into();
        assert!(p3 >= p6);
    }

    #[test]
    #[should_panic = "The currencies of both prices should match"]
    fn test_cmp_currencies_mismatch() {
        let p1: PriceDTO<SuperGroup, SubGroup> = Price::new(
            Coin::<SuperGroupTestC1>::new(20),
            Coin::<SuperGroupTestC2>::new(5000),
        )
        .into();
        let p2 = Price::new(
            Coin::<SuperGroupTestC1>::new(20),
            Coin::<SubGroupTestC1>::new(5000),
        )
        .into();
        let _ = p1 < p2;
    }
}

#[cfg(test)]
mod test_invariant {

    use serde::Deserialize;

    use sdk::cosmwasm_std::{from_slice, StdError, StdResult};

    use crate::coin::{Coin, CoinDTO};
    use currency::test::{SubGroup, SuperGroup, SuperGroupTestC1, SuperGroupTestC2};
    use currency::{Currency, Group};

    use super::PriceDTO;

    type TC = SubGroup;

    #[test]
    #[should_panic = "zero"]
    fn base_zero() {
        new_invalid(
            Coin::<SuperGroupTestC1>::new(0),
            Coin::<SuperGroupTestC2>::new(5),
        );
    }

    #[test]
    fn base_zero_json() {
        let json = format!(
            r#"{{"amount": {{"amount": "0", "ticker": "{}"}}, "amount_quote": {{"amount": "5", "ticker": "{}"}}}}"#,
            SuperGroupTestC1::TICKER,
            SuperGroupTestC2::TICKER
        );
        assert_err(load(&json.into_bytes()), "not be zero");
    }

    #[test]
    #[should_panic = "zero"]
    fn quote_zero() {
        new_invalid(
            Coin::<SuperGroupTestC1>::new(10),
            Coin::<SuperGroupTestC2>::new(0),
        );
    }

    #[test]
    fn quote_zero_json() {
        let json = format!(
            r#"{{"amount": {{"amount": "10", "ticker": "{}"}}, "amount_quote": {{"amount": "0", "ticker": "{}"}}}}"#,
            SuperGroupTestC1::TICKER,
            SuperGroupTestC2::TICKER
        );
        assert_err(load(&json.into_bytes()), "not be zero");
    }

    #[test]
    #[should_panic = "should be equal to the identity if the currencies match"]
    fn currencies_match() {
        new_invalid(
            Coin::<SuperGroupTestC2>::new(4),
            Coin::<SuperGroupTestC2>::new(5),
        );
    }

    #[test]
    fn currencies_match_json() {
        let json = format!(
            r#"{{"amount": {{"amount": "10", "ticker": "{}"}}, "amount_quote": {{"amount": "5", "ticker": "{}"}}}}"#,
            SuperGroupTestC1::TICKER,
            SuperGroupTestC1::TICKER
        );
        assert_err(
            load(&json.into_bytes()),
            "should be equal to the identity if the currencies match",
        );
    }

    #[test]
    fn currencies_match_ok() {
        let p = PriceDTO::<TC, TC>::new(
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
        let r = load_with_groups::<TC, SuperGroup>(br#"{"amount": {"amount": "4", "ticker": "unls"}, "amount_quote": {"amount": "5", "ticker": "udai"}}"#);
        assert_err(r, "pretending to be ticker of a currency pertaining to");
    }

    fn new_invalid<C, QuoteC>(base: Coin<C>, quote: Coin<QuoteC>)
    where
        C: Currency,
        QuoteC: Currency,
    {
        let _p = PriceDTO::<TC, TC>::new(base.into(), quote.into());
        #[cfg(not(debug_assertions))]
        {
            _p.invariant_held().expect("should have returned an error");
        }
    }

    fn load(json: &[u8]) -> StdResult<PriceDTO<TC, TC>> {
        load_with_groups::<TC, TC>(json)
    }

    fn load_with_groups<G, QuoteG>(json: &[u8]) -> StdResult<PriceDTO<G, QuoteG>>
    where
        G: Group + for<'a> Deserialize<'a>,
        QuoteG: Group + for<'a> Deserialize<'a>,
    {
        from_slice::<PriceDTO<G, QuoteG>>(json)
    }

    fn assert_err<G, QuoteG>(r: Result<PriceDTO<G, QuoteG>, StdError>, msg: &str)
    where
        G: Group,
        QuoteG: Group,
    {
        assert!(matches!(
            r,
            Err(StdError::ParseErr {
                target_type,
                msg: real_msg
            }) if target_type.contains("PriceDTO") && real_msg.contains(msg)
        ));
    }
}
