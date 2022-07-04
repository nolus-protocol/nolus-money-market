use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    fractionable::Percentable,
    percent::Percent,
    ratio::Rational,
};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Liability {
    /// The initial percentage of the amount due versus the locked collateral
    /// init_percent > 0
    init_percent: Percent,
    /// The healty percentage of the amount due versus the locked collateral
    /// healthy_percent >= init_percent
    healthy_percent: Percent,
    /// The maximum percentage of the amount due versus the locked collateral
    /// max_percent > healthy_percent
    max_percent: Percent,
    /// At what time cadence to recalculate the liability
    /// recalc_secs >= 3600
    recalc_secs: u32,
}

const SECS_IN_HOUR: u32 = 60 * 60; // TODO move to a duration lib?

impl Liability {
    pub fn new(
        init_percent: Percent,
        delta_to_healthy_percent: Percent,
        delta_to_max_percent: Percent,
        recalc_hours: u16,
    ) -> Self {
        assert!(init_percent > Percent::ZERO);
        assert!(delta_to_max_percent > Percent::ZERO);
        assert!(
            init_percent.checked_add(delta_to_healthy_percent).is_ok(),
            "healthy percent overflow"
        );
        let healthy_percent = init_percent + delta_to_healthy_percent;

        assert!(
            healthy_percent.checked_add(delta_to_max_percent).is_ok(),
            "max percent overflow"
        );
        let max_percent = healthy_percent + delta_to_max_percent;
        assert!(recalc_hours > 0);

        let obj = Self {
            init_percent,
            healthy_percent,
            max_percent,
            recalc_secs: u32::from(recalc_hours) * SECS_IN_HOUR,
        };
        debug_assert!(obj.invariant_held().is_ok());
        obj
    }

    pub fn invariant_held(&self) -> Result<()> {
        // TODO restrict further the accepted percents to 100 since there is no much sense of having no borrow
        if self.init_percent > Percent::ZERO
            && self.healthy_percent >= self.init_percent
            && self.max_percent > self.healthy_percent
            && self.recalc_secs >= SECS_IN_HOUR
        {
            Result::Ok(())
        } else {
            Result::Err(Error::broken_invariant_err::<Liability>())
        }
    }

    pub fn init_borrow_amount<P>(&self, downpayment: P) -> P
    where
        P: Percentable,
    {
        use crate::fraction::Fraction;
        debug_assert!(self.init_percent < Percent::HUNDRED);

        // borrow = init%.of(borrow + downpayment)
        // (100% - init%).of(borrow) = init%.of(downpayment)
        // borrow = init% / (100% - init%) * downpayment
        let ratio = Rational::new(self.init_percent, Percent::HUNDRED - self.init_percent);
        ratio.of(downpayment)
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::from_slice;

    use crate::{coin::Coin, currency::Usdc, error::Error, percent::Percent};

    use super::{Liability, SECS_IN_HOUR};

    #[test]
    fn new_valid() {
        let obj = Liability::new(
            Percent::from_percent(10),
            Percent::from_percent(0),
            Percent::from_percent(5),
            20,
        );
        assert_eq!(
            Liability {
                init_percent: Percent::from_percent(10),
                healthy_percent: Percent::from_percent(10),
                max_percent: Percent::from_percent(15),
                recalc_secs: 20 * SECS_IN_HOUR,
            },
            obj,
        );
    }

    #[test]
    fn new_edge_case() {
        let obj = Liability::new(
            Percent::from_percent(1),
            Percent::from_percent(0),
            Percent::from_percent(1),
            1,
        );
        assert_eq!(
            Liability {
                init_percent: Percent::from_percent(1),
                healthy_percent: Percent::from_percent(1),
                max_percent: Percent::from_percent(2),
                recalc_secs: SECS_IN_HOUR,
            },
            obj,
        );
    }

    #[test]
    #[should_panic]
    fn new_invalid_init_percent() {
        Liability::new(
            Percent::from_percent(0),
            Percent::from_percent(0),
            Percent::from_percent(1),
            1,
        );
    }

    #[test]
    #[should_panic]
    fn new_overflow_healthy_percent() {
        Liability::new(
            Percent::from_percent(45),
            Percent::from_permille(u32::MAX - 450 + 1),
            Percent::from_percent(1),
            1,
        );
    }

    #[test]
    #[should_panic]
    fn new_invalid_delta_max_percent() {
        Liability::new(
            Percent::from_percent(10),
            Percent::from_percent(5),
            Percent::from_percent(0),
            1,
        );
    }

    #[test]
    #[should_panic]
    fn new_overflow_max_percent() {
        Liability::new(
            Percent::from_permille(10),
            Percent::from_permille(5),
            Percent::from_permille(u32::MAX - 10 - 5 + 1),
            1,
        );
    }

    #[test]
    #[should_panic]
    fn new_invalid_recalc_hours() {
        Liability::new(
            Percent::from_percent(10),
            Percent::from_percent(5),
            Percent::from_percent(10),
            0,
        );
    }

    #[test]
    fn deserialize_invalid_state() {
        let deserialized: Liability = from_slice(
            br#"{"init_percent":40,"healthy_percent":30,"max_percent":20,"recalc_secs":36000}"#,
        )
        .unwrap();
        assert_eq!(
            Error::broken_invariant_err::<Liability>(),
            deserialized.invariant_held().unwrap_err()
        );
    }

    fn test_init_borrow_amount(d: u128, p: u16, exp: u128) {
        use crate::fraction::Fraction;
        type Currency = Usdc;
        let downpayment = Coin::<Currency>::new(d);
        let percent = Percent::from_percent(p);
        let calculated = Liability {
            init_percent: percent,
            healthy_percent: Percent::from_percent(99),
            max_percent: Percent::from_percent(100),
            recalc_secs: 20000,
        }
        .init_borrow_amount(downpayment);
        assert_eq!(Coin::<Currency>::new(exp), calculated);
        assert_eq!(calculated, percent.of(downpayment + calculated));
    }

    #[test]
    fn init_borrow() {
        test_init_borrow_amount(1000, 10, 111);
        test_init_borrow_amount(1, 10, 0);
        test_init_borrow_amount(1000, 99, 990 * 100);
        test_init_borrow_amount(10, 65, 18);
        test_init_borrow_amount(1, 65, 1);
        test_init_borrow_amount(2, 65, 3);
    }
}
