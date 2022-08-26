use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    duration::Duration,
    error::{Error, Result},
    fractionable::Percentable,
    percent::Percent,
    ratio::Rational
};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Eq, PartialEq, JsonSchema)]
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
    /// The percentage above which the first liquidity warning is issued.
    first_liq_warn: Percent,
    /// The percentage above which the second liquidity warning is issued.
    second_liq_warn: Percent,
    /// The percentage above which the third liquidity warning is issued.
    third_liq_warn: Percent,
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
        minus_delta_of_first_liq_warn: Percent,
        minus_delta_of_second_liq_warn: Percent,
        minus_delta_of_third_liq_warn: Percent,
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

        let third_liquidity_warning = max_percent.checked_sub(minus_delta_of_third_liq_warn).expect("percentage underflow");

        let second_liquidity_warning = third_liquidity_warning.checked_sub(minus_delta_of_second_liq_warn).expect("percentage underflow");

        let first_liquidity_warning = second_liquidity_warning.checked_sub(minus_delta_of_first_liq_warn).expect("percentage underflow");

        assert!(
            second_liquidity_warning < third_liquidity_warning,
            "Third liquidity warning is below second one!",
        );

        assert!(
            first_liquidity_warning < second_liquidity_warning,
            "Second liquidity warning is below first one!",
        );

        assert!(
            healthy_percent < first_liquidity_warning,
            "First liquidity warning is below healthy percentage!",
        );

        assert!(recalc_hours > 0);

        let obj = Self {
            init_percent,
            healthy_percent,
            max_percent,
            first_liq_warn: first_liquidity_warning,
            second_liq_warn: second_liquidity_warning,
            third_liq_warn: third_liquidity_warning,
            recalc_secs: u32::from(recalc_hours) * SECS_IN_HOUR,
        };
        debug_assert!(obj.invariant_held().is_ok());
        obj
    }

    pub const fn healthy_percent(&self) -> Percent {
        self.healthy_percent
    }

    pub const fn first_liq_warn_percent(&self) -> Percent {
        self.first_liq_warn
    }

    pub const fn second_liq_warn_percent(&self) -> Percent {
        self.second_liq_warn
    }

    pub const fn third_liq_warn_percent(&self) -> Percent {
        self.third_liq_warn
    }

    pub const fn max_percent(&self) -> Percent {
        self.max_percent
    }

    pub const fn recalculation_time(&self) -> Duration {
        Duration::from_secs(self.recalc_secs)
    }

    pub fn invariant_held(&self) -> Result<()> {
        // TODO restrict further the accepted percents to 100 since there is no much sense of having no borrow
        if self.init_percent > Percent::ZERO
            && self.healthy_percent >= self.init_percent
            && self.first_liq_warn > self.healthy_percent
            && self.second_liq_warn > self.first_liq_warn
            && self.third_liq_warn > self.second_liq_warn
            && self.max_percent > self.third_liq_warn
            && self.max_percent <= Percent::HUNDRED
            && self.recalc_secs >= SECS_IN_HOUR
        {
            Ok(())
        } else {
            Err(Error::broken_invariant_err::<Liability>())
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
            Percent::from_percent(1),
            Percent::from_percent(1),
            Percent::from_percent(1),
            20,
        );
        assert_eq!(
            Liability {
                init_percent: Percent::from_percent(10),
                healthy_percent: Percent::from_percent(10),
                max_percent: Percent::from_percent(15),
                first_liq_warn: Percent::from_percent(12),
                second_liq_warn: Percent::from_percent(13),
                third_liq_warn: Percent::from_percent(14),
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
            Percent::from_permille(1),
            Percent::from_permille(1),
            Percent::from_permille(1),
            1,
        );
        assert_eq!(
            Liability {
                init_percent: Percent::from_percent(1),
                healthy_percent: Percent::from_percent(1),
                max_percent: Percent::from_percent(2),
                first_liq_warn: Percent::from_permille(17),
                second_liq_warn: Percent::from_permille(18),
                third_liq_warn: Percent::from_permille(19),
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
            Percent::from_permille(1),
            Percent::from_permille(1),
            Percent::from_permille(1),
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
            Percent::from_permille(1),
            Percent::from_permille(1),
            Percent::from_permille(1),
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
            Percent::from_permille(1),
            Percent::from_permille(1),
            Percent::from_permille(1),
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
            Percent::from_permille(1),
            Percent::from_permille(1),
            Percent::from_permille(1),
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
            Percent::from_permille(1),
            Percent::from_permille(1),
            Percent::from_permille(1),
            0,
        );
    }

    #[test]
    fn deserialize_invalid_state() {
        let deserialized: Liability = from_slice(
            br#"{"init_percent":40,"healthy_percent":30,"first_liq_warn":2,"second_liq_warn":3,"third_liq_warn":2,"max_percent":20,"recalc_secs":36000}"#,
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
            first_liq_warn: Percent::from_permille(992),
            second_liq_warn: Percent::from_permille(995),
            third_liq_warn: Percent::from_permille(998),
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
