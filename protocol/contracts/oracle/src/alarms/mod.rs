use serde::{Deserialize, Serialize};
use thiserror::Error;

use marketprice::SpotPrice;
use sdk::schemars::{self, JsonSchema};

mod unchecked;

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
#[serde(try_from = "unchecked::Alarm")]
pub struct Alarm {
    below: SpotPrice,
    above: Option<SpotPrice>,
}

impl Alarm {
    pub fn new<P>(below: P, above_or_equal: Option<P>) -> Alarm
    where
        P: Into<SpotPrice>,
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

    fn invariant_held(&self) -> Result<(), AlarmError> {
        if let Some(above_or_equal) = &self.above {
            if self.below.base().ticker() != above_or_equal.base().ticker()
                || self.below.quote().ticker() != above_or_equal.quote().ticker()
            {
                Err(AlarmError(
                    "Mismatch of above alarm and below alarm currencies",
                ))?
            }
            if &self.below > above_or_equal {
                Err(AlarmError(
                    "The below alarm price should be less than or equal to the above_or_equal alarm price",
                ))?
            }
        }
        Ok(())
    }
}

impl From<Alarm> for (SpotPrice, Option<SpotPrice>) {
    fn from(value: Alarm) -> Self {
        (value.below, value.above)
    }
}

#[derive(Error, Debug, PartialEq)]
#[error("[PriceAlarms] {0}")]
pub struct AlarmError(&'static str);

#[cfg(test)]
mod test {
    use std::fmt::{Display, Formatter, Result as FmtResult};

    use currency::{
        dex::test::{PaymentC5, PaymentC6, PaymentC7},
        Currency,
    };
    use finance::{
        coin::{Coin, CoinDTO},
        price::{self, dto::PriceDTO, Price},
    };
    use sdk::cosmwasm_std::{from_json, to_json_vec, StdError};
    use swap::SwapGroup;

    use super::*;

    #[test]
    fn below_price_ok() {
        let exp_price = price::total_of(Coin::<PaymentC6>::new(10)).is(Coin::<PaymentC6>::new(10));
        let exp_res = Ok(Alarm::new(exp_price, None));
        assert_eq!(exp_res, from_below(exp_price));
    }

    #[test]
    fn below_price_err() {
        assert_err(
            from_both_str_impl(
                alarm_half_coins_to_json(
                    AlarmPrice::Below,
                    Coin::<PaymentC7>::new(2),
                    Coin::<PaymentC7>::new(10),
                ),
                None::<&str>,
            ),
            "The price should be equal to the identity if the currencies match",
        );
        assert_err(
            from_both_str_impl(
                alarm_half_coins_to_json(
                    AlarmPrice::Below,
                    Coin::<PaymentC5>::new(5),
                    Coin::<PaymentC5>::new(0),
                ),
                None::<&str>,
            ),
            "The quote amount should not be zero",
        );
        assert_err(
            from_both_str_impl(
                alarm_half_coins_to_json(
                    AlarmPrice::Below,
                    Coin::<PaymentC6>::new(0),
                    Coin::<PaymentC6>::new(5),
                ),
                None::<&str>,
            ),
            "The amount should not be zero",
        );
    }

    #[test]
    fn above_price_err() {
        let below = alarm_half_coins_to_json(
            AlarmPrice::Below,
            Coin::<PaymentC5>::new(13),
            Coin::<PaymentC6>::new(15),
        );
        assert_err(
            from_both_str_impl(
                &below,
                Some(&alarm_half_coins_to_json(
                    AlarmPrice::Above,
                    Coin::<PaymentC7>::new(2),
                    Coin::<PaymentC7>::new(10),
                )),
            ),
            "The price should be equal to the identity if the currencies match",
        );
        assert_err(
            from_both_str_impl(
                &below,
                Some(&alarm_half_coins_to_json(
                    AlarmPrice::Above,
                    Coin::<PaymentC5>::new(5),
                    Coin::<PaymentC5>::new(0),
                )),
            ),
            "The quote amount should not be zero",
        );
        assert_err(
            from_both_str_impl(
                &below,
                Some(&alarm_half_coins_to_json(
                    AlarmPrice::Above,
                    Coin::<PaymentC6>::new(0),
                    Coin::<PaymentC6>::new(5),
                )),
            ),
            "The amount should not be zero",
        );
    }

    #[test]
    fn currencies_mismatch() {
        let below = price::total_of(Coin::<PaymentC6>::new(2)).is(Coin::<PaymentC7>::new(10));
        let above = price::total_of(Coin::<PaymentC6>::new(2)).is(Coin::<PaymentC5>::new(10));

        assert_err(from_both(below, above), "Mismatch of ");
        assert_err(from_both(below, above.inv()), "Mismatch of ");
        assert_err(from_both(below.inv(), above.inv()), "Mismatch of ");
    }

    #[test]
    fn below_not_less_than_above() {
        let below = price::total_of(Coin::<PaymentC6>::new(2)).is(Coin::<PaymentC7>::new(10));
        let above = price::total_of(Coin::<PaymentC6>::new(2)).is(Coin::<PaymentC7>::new(9));

        assert_err(
            from_both(below, above),
            "should be less than or equal to the above",
        );
    }

    #[test]
    fn below_price_eq_above() {
        let price = price::total_of(Coin::<PaymentC7>::new(1)).is(Coin::<PaymentC6>::new(10));
        let alarm = Alarm::new(price, Some(price));
        let msg = "valid alarm with equal above_or_equal and below prices";
        alarm.invariant_held().expect(msg);
        assert_eq!(alarm, from_both(price, price).expect(msg));
    }

    #[test]
    fn below_price_less_than_above() {
        let price_below = price::total_of(Coin::<PaymentC7>::new(1)).is(Coin::<PaymentC6>::new(10));
        let price_above_or_equal =
            price::total_of(Coin::<PaymentC7>::new(1)).is(Coin::<PaymentC6>::new(11));
        let alarm = Alarm::new(price_below, Some(price_above_or_equal));
        let msg = "valid alarm";
        alarm.invariant_held().expect(msg);
        assert_eq!(
            alarm,
            from_both(price_below, price_above_or_equal).expect(msg)
        );
    }

    #[track_caller]
    fn assert_err(r: Result<Alarm, StdError>, msg: &str) {
        assert!(r.is_err());
        assert!(matches!(
            dbg!(r),
            Err(StdError::ParseErr {
                target_type,
                msg: real_msg
            }) if target_type.contains("Alarm") && real_msg.contains(msg)
        ));
    }

    fn from_below<C1, Q1>(below: Price<C1, Q1>) -> Result<Alarm, StdError>
    where
        C1: Currency + Serialize,
        Q1: Currency + Serialize,
    {
        from_both_impl::<_, _, C1, Q1>(below, None)
    }

    fn from_both<C1, Q1, C2, Q2>(
        below: Price<C1, Q1>,
        above: Price<C2, Q2>,
    ) -> Result<Alarm, StdError>
    where
        C1: Currency + Serialize,
        C2: Currency + Serialize,
        Q1: Currency + Serialize,
        Q2: Currency + Serialize,
    {
        from_both_impl(below, Some(above))
    }

    fn from_both_impl<C1, Q1, C2, Q2>(
        below: Price<C1, Q1>,
        above: Option<Price<C2, Q2>>,
    ) -> Result<Alarm, StdError>
    where
        C1: Currency + Serialize,
        C2: Currency + Serialize,
        Q1: Currency + Serialize,
        Q2: Currency + Serialize,
    {
        let above_str = above.map(|above| alarm_half_to_json(AlarmPrice::Above, above));
        let below_str = alarm_half_to_json(AlarmPrice::Below, below);
        from_both_str_impl(below_str, above_str)
    }

    fn from_both_str_impl<Str1, Str2>(below: Str1, above: Option<Str2>) -> Result<Alarm, StdError>
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

    fn alarm_half_to_json<C, Q>(price_type: AlarmPrice, price: Price<C, Q>) -> String
    where
        C: Currency + Serialize,
        Q: Currency + Serialize,
    {
        let price_dto = PriceDTO::<SwapGroup, SwapGroup>::from(price);
        alarm_half_to_json_str(price_type, &as_json(&price_dto))
    }

    fn alarm_half_coins_to_json<C, Q>(
        price_type: AlarmPrice,
        amount: Coin<C>,
        amount_quote: Coin<Q>,
    ) -> String
    where
        C: Currency,
        Q: Currency,
    {
        let price = format!(
            r#"{{"amount": {},"amount_quote": {}}}"#,
            as_json(&CoinDTO::<SwapGroup>::from(amount)),
            as_json(&CoinDTO::<SwapGroup>::from(amount_quote))
        );
        alarm_half_to_json_str(price_type, &price)
    }

    fn alarm_half_to_json_str(price_type: AlarmPrice, price: &str) -> String {
        format!(r#""{}": {}"#, price_type, price)
    }

    fn as_json<S>(to_serialize: &S) -> String
    where
        S: Serialize,
    {
        String::from_utf8(to_json_vec(to_serialize).unwrap()).unwrap()
    }
}
