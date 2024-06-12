use std::result::Result as StdResult;

use serde::{Deserialize, Serialize};

use currency::{Currency, Group};
use finance::{
    error,
    price::{
        base::BasePrice,
        with_price::{self, WithPrice},
        Price,
    },
};
use thiserror::Error;

use sdk::{
    cosmwasm_std::StdError as CosmWasmError,
    schemars::{self, JsonSchema},
};

mod unchecked;

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
#[serde(deny_unknown_fields, rename_all = "snake_case", bound(serialize = ""))]
pub enum ExecuteMsg<G, Lpn, Lpns>
where
    G: Group,
    Lpn: Currency,
    Lpns: Group,
{
    AddPriceAlarm { alarm: Alarm<G, Lpn, Lpns> },
}

pub type Result<T> = StdResult<T, Error>;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Oracle; Stub] Failed to add alarm! Cause: {0}")]
    StubAddAlarm(CosmWasmError),

    #[error("[PriceAlarms] {0}")]
    AlarmError(String),
}

impl From<error::Error> for Error {
    fn from(err: error::Error) -> Self {
        Self::AlarmError(err.to_string())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(try_from = "unchecked::Alarm<G, Lpns>", into = "unchecked::Alarm<G, Lpns>", bound(serialize = "", deserialize = ""))]
pub struct Alarm<G, Lpn, Lpns>
where
    G: Group,
    Lpn: Currency + ?Sized,
    Lpns: Group,
{
    below: BasePrice<G, Lpn, Lpns>,
    above: Option<BasePrice<G, Lpn, Lpns>>,
}

impl<G, Lpn, Lpns> Alarm<G, Lpn, Lpns>
where
    G: Group,
    Lpn: Currency,
    Lpns: Group,
{
    // TODO take Price<C, Q>-es instead
    pub fn new<P>(below: P, above_or_equal: Option<P>) -> Alarm<G, Lpn, Lpns>
    where
        P: Into<BasePrice<G, Lpn, Lpns>>,
    {
        let below = below.into();
        let above_or_equal = above_or_equal.map(Into::into);
        let res = Self {
            below,
            above: above_or_equal,
        };
        debug_assert_eq!(Ok(()), res.invariant_held());
        res
    }

    fn invariant_held(&self) -> Result<()> {
        if let Some(above_or_equal) = &self.above {
            if self.below.base_ticker() != above_or_equal.base_ticker() {
                return Err(Error::AlarmError(
                    "Mismatch of above alarm and below alarm currencies".to_string(),
                ));
            }

            struct BaseCurrencyType<'a, BaseG, QuoteC, QuoteG>
            where
                BaseG: Group,
                QuoteC: Currency,
                QuoteG: Group,
            {
                below_price: &'a BasePrice<BaseG, QuoteC, QuoteG>,
            }

            impl<'a, BaseG, QuoteC, QuoteG> WithPrice<QuoteC> for BaseCurrencyType<'a, BaseG, QuoteC, QuoteG>
            where
                BaseG: Group,
                QuoteC: Currency + ?Sized,
                QuoteG: Group,
            {
                type Output = ();

                type Error = Error;

                fn exec<C>(
                    self,
                    above_or_equal: Price<C, QuoteC>,
                ) -> StdResult<Self::Output, Self::Error>
                where
                    C: Currency + ?Sized,
                {
                    Price::<C, QuoteC>::try_from(self.below_price).map_err(Into::into).and_then(|below_price| {
                            if below_price > above_or_equal {
                                Err(Error::AlarmError("The below alarm price should be less than or equal to the above_or_equal alarm price".to_string()))
                            } else {
                                Ok(())
                            }
                        })
                }
            }
            return with_price::execute(
                above_or_equal,
                BaseCurrencyType {
                    below_price: &self.below,
                },
            )
            .map_err(Into::into);
        }
        Ok(())
    }
}

impl<G, Lpn, Lpns> From<Alarm<G, Lpn, Lpns>>
    for (BasePrice<G, Lpn, Lpns>, Option<BasePrice<G, Lpn, Lpns>>)
where
    G: Group,
    Lpn: Currency,
    Lpns: Group,
{
    fn from(value: Alarm<G, Lpn, Lpns>) -> Self {
        (value.below, value.above)
    }
}

impl<G, Lpn, Lpns> Clone for Alarm<G, Lpn, Lpns>
where
    G: Group,
    Lpn: Currency + ?Sized,
    Lpns: Group
{
    fn clone(&self) -> Self {
        Self {
            below: self.below.clone(),
            above: self.above.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use serde::Serialize;
    use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

    use currencies::{
        test::{LpnC, PaymentC5, PaymentC6, PaymentC7},
        Lpns, PaymentGroup,
    };
    use currency::{Currency, Group};
    use finance::{
        coin::{Coin, CoinDTO},
        price::{self, base::BasePrice, Price},
    };
    use sdk::cosmwasm_std::{from_json, to_json_vec, StdError};

    use crate::api::AlarmCurrencies as AssetG;

    use super::Alarm;

    #[test]
    fn below_price_ok() {
        let exp_price = price::total_of(Coin::<PaymentC6>::new(10)).is(Coin::<LpnC>::new(10));
        let exp_res = Ok(Alarm::new(exp_price, None));
        assert_eq!(exp_res, from_below(exp_price));
    }

    #[test]
    #[should_panic = " should not be zero"]
    fn below_price_err() {
        assert_err(
            alarm_half_coins_to_json(
                AlarmPrice::Below,
                Coin::<PaymentC5>::new(5),
                Coin::<LpnC>::new(0),
            )
            .and_then(|json| from_both_str_impl(json, None::<&str>)),
            "The quote amount should not be zero",
        );
        assert_err(
            alarm_half_coins_to_json(
                AlarmPrice::Below,
                Coin::<PaymentC6>::new(0),
                Coin::<LpnC>::new(5),
            )
            .and_then(|json| from_both_str_impl(json, None::<&str>)),
            "The amount should not be zero",
        );
    }

    #[test]
    #[should_panic = " should not be zero"]
    fn above_price_err() {
        let below = alarm_half_coins_to_json(
            AlarmPrice::Below,
            Coin::<PaymentC5>::new(13),
            Coin::<LpnC>::new(15),
        )
        .unwrap();

        assert_err(
            alarm_half_coins_to_json(
                AlarmPrice::Above,
                Coin::<PaymentC5>::new(5),
                Coin::<LpnC>::new(0),
            )
            .and_then(|json| from_both_str_impl(&below, Some(&json))),
            "The quote amount should not be zero",
        );
        assert_err(
            alarm_half_coins_to_json(
                AlarmPrice::Above,
                Coin::<PaymentC6>::new(0),
                Coin::<LpnC>::new(5),
            )
            .and_then(|json| from_both_str_impl(&below, Some(&json))),
            "The amount should not be zero",
        );
    }

    #[test]
    fn currencies_mismatch() {
        let below = price::total_of(Coin::<PaymentC7>::new(2)).is(Coin::<LpnC>::new(10));
        let above = price::total_of(Coin::<PaymentC6>::new(2)).is(Coin::<LpnC>::new(10));
        let below_extra = price::total_of(Coin::<PaymentC7>::new(2)).is(Coin::<PaymentC6>::new(10));

        assert_err(from_both(below, above), "Mismatch of ");
        assert_err(
            from_both(below, above.inv()),
            "Amount quote serializaion failed",
        );
        assert_err(
            from_both(below.inv(), above),
            "Amount quote serializaion failed",
        );
        assert_err(
            from_both(below, below_extra.inv()),
            "Amount quote serializaion failed",
        );
    }

    #[test]
    fn below_not_less_than_above() {
        let below = price::total_of(Coin::<PaymentC6>::new(2)).is(Coin::<LpnC>::new(10));
        let above = price::total_of(Coin::<PaymentC6>::new(2)).is(Coin::<LpnC>::new(9));

        assert_err(
            from_both(below, above),
            "should be less than or equal to the above",
        );
    }

    #[test]
    fn below_price_eq_above() {
        let price = price::total_of(Coin::<PaymentC7>::new(1)).is(Coin::<LpnC>::new(10));
        let alarm = Alarm::new(price, Some(price));
        let msg = "valid alarm with equal above_or_equal and below prices";
        alarm.invariant_held().expect(msg);
        assert_eq!(alarm, from_both(price, price).expect(msg));
    }

    #[test]
    fn below_price_less_than_above() {
        let price_below = price::total_of(Coin::<PaymentC7>::new(1)).is(Coin::<LpnC>::new(10));
        let price_above_or_equal =
            price::total_of(Coin::<PaymentC7>::new(1)).is(Coin::<LpnC>::new(11));
        let alarm = Alarm::new(price_below, Some(price_above_or_equal));
        let msg = "valid alarm";
        alarm.invariant_held().expect(msg);
        assert_eq!(
            alarm,
            from_both(price_below, price_above_or_equal).expect(msg)
        );
    }

    #[track_caller]
    fn assert_err<G, Lpn, LpnG>(r: Result<Alarm<G, Lpn, LpnG>, StdError>, msg: &str)
    where
        G: Group + Debug,
        Lpn: Currency,
        LpnG: Group + Debug,
    {
        assert!(r.is_err());
        assert!(matches!(
            dbg!(r),
            Err(StdError::ParseErr {
                target_type,
                msg: real_msg
            }) if target_type.contains("Alarm") && real_msg.contains(msg)
        ));
    }

    fn from_below<C1, Q1>(below: Price<C1, Q1>) -> Result<Alarm<AssetG, LpnC, Lpns>, StdError>
    where
        C1: Currency + Serialize,
        Q1: Currency + Serialize,
    {
        from_both_impl::<_, C1, _, Q1>(below, None)
    }

    fn from_both<C1, C2, Q1, Q2>(
        below: Price<C1, Q1>,
        above: Price<C2, Q2>,
    ) -> Result<Alarm<AssetG, LpnC, Lpns>, StdError>
    where
        C1: Currency,
        C2: Currency,
        Q1: Currency,
        Q2: Currency,
    {
        from_both_impl(below, Some(above))
    }

    fn from_both_impl<C1, C2, Q1, Q2>(
        below: Price<C1, Q1>,
        above: Option<Price<C2, Q2>>,
    ) -> Result<Alarm<AssetG, LpnC, Lpns>, StdError>
    where
        C1: Currency,
        C2: Currency,
        Q1: Currency,
        Q2: Currency,
    {
        let above_str = above
            .map(|above| alarm_half_to_json(AlarmPrice::Above, above))
            .transpose()?;
        let below_str = alarm_half_to_json(AlarmPrice::Below, below)?;
        from_both_str_impl(below_str, above_str)
    }

    fn from_both_str_impl<Str1, Str2>(
        below: Str1,
        above: Option<Str2>,
    ) -> Result<Alarm<AssetG, LpnC, Lpns>, StdError>
    where
        Str1: AsRef<str>,
        Str2: AsRef<str>,
    {
        let full_json = above.map_or_else(
            || format!(r#"{{{}}}"#, below.as_ref()),
            |above| format!(r#"{{{}, {}}}"#, below.as_ref(), above.as_ref()),
        );
        from_json(dbg!(full_json).into_bytes())
    }

    enum AlarmPrice {
        Above,
        Below,
    }
    impl Display for AlarmPrice {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            f.write_str(match self {
                AlarmPrice::Below => "below",
                AlarmPrice::Above => "above",
            })
        }
    }

    fn alarm_half_to_json<C, Q>(
        price_type: AlarmPrice,
        price: Price<C, Q>,
    ) -> Result<String, StdError>
    where
        C: Currency,
        Q: Currency,
    {
        let base_price = BasePrice::<PaymentGroup, Q, Lpns>::from(price);
        as_json(&base_price).map(|string_price| alarm_half_to_json_str(price_type, &string_price))
    }

    fn alarm_half_coins_to_json<C, Q>(
        price_type: AlarmPrice,
        amount: Coin<C>,
        amount_quote: Coin<Q>,
    ) -> Result<String, StdError>
    where
        C: Currency,
        Q: Currency,
    {
        as_json(&CoinDTO::<PaymentGroup>::from(amount)).and_then(|amount_str| {
            as_json(&CoinDTO::<PaymentGroup>::from(amount_quote)).map(|amount_quote_str| {
                let price = format!(
                    r#"{{"amount": {},"amount_quote": {}}}"#,
                    amount_str, amount_quote_str
                );
                alarm_half_to_json_str(price_type, &price)
            })
        })
    }

    fn alarm_half_to_json_str(price_type: AlarmPrice, price: &str) -> String {
        format!(r#""{}": {}"#, price_type, price)
    }

    fn as_json<S>(to_serialize: &S) -> Result<String, StdError>
    where
        S: Serialize,
    {
        to_json_vec(to_serialize)
            .and_then(|json_bytes| String::from_utf8(json_bytes).map_err(Into::into))
    }
}
