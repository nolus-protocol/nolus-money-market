use std::ops::Sub;

use serde::{Deserialize, Serialize};

use crate::{
    duration::Duration,
    error::{Error, Result},
    fraction::FractionLegacy,
    fractionable::FractionableLegacy,
    percent::{Percent, Percent100, Units as PercentUnits},
    ratio::SimpleFraction,
    rational::Rational,
    zero::Zero,
};

pub use self::{level::Level, zone::Zone};

mod level;
mod unchecked;
mod zone;

#[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    try_from = "unchecked::Liability"
)]
pub struct Liability {
    /// The initial percentage of the amount due versus the locked collateral
    /// initial > 0
    initial: Percent100,
    /// The healty percentage of the amount due versus the locked collateral
    /// healthy >= initial
    healthy: Percent100,
    /// The percentage above which the first liquidity warning is issued.
    first_liq_warn: Percent100,
    /// The percentage above which the second liquidity warning is issued.
    second_liq_warn: Percent100,
    /// The percentage above which the third liquidity warning is issued.
    third_liq_warn: Percent100,
    /// The maximum percentage of the amount due versus the locked collateral
    /// max > healthy
    max: Percent100,
    /// At what time cadence to recalculate the liability
    ///
    /// Limitation: recalc_time >= 1 hour
    recalc_time: Duration,
}

impl Liability {
    const MAX_TOP_BOUND: Percent100 = Percent100::HUNDRED;
    const MIN_STEP_LTV: Percent100 = Percent100::PRECISION;

    #[track_caller]
    #[cfg(any(test, feature = "testing"))]
    pub fn new(
        initial: Percent100,
        healthy: Percent100,
        first_liq_warn: Percent100,
        second_liq_warn: Percent100,
        third_liq_warn: Percent100,
        max: Percent100,
        recalc_time: Duration,
    ) -> Self {
        let obj = Self {
            initial,
            healthy,
            first_liq_warn,
            second_liq_warn,
            third_liq_warn,
            max,
            recalc_time,
        };
        debug_assert_eq!(Ok(()), obj.invariant_held());
        obj
    }

    pub const fn healthy_percent(&self) -> Percent100 {
        self.healthy
    }

    pub const fn max(&self) -> Percent100 {
        self.max
    }

    pub fn cap_to_zone(&self, ltv: Percent100) -> Percent100 {
        ltv.min(
            self.max
                .checked_sub(Self::MIN_STEP_LTV)
                .expect("Invariant violated: max must be greater than MIN_STEP_LTV"),
        )
    }

    pub fn zone_of(&self, ltv: Percent100) -> Zone {
        debug_assert!(ltv < self.max, "Ltv >= max is outside any liability zone!");

        if ltv < self.first_liq_warn {
            Zone::no_warnings(self.first_liq_warn)
        } else if ltv < self.second_liq_warn {
            Zone::first(self.first_liq_warn, self.second_liq_warn)
        } else if ltv < self.third_liq_warn {
            Zone::second(self.second_liq_warn, self.third_liq_warn)
        } else {
            Zone::third(self.third_liq_warn, self.max)
        }
    }

    pub const fn recalculation_time(&self) -> Duration {
        self.recalc_time
    }

    pub fn init_borrow_amount<P>(&self, downpayment: P, may_max_ltd: Option<Percent>) -> P
    where
        P: Copy + FractionableLegacy<PercentUnits> + Ord,
    {
        debug_assert!(self.initial > Percent100::ZERO);
        debug_assert!(self.initial < Percent100::HUNDRED);

        let default_ltd = SimpleFraction::new(self.initial, self.initial.complement());
        default_ltd
            .of(downpayment)
            .map(|default_borrow| {
                may_max_ltd
                    .and_then(|max_ltd| max_ltd.of(downpayment))
                    .map(|requested_borrow| requested_borrow.min(default_borrow))
                    .unwrap_or(default_borrow)
            })
            .expect("TODO method has to return Option")
    }

    /// Post-assert: (total_due - amount_to_liquidate) / (lease_amount - amount_to_liquidate) ~= self.healthy_percent(), if total_due < lease_amount.
    /// Otherwise, amount_to_liquidate == total_due
    pub fn amount_to_liquidate<P>(&self, lease_amount: P, total_due: P) -> P
    where
        P: Copy + FractionableLegacy<PercentUnits> + Ord + Sub<Output = P> + Zero,
    {
        if total_due < self.max.of(lease_amount) {
            return P::ZERO;
        }
        if lease_amount <= total_due {
            return lease_amount;
        }

        // from 'due - liquidation = healthy% of (lease - liquidation)' follows
        // liquidation = 100% / (100% - healthy%) of (due - healthy% of lease)
        let multiplier = SimpleFraction::new(Percent100::HUNDRED, self.healthy.complement());
        let extra_liability_lpn = total_due - total_due.min(self.healthy.of(lease_amount));
        multiplier
            .of(extra_liability_lpn)
            .expect("TODO the method has to return Option")
    }

    fn invariant_held(&self) -> Result<()> {
        check(
            self.initial > Percent100::ZERO,
            "Initial % should not be zero",
        )?;

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
        check(self.max <= Self::MAX_TOP_BOUND, "Max % should be <= 100%")?;
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
    use currency::test::SubGroupTestC10;
    use sdk::cosmwasm_std::{StdError, from_json};

    use crate::{
        coin::{Amount, Coin},
        duration::Duration,
        fraction::FractionLegacy,
        percent::{Percent, Percent100, Units},
        zero::Zero,
    };

    use super::{Liability, Zone};

    #[test]
    fn new_valid() {
        let exp = Liability {
            initial: Percent100::from_percent(10),
            healthy: Percent100::from_percent(10),
            first_liq_warn: Percent100::from_percent(12),
            second_liq_warn: Percent100::from_percent(13),
            third_liq_warn: Percent100::from_percent(14),
            max: Percent100::from_percent(15),
            recalc_time: Duration::from_hours(10),
        };
        assert_load_ok(exp, br#"{"initial":100,"healthy":100,"first_liq_warn":120,"second_liq_warn":130,"third_liq_warn":140,"max":150,"recalc_time": 36000000000000}"#);
    }

    #[test]
    fn new_edge_case() {
        let exp = Liability {
            initial: Percent100::from_percent(1),
            healthy: Percent100::from_percent(1),
            first_liq_warn: Percent100::from_permille(11),
            second_liq_warn: Percent100::from_permille(12),
            third_liq_warn: Percent100::from_permille(13),
            max: Percent100::from_permille(14),
            recalc_time: Duration::HOUR,
        };

        assert_load_ok(exp, br#"{"initial":10,"healthy":10,"first_liq_warn":11,"second_liq_warn":12,"third_liq_warn":13,
                        "max":14,"recalc_time":3600000000000}"#);
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
    fn test_zone_of() {
        let first_liquidation_warn = Percent100::from_permille(792);
        let second_liquidation_warn = Percent100::from_permille(815);
        let third_liquidation_warn = Percent100::from_permille(826);
        let max = Percent100::from_percent(85);
        let l = Liability {
            initial: Percent100::from_percent(60),
            healthy: Percent100::from_percent(65),
            first_liq_warn: first_liquidation_warn,
            second_liq_warn: second_liquidation_warn,
            third_liq_warn: third_liquidation_warn,
            max,
            recalc_time: Duration::from_secs(20000),
        };
        assert_eq!(zone_of(&l, 0), Zone::no_warnings(first_liquidation_warn));
        assert_eq!(zone_of(&l, 660), Zone::no_warnings(first_liquidation_warn));
        assert_eq!(zone_of(&l, 791), Zone::no_warnings(first_liquidation_warn));
        assert_eq!(
            zone_of(&l, 792),
            Zone::first(first_liquidation_warn, second_liquidation_warn)
        );
        assert_eq!(
            zone_of(&l, 814),
            Zone::first(first_liquidation_warn, second_liquidation_warn)
        );
        assert_eq!(
            zone_of(&l, 815),
            Zone::second(second_liquidation_warn, third_liquidation_warn)
        );
        assert_eq!(
            zone_of(&l, 825),
            Zone::second(second_liquidation_warn, third_liquidation_warn)
        );
        assert_eq!(zone_of(&l, 826), Zone::third(third_liquidation_warn, max));
        assert_eq!(zone_of(&l, 849), Zone::third(third_liquidation_warn, max));
    }

    #[test]
    fn init_borrow() {
        test_init_borrow_amount(1000, 50, 1000, None);
        test_init_borrow_amount(1, 10, 0, None);
        test_init_borrow_amount(1000, 99, 990 * 100, None);
        test_init_borrow_amount(10, 65, 18, None);
        test_init_borrow_amount(100, 60, 150, None);
        test_init_borrow_amount(250, 59, 359, None);
        test_init_borrow_amount(70, 5, 3, None);
        test_init_borrow_amount(90, 25, 30, None);
    }

    #[test]
    fn init_borrow_max_ltd() {
        test_init_borrow_amount(50000, 60, 25000, Some(Percent::from_percent(50)));
        test_init_borrow_amount(1000, 10, 100, Some(Percent::from_percent(10)));
        test_init_borrow_amount(1, 10, 0, Some(Percent::from_percent(5)));
        test_init_borrow_amount(1000, 60, 1500, Some(Percent::from_percent(190)));
        test_init_borrow_amount(4000, 55, 4800, Some(Percent::from_percent(120)));
        test_init_borrow_amount(200, 49, 192, Some(Percent::from_percent(100)));
        test_init_borrow_amount(1, 65, 0, Some(Percent::from_percent(65)));
        test_init_borrow_amount(2000, 60, 3000, Some(Percent::from_percent(250)));
        test_init_borrow_amount(300000, 65, 450000, Some(Percent::from_percent(150)));
        test_init_borrow_amount(50, 45, 40, Some(Percent::from_permille(999)));

        test_init_borrow_amount(1000, 65, 0, Some(Percent::ZERO));
    }

    #[test]
    fn amount_to_liquidate() {
        let healthy = 85;
        let max = 90;
        let liability = Liability {
            initial: Percent100::from_percent(60),
            healthy: Percent100::from_percent(healthy),
            max: Percent100::from_percent(max),
            first_liq_warn: Percent100::from_permille(860),
            second_liq_warn: Percent100::from_permille(865),
            third_liq_warn: Percent100::from_permille(870),
            recalc_time: Duration::from_secs(20000),
        };
        let lease_amount: Amount = 100;
        let healthy_amount = Percent100::from_percent(healthy).of(lease_amount);
        let max_amount = Percent100::from_percent(max).of(lease_amount);
        amount_to_liquidate_int(liability, lease_amount, Amount::ZERO, Amount::ZERO);
        amount_to_liquidate_int(liability, lease_amount, healthy_amount - 10, Amount::ZERO);
        amount_to_liquidate_int(liability, lease_amount, healthy_amount - 1, Amount::ZERO);
        amount_to_liquidate_int(liability, lease_amount, healthy_amount, Amount::ZERO);
        amount_to_liquidate_int(liability, lease_amount, healthy_amount + 1, Amount::ZERO);
        amount_to_liquidate_int(liability, lease_amount, max_amount - 1, Amount::ZERO);
        amount_to_liquidate_int(liability, lease_amount, max_amount, 33);
        amount_to_liquidate_int(liability, lease_amount, max_amount + 1, 40);
        amount_to_liquidate_int(liability, lease_amount, max_amount + 8, 86);
        amount_to_liquidate_int(liability, lease_amount, lease_amount - 1, 93);
        amount_to_liquidate_int(liability, lease_amount, lease_amount, lease_amount);
        amount_to_liquidate_int(liability, lease_amount, lease_amount + 1, lease_amount);
        amount_to_liquidate_int(liability, lease_amount, lease_amount + 10, lease_amount);
    }

    #[test]
    fn cap_to_zone() {
        const MAX: Percent100 = Percent100::from_permille(14);
        let obj = Liability {
            initial: Percent100::from_percent(1),
            healthy: Percent100::from_percent(1),
            first_liq_warn: Percent100::from_permille(11),
            second_liq_warn: Percent100::from_permille(12),
            third_liq_warn: Percent100::from_permille(13),
            max: MAX,
            recalc_time: Duration::HOUR,
        };
        assert_eq!(
            MAX - Liability::MIN_STEP_LTV - Liability::MIN_STEP_LTV,
            obj.cap_to_zone(MAX - Liability::MIN_STEP_LTV - Liability::MIN_STEP_LTV)
        );
        assert_eq!(
            MAX - Liability::MIN_STEP_LTV,
            obj.cap_to_zone(MAX - Liability::MIN_STEP_LTV)
        );
        assert_eq!(MAX - Liability::MIN_STEP_LTV, obj.cap_to_zone(MAX));
        assert_eq!(
            MAX - Liability::MIN_STEP_LTV,
            obj.cap_to_zone(MAX + Liability::MIN_STEP_LTV)
        );
    }

    #[track_caller]
    fn amount_to_liquidate_int(liability: Liability, lease: Amount, due: Amount, exp: Amount) {
        let liq = liability.amount_to_liquidate(lease, due);
        assert_eq!(exp, liq);
        if due.clamp(liability.max.of(lease), lease) == due {
            assert!(
                liability.healthy.of(lease - exp).abs_diff(due - exp) <= 1,
                "Lease = {lease}, due = {due}, exp = {exp}"
            );
        }
    }

    fn assert_load_ok(exp: Liability, json: &[u8]) {
        assert_eq!(Ok(exp), from_json::<Liability>(json));
    }

    #[track_caller]
    fn assert_load_err(json: &[u8], msg: &str) {
        assert!(matches!(
            from_json::<Liability>(json),
            Err(StdError::ParseErr {
                target_type,
                msg: real_msg,
                backtrace: _
            }) if target_type.contains("Liability") && real_msg.contains(msg)
        ));
    }

    fn zone_of(l: &Liability, permilles: Units) -> Zone {
        l.zone_of(Percent100::from_permille(permilles))
    }

    fn test_init_borrow_amount(d: u128, p: u32, exp: u128, max_p: Option<Percent>) {
        type Currency = SubGroupTestC10;

        let downpayment = Coin::<Currency>::new(d);
        let percent = Percent100::from_percent(p);
        let calculated = Liability {
            initial: percent,
            healthy: Percent100::from_percent(99),
            max: Percent100::from_percent(100),
            first_liq_warn: Percent100::from_permille(992),
            second_liq_warn: Percent100::from_permille(995),
            third_liq_warn: Percent100::from_permille(998),
            recalc_time: Duration::from_secs(20000),
        }
        .init_borrow_amount(downpayment, max_p);

        assert_eq!(calculated, Coin::<Currency>::new(exp));
    }
}
