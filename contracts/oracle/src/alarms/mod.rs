use serde::{Deserialize, Serialize};
use thiserror::Error;

use marketprice::SpotPrice;
use sdk::schemars::{self, JsonSchema};

mod unchecked;

pub type Id = u64;

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
    use currency::lease::{Cro, Weth};
    use finance::coin::Coin;
    use sdk::cosmwasm_std::{from_slice, StdError};

    use super::*;

    #[test]
    fn below_price_ok() {
        let exp_price = SpotPrice::new(Coin::<Weth>::new(10).into(), Coin::<Weth>::new(10).into());
        let exp_res = Ok(Alarm::new(exp_price, None));
        assert_eq!(exp_res, from_slice(br#"{"below": {"amount": {"amount": "10", "ticker": "WETH"}, "amount_quote": {"amount": "10", "ticker": "WETH"}}}"#));
    }

    #[test]
    fn below_price_err() {
        assert_err(from_slice(br#"{"below": {"amount": {"amount": "2", "ticker": "WBTC"}, "amount_quote": {"amount": "10", "ticker": "WBTC"}}}"#), 
                                "The price should be equal to the identity if the currencies match");
        assert_err(from_slice(br#"{"below": {"amount": {"amount": "5", "ticker": "DAI"}, "amount_quote": {"amount": "0", "ticker": "DAI"}}}"#),
                                "The quote amount should not be zero");
        assert_err(from_slice(br#"{"below": {"amount": {"amount": "0", "ticker": "DAI"}, "amount_quote": {"amount": "5", "ticker": "DAI"}}}"#),
                                "The amount should not be zero");
    }

    #[test]
    fn above_price_zero() {
        assert_err(from_slice(br#"{"below": {"amount": {"amount": "0", "ticker": "ABC"}, "amount_quote": {"amount": "10", "ticker": "ABC"}}}"#),
                                "The amount should not be zero");
    }

    #[test]
    fn currencies_mismatch() {
        assert_err(from_slice(br#"{"below": {"amount": {"amount": "2", "ticker": "WBTC"}, 
                                                "amount_quote": {"amount": "10", "ticker": "CRO"}},
                                        "above": {"amount": {"amount": "2", "ticker": "WBTC"}, 
                                                "amount_quote": {"amount": "10", "ticker": "WETH"}}}"#), 
                                "Mismatch of ");
        assert_err(from_slice(br#"{"below": {"amount": {"amount": "2", "ticker": "WBTC"}, 
                                                "amount_quote": {"amount": "10", "ticker": "CRO"}},
                                        "above": {"amount": {"amount": "2", "ticker": "WETH"}, 
                                                "amount_quote": {"amount": "10", "ticker": "CRO"}}}"#),
                                "Mismatch of ");
    }

    #[test]
    fn below_not_less_than_above() {
        assert_err(from_slice(br#"{"below": {"amount": {"amount": "2", "ticker": "WBTC"}, 
                                                "amount_quote": {"amount": "10", "ticker": "CRO"}},
                                        "above": {"amount": {"amount": "2", "ticker": "WBTC"}, 
                                                "amount_quote": {"amount": "9", "ticker": "CRO"}}}"#),
                                "should be less than or equal to the above");
    }

    #[test]
    fn below_price_eq_above() {
        let price = SpotPrice::new(Coin::<Weth>::new(1).into(), Coin::<Cro>::new(10).into());
        let alarm = Alarm::new(price.clone(), Some(price));
        alarm
            .invariant_held()
            .expect("valid alarm with equal above_or_equal and below prices");
    }

    #[test]
    fn below_price_less_than_above() {
        let price_below = SpotPrice::new(Coin::<Weth>::new(1).into(), Coin::<Cro>::new(10).into());
        let price_above_or_equal =
            SpotPrice::new(Coin::<Weth>::new(1).into(), Coin::<Cro>::new(11).into());
        let alarm = Alarm::new(price_below, Some(price_above_or_equal));
        alarm.invariant_held().expect("valid alarm");
    }

    #[track_caller]
    fn assert_err(r: Result<Alarm, StdError>, msg: &str) {
        assert!(matches!(
            r,
            Err(StdError::ParseErr {
                target_type,
                msg: real_msg
            }) if target_type.contains("Alarm") && real_msg.contains(msg)
        ));
    }
}
