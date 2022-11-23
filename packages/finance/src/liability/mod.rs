use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    duration::Duration,
    error::{Error, Result},
    fractionable::Percentable,
    percent::Percent,
    ratio::Rational,
};

mod unchecked;

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[serde(try_from = "unchecked::Liability")]
pub struct Liability {
    /// The initial percentage of the amount due versus the locked collateral
    /// initial > 0
    initial: Percent,
    /// The healty percentage of the amount due versus the locked collateral
    /// healthy >= initial
    healthy: Percent,
    /// The percentage above which the first liquidity warning is issued.
    first_liq_warn: Percent,
    /// The percentage above which the second liquidity warning is issued.
    second_liq_warn: Percent,
    /// The percentage above which the third liquidity warning is issued.
    third_liq_warn: Percent,
    /// The maximum percentage of the amount due versus the locked collateral
    /// max > healthy
    max: Percent,
    /// At what time cadence to recalculate the liability
    ///
    /// Limitation: recalc_time >= 1 hour
    recalc_time: Duration,
}

impl Liability {
    #[track_caller]
    #[cfg(any(test, feature = "testing"))]
    pub fn new(
        initial: Percent,
        delta_to_healthy: Percent,
        delta_to_max: Percent,
        minus_delta_of_first_liq_warn: Percent,
        minus_delta_of_second_liq_warn: Percent,
        minus_delta_of_third_liq_warn: Percent,
        recalc_time: Duration,
    ) -> Self {
        let healthy = initial + delta_to_healthy;
        let max = healthy + delta_to_max;
        let third_liquidity_warning = max - minus_delta_of_third_liq_warn;
        let second_liquidity_warning = third_liquidity_warning - minus_delta_of_second_liq_warn;
        let first_liquidity_warning = second_liquidity_warning - minus_delta_of_first_liq_warn;
        let obj = Self {
            initial,
            healthy,
            max,
            first_liq_warn: first_liquidity_warning,
            second_liq_warn: second_liquidity_warning,
            third_liq_warn: third_liquidity_warning,
            recalc_time,
        };
        debug_assert_eq!(Ok(()), obj.invariant_held());
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

    fn invariant_held(&self) -> Result<()> {
        check(self.initial > Percent::ZERO, "Initial % should not be zero")?;

        check(
            self.initial <= self.healthy,
            "Initial % should be <= healthy %",
        )?;

        check(
            self.healthy < self.first_liq_warn,
            "Healthy % should be < first liquidation %",
        )?;
        check(
            self.first_liq_warn < self.second_liq_warn,
            "First liquidation % should be < second liquidation %",
        )?;
        check(
            self.second_liq_warn < self.third_liq_warn,
            "Second liquidation % should be < third liquidation %",
        )?;
        check(
            self.third_liq_warn < self.max,
            "Third liquidation % should be < max %",
        )?;
        check(self.max <= Percent::HUNDRED, "Max % should be <= 100%")?;
        check(
            self.recalc_time >= Duration::HOUR,
            "Recalculation cadence should be >= 1h",
        )?;

        Ok(())
    }
}

fn check(invariant: bool, msg: &str) -> Result<()> {
    Error::broken_invariant_if::<Liability>(!invariant, msg)
}

#[cfg(test)]
mod test {
    use sdk::cosmwasm_std::{from_slice, StdError};

    use crate::{coin::Coin, duration::Duration, percent::Percent, test::currency::Usdc};

    use super::Liability;

    #[test]
    fn new_valid() {
        let exp = Liability {
            initial: Percent::from_percent(10),
            healthy: Percent::from_percent(10),
            first_liq_warn: Percent::from_percent(12),
            second_liq_warn: Percent::from_percent(13),
            third_liq_warn: Percent::from_percent(14),
            max: Percent::from_percent(15),
            recalc_time: Duration::from_hours(10),
        };
        assert_load_ok(br#"{"initial":100,"healthy":100,"first_liq_warn":120,"second_liq_warn":130,"third_liq_warn":140,"max":150,"recalc_time": 36000000000000}"#,
        exp);
    }

    #[test]
    fn new_edge_case() {
        let exp = Liability {
            initial: Percent::from_percent(1),
            healthy: Percent::from_percent(1),
            first_liq_warn: Percent::from_permille(11),
            second_liq_warn: Percent::from_permille(12),
            third_liq_warn: Percent::from_permille(13),
            max: Percent::from_permille(14),
            recalc_time: Duration::HOUR,
        };

        assert_load_ok(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"recalc_time":3600000000000}"#, exp);
    }

    #[test]
    fn new_invalid_init_percent() {
        assert_load_err(br#"{"initial":0,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"recalc_time":3600000000000}"#, "should not be zero");
    }

    #[test]
    fn new_overflow_percent() {
        const ERR_MSG: &str = "Invalid number";

        assert_load_err(br#"{"initial":4294967296,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"recalc_time":3600000000000}"#, ERR_MSG); // u32::MAX + 1

        assert_load_err(br#"{"initial":10,"healthy":4294967296,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"recalc_time":3600000000000}"#, ERR_MSG); // u32::MAX + 1

        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":4294967296,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"recalc_time":3600000000000}"#, ERR_MSG); // u32::MAX + 1

        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":4294967296,"third_liq_warn":13,
                        "max":14,"recalc_time":3600000000000}"#, ERR_MSG); // u32::MAX + 1

        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":4294967296,
                        "max":14,"recalc_time":3600000000000}"#, ERR_MSG); // u32::MAX + 1

        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":4294967296,"recalc_time":3600000000000}"#, ERR_MSG); // u32::MAX + 1

        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"recalc_time":18446744073709551616}"#, ERR_MSG);
        // u64::MAX + 1
    }

    #[test]
    fn new_invalid_percents_relations() {
        assert_load_err(br#"{"initial":10,"healthy":9,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"recalc_time":3600000000000}"#, "<= healthy %");
        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":10,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"recalc_time":3600000000000}"#, "< first liquidation %");
        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":11,"third_liq_warn":13,
                        "max":14,"recalc_time":3600000000000}"#, "< second liquidation %");
        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":12,
                        "max":14,"recalc_time":3600000000000}"#, "< third liquidation %");
        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":13,"recalc_time":3600000000000}"#, "< max %");
    }

    #[test]
    fn new_invalid_recalc_hours() {
        assert_load_err(br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"recalc_time":3599999999999}"#, ">= 1h");
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

    fn assert_load_ok(json: &[u8], exp: Liability) {
        assert_eq!(Ok(exp), from_slice::<Liability>(json));
    }

    #[track_caller]
    fn assert_load_err(json: &[u8], msg: &str) {
        assert!(matches!(
            dbg!(from_slice::<Liability>(json)),
            Err(StdError::ParseErr {
                target_type,
                msg: real_msg
            }) if target_type.contains("Liability") && real_msg.contains(msg)
        ));
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
}
