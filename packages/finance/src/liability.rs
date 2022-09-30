use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    duration::Duration,
    error::{Error, Result},
    fractionable::Percentable,
    percent::Percent,
    ratio::Rational,
};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Liability {
    /// The initial percentage of the amount due versus the locked collateral
    /// init_percent > 0
    initial: Percent,
    /// The healty percentage of the amount due versus the locked collateral
    /// healthy_percent >= init_percent
    healthy: Percent,
    /// The maximum percentage of the amount due versus the locked collateral
    /// max_percent > healthy_percent
    max: Percent,
    /// The percentage above which the first liquidity warning is issued.
    first_liq_warn: Percent,
    /// The percentage above which the second liquidity warning is issued.
    second_liq_warn: Percent,
    /// The percentage above which the third liquidity warning is issued.
    third_liq_warn: Percent,
    /// At what time cadence to recalculate the liability
    ///
    /// Limitation: recalc_time >= 1 hour
    recalc_time: Duration,
}

impl Liability {
    pub fn new(
        initial: Percent,
        delta_to_healthy: Percent,
        delta_to_max: Percent,
        minus_delta_of_first_liq_warn: Percent,
        minus_delta_of_second_liq_warn: Percent,
        minus_delta_of_third_liq_warn: Percent,
        recalc_hours: u16,
    ) -> Self {
        assert!(initial > Percent::ZERO);
        assert!(delta_to_max > Percent::ZERO);
        assert!(
            initial.checked_add(delta_to_healthy).is_ok(),
            "healthy percent overflow"
        );
        let healthy = initial + delta_to_healthy;

        assert!(
            healthy.checked_add(delta_to_max).is_ok(),
            "max percent overflow"
        );
        let max = healthy + delta_to_max;

        let third_liquidity_warning = max
            .checked_sub(minus_delta_of_third_liq_warn)
            .expect("percentage underflow");

        let second_liquidity_warning = third_liquidity_warning
            .checked_sub(minus_delta_of_second_liq_warn)
            .expect("percentage underflow");

        let first_liquidity_warning = second_liquidity_warning
            .checked_sub(minus_delta_of_first_liq_warn)
            .expect("percentage underflow");

        assert!(
            second_liquidity_warning < third_liquidity_warning,
            "Third liquidity warning is below second one!",
        );

        assert!(
            first_liquidity_warning < second_liquidity_warning,
            "Second liquidity warning is below first one!",
        );

        assert!(
            healthy < first_liquidity_warning,
            "First liquidity warning is below healthy percentage!",
        );

        assert!(recalc_hours > 0);

        let obj = Self {
            initial,
            healthy,
            max,
            first_liq_warn: first_liquidity_warning,
            second_liq_warn: second_liquidity_warning,
            third_liq_warn: third_liquidity_warning,
            recalc_time: Duration::from_hours(recalc_hours),
        };
        debug_assert!(obj.invariant_held().is_ok());
        obj
    }

    pub const fn healthy_percent(&self) -> Percent {
        self.healthy
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
        self.max
    }

    pub const fn recalculation_time(&self) -> Duration {
        self.recalc_time
    }

    pub fn invariant_held(&self) -> Result<()> {
        // TODO restrict further the accepted percents to 100 since there is no much sense of having no borrow
        Self::broken_invariant(self.initial > Percent::ZERO, "Initial % should not be zero")?;

        Self::broken_invariant(
            self.healthy >= self.initial,
            "Healthy % should be >= initial %",
        )?;

        Self::broken_invariant(
            self.first_liq_warn > self.healthy,
            "First liquidation % should be > healthy %",
        )?;
        Self::broken_invariant(
            self.second_liq_warn > self.first_liq_warn,
            "Second liquidation % should be > first liquidation %",
        )?;
        Self::broken_invariant(
            self.third_liq_warn > self.second_liq_warn,
            "Third liquidation % should be > second liquidation %",
        )?;
        Self::broken_invariant(
            self.max > self.third_liq_warn,
            "Max % should be > third liquidation %",
        )?;
        Self::broken_invariant(self.max <= Percent::HUNDRED, "Max % should be <= 100%")?;
        Self::broken_invariant(
            self.recalc_time >= Duration::HOUR,
            "Recalculate cadence in seconds should be >= 1h",
        )?;

        Ok(())
    }

    fn broken_invariant(check: bool, msg: &str) -> Result<()> {
        if !check {
            return Err(Error::broken_invariant_err::<Liability>(msg));
        }
        Ok(())
    }

    pub fn init_borrow_amount<P>(&self, downpayment: P) -> P
    where
        P: Percentable,
    {
        use crate::fraction::Fraction;
        debug_assert!(self.initial < Percent::HUNDRED);

        // borrow = init%.of(borrow + downpayment)
        // (100% - init%).of(borrow) = init%.of(downpayment)
        // borrow = init% / (100% - init%) * downpayment
        let ratio = Rational::new(self.initial, Percent::HUNDRED - self.initial);
        ratio.of(downpayment)
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::from_slice;

    use crate::{
        coin::Coin, duration::Duration, error::Error, percent::Percent, test::currency::Usdc,
    };

    use super::Liability;

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
                initial: Percent::from_percent(10),
                healthy: Percent::from_percent(10),
                max: Percent::from_percent(15),
                first_liq_warn: Percent::from_percent(12),
                second_liq_warn: Percent::from_percent(13),
                third_liq_warn: Percent::from_percent(14),
                recalc_time: Duration::from_hours(20),
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
                initial: Percent::from_percent(1),
                healthy: Percent::from_percent(1),
                max: Percent::from_percent(2),
                first_liq_warn: Percent::from_permille(17),
                second_liq_warn: Percent::from_permille(18),
                third_liq_warn: Percent::from_permille(19),
                recalc_time: Duration::HOUR,
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
            br#"{"initial":40,"healthy":30,"first_liq_warn":2,"second_liq_warn":3,"third_liq_warn":2,"max":20,"recalc_time":36000}"#,
        )
        .unwrap();
        assert_eq!(
            Error::broken_invariant_err::<Liability>("Healthy % should be >= initial %"),
            deserialized.invariant_held().unwrap_err()
        );
    }

    fn test_init_borrow_amount(d: u128, p: u16, exp: u128) {
        use crate::fraction::Fraction;
        type Currency = Usdc;
        let downpayment = Coin::<Currency>::new(d);
        let percent = Percent::from_percent(p);
        let calculated = Liability {
            initial: percent,
            healthy: Percent::from_percent(99),
            max: Percent::from_percent(100),
            first_liq_warn: Percent::from_permille(992),
            second_liq_warn: Percent::from_permille(995),
            third_liq_warn: Percent::from_permille(998),
            recalc_time: Duration::from_secs(20000),
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
