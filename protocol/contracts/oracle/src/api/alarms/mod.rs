use std::result::Result as StdResult;

use serde::{Deserialize, Serialize};

use currencies::LeaseGroup;
use currency::{Currency, Group};
use finance::price::base::BasePrice;
use thiserror::Error;

use sdk::{
    cosmwasm_std::StdError as CosmWasmError,
    schemars::{self, JsonSchema},
};

mod unchecked;

pub type AlarmCurrencies = LeaseGroup;

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "Alarm<AlarmCurrencies, Lpn>: Serialize")
)]
pub enum ExecuteMsg<Lpn>
where
    Lpn: Currency,
{
    AddPriceAlarm { alarm: Alarm<AlarmCurrencies, Lpn> },
}

pub type Result<T> = StdResult<T, Error>;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Oracle; Stub] Failed to add alarm! Cause: {0}")]
    StubAddAlarm(CosmWasmError),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
#[serde(
    try_from = "unchecked::Alarm<G, Lpn>",
    bound(serialize = "BasePrice<G, Lpn>: Serialize")
)]
/// `G` and `LpnG` should not overlap
pub struct Alarm<G, Lpn>
where
    G: Group,
    Lpn: Currency,
{
    below: BasePrice<G, Lpn>,
    above: Option<BasePrice<G, Lpn>>,
}

impl<G, Lpn> Alarm<G, Lpn>
where
    G: Group,
    Lpn: Currency,
{
    // TODO take Price<C, Q>-es instead
    pub fn new<P>(below: P, above_or_equal: Option<P>) -> Alarm<G, Lpn>
    where
        P: Into<BasePrice<G, Lpn>>,
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

    fn invariant_held(&self) -> StdResult<(), AlarmError> {
        if let Some(above_or_equal) = &self.above {
            if self.below.base_ticker() != above_or_equal.base_ticker() {
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

impl<G, Lpn> From<Alarm<G, Lpn>> for (BasePrice<G, Lpn>, Option<BasePrice<G, Lpn>>)
where
    G: Group,
    Lpn: Currency,
{
    fn from(value: Alarm<G, Lpn>) -> Self {
        (value.below, value.above)
    }
}

#[derive(Error, Debug, PartialEq)]
#[error("[PriceAlarms] {0}")]
pub struct AlarmError(&'static str);

#[cfg(test)]
mod test {
    use serde::Serialize;
    use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

    use currencies::{
        test::{LpnC, PaymentC5, PaymentC6, PaymentC7},
        PaymentGroup,
    };
    use currency::{Currency, Group};
    use finance::{
        coin::{Coin, CoinDTO},
        price::{self, base::BasePrice, Price},
    };
    use sdk::cosmwasm_std::{from_json, to_json_vec, StdError};

    use super::{Alarm, AlarmCurrencies as AssetG};

    #[test]
    fn below_price_ok() {
        let exp_price = price::total_of(Coin::<PaymentC6>::new(10)).is(Coin::<LpnC>::new(10));
        let exp_res = Ok(Alarm::new(exp_price, None));
        assert_eq!(exp_res, from_below(exp_price));
    }

    #[test]
    fn below_price_err() {
        assert_err(
            from_both_str_impl(
                alarm_half_coins_to_json(
                    AlarmPrice::Below,
                    Coin::<PaymentC5>::new(5),
                    Coin::<LpnC>::new(0),
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
                    Coin::<LpnC>::new(5),
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
            Coin::<LpnC>::new(15),
        );
        assert_err(
            from_both_str_impl(
                &below,
                Some(&alarm_half_coins_to_json(
                    AlarmPrice::Above,
                    Coin::<PaymentC5>::new(5),
                    Coin::<LpnC>::new(0),
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
                    Coin::<LpnC>::new(5),
                )),
            ),
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
            "pretending to be ticker of a currency pertaining to the lease group",
        );
        assert_err(
            from_both(below.inv(), above),
            "pretending to be ticker of a currency pertaining to the lease group",
        );
        assert_err(
            from_both(below, below_extra.inv()),
            "above alarm and below alarm currencies",
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
    fn assert_err<G, Lpn>(r: Result<Alarm<G, Lpn>, StdError>, msg: &str)
    where
        G: Group + Debug,
        Lpn: Currency,
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

    fn from_below<C1, Q1>(below: Price<C1, Q1>) -> Result<Alarm<AssetG, Lpn>, StdError>
    where
        C1: Currency + Serialize,
        Q1: Currency + Serialize,
    {
        from_both_impl::<_, C1, _, Q1>(below, None)
    }

    fn from_both<C1, C2, Q1, Q2>(
        below: Price<C1, Q1>,
        above: Price<C2, Q2>,
    ) -> Result<Alarm<AssetG, Lpn>, StdError>
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
    ) -> Result<Alarm<AssetG, Lpn>, StdError>
    where
        C1: Currency,
        C2: Currency,
        Q1: Currency,
        Q2: Currency,
    {
        let above_str = above.map(|above| alarm_half_to_json(AlarmPrice::Above, above));
        let below_str = alarm_half_to_json(AlarmPrice::Below, below);
        from_both_str_impl(below_str, above_str)
    }

    fn from_both_str_impl<Str1, Str2>(
        below: Str1,
        above: Option<Str2>,
    ) -> Result<Alarm<AssetG, Lpn>, StdError>
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
        C: Currency,
        Q: Currency,
    {
        let base_price = BasePrice::<PaymentGroup, Q>::from(price);
        alarm_half_to_json_str(price_type, &as_json(&base_price))
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
            as_json(&CoinDTO::<PaymentGroup>::from(amount)),
            as_json(&CoinDTO::<PaymentGroup>::from(amount_quote))
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
