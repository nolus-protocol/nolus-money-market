use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::SpotPrice;

use self::errors::AlarmError;

pub mod errors;
pub mod price;
mod unchecked;

pub type Id = u64;

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteAlarmMsg {
    PriceAlarm(),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
#[serde(try_from = "unchecked::Alarm")]
pub struct Alarm {
    below: SpotPrice,
    above: Option<SpotPrice>,
}

impl Alarm {
    pub fn new<P>(below: P, above: Option<P>) -> Alarm
    where
        P: Into<SpotPrice>,
    {
        let below = below.into();
        let above = above.map(Into::into);
        let res = Self { below, above };
        debug_assert_eq!(Ok(()), res.invariant_held());
        res
    }

    pub fn should_fire(&self, current_price: &SpotPrice) -> bool {
        current_price < &self.below
            || (self.above.is_some() && current_price > self.above.as_ref().unwrap())
    }

    fn invariant_held(&self) -> Result<(), AlarmError> {
        if let Some(above) = &self.above {
            if self.below.base().ticker() != above.base().ticker()
                || self.below.quote().ticker() != above.quote().ticker()
            {
                errors::add_alarm_error("Mismatch of above alarm and below alarm currencies")?
            }
            if &self.below >= above {
                errors::add_alarm_error(
                    "The below alarm price should be less than the above alarm price",
                )?
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use currency::lease::Weth;
    use finance::coin::Coin;
    use sdk::cosmwasm_std::from_slice;

    use crate::{alarms::Alarm, SpotPrice};
    use sdk::cosmwasm_std::StdError;

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
                                "should be less than the above");
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
