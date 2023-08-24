use std::ops::Sub;

use currency::Currency;

use crate::{
    coin::{Amount, Coin},
    duration::Duration,
    fraction::Fraction,
    fractionable::Percentable,
    percent::{Percent, Units},
    ratio::Rational,
    zero::Zero,
};

pub use self::dto::LiabilityDTO;
pub use self::level::Level;
pub use self::liquidation::{check_liability, Cause, Liquidation, Status};
pub use self::zone::Zone;

mod dto;
mod level;
mod liquidation;
mod zone;

pub const MIN_LIQ_AMOUNT: Amount = 10_000;
pub const MIN_ASSET_AMOUNT: Amount = 15_000_000;

#[derive(Copy, Clone, Debug)]
pub struct Liability<Lpn> {
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
    /// The minimum amount that triggers a liquidation
    min_liquidation: Coin<Lpn>,
    ///  The minimum amount that a lease asset should be evaluated past any partial liquidation. If not, a full liquidation is performed
    min_asset: Coin<Lpn>,
    /// At what time cadence to recalculate the liability
    ///
    /// Limitation: recalc_time >= 1 hour
    recalc_time: Duration,
}

impl<Lpn> Liability<Lpn>
where
    Lpn: Currency,
{
    pub const fn healthy_percent(&self) -> Percent {
        self.healthy
    }

    pub const fn first_liq_warn(&self) -> Percent {
        self.first_liq_warn
    }

    pub const fn second_liq_warn(&self) -> Percent {
        self.second_liq_warn
    }

    pub const fn third_liq_warn(&self) -> Percent {
        self.third_liq_warn
    }

    pub const fn max(&self) -> Percent {
        self.max
    }

    pub fn zone_of(&self, ltv: Percent) -> Zone {
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
        P: Percentable + Ord + Copy,
    {
        debug_assert!(self.initial > Percent::ZERO);
        debug_assert!(self.initial < Percent::HUNDRED);

        let default_ltd = Rational::new(self.initial, Percent::HUNDRED - self.initial);
        let default_borrow = default_ltd.of(downpayment);
        may_max_ltd
            .map(|max_ltd| max_ltd.of(downpayment))
            .map(|requested_borrow| requested_borrow.min(default_borrow))
            .unwrap_or(default_borrow)
    }

    /// Post-assert: (total_due - amount_to_liquidate) / (lease_amount - amount_to_liquidate) ~= self.healthy_percent(), if total_due < lease_amount.
    /// Otherwise, amount_to_liquidate == total_due
    pub fn amount_to_liquidate<P>(&self, lease_amount: P, total_due: P) -> P
    where
        P: Percentable + Copy + Ord + Sub<Output = P> + Zero,
    {
        if total_due < self.max.of(lease_amount) {
            return P::ZERO;
        }
        if lease_amount <= total_due {
            return lease_amount;
        }

        // from 'due - liquidation = healthy% of (lease - liquidation)' follows
        // liquidation = 100% / (100% - healthy%) of (due - healthy% of lease)
        let multiplier = Rational::new(Percent::HUNDRED, Percent::HUNDRED - self.healthy_percent());
        let extra_liability_lpn =
            total_due - total_due.min(self.healthy_percent().of(lease_amount));
        Fraction::<Units>::of(&multiplier, extra_liability_lpn)
    }
}

#[cfg(test)]
mod test {
    use currency::lpn::Usdc;

    use crate::{
        coin::{Amount, Coin},
        duration::Duration,
        fraction::Fraction,
        liability::{MIN_ASSET_AMOUNT, MIN_LIQ_AMOUNT},
        percent::{Percent, Units},
        zero::Zero,
    };

    use super::{Liability, Zone};

    pub type TestLpn = Usdc;
    pub type CoinLpn = Coin<TestLpn>;

    #[test]
    fn test_zone_of() {
        let l = Liability::<TestLpn> {
            initial: Percent::from_percent(60),
            healthy: Percent::from_percent(65),
            max: Percent::from_percent(85),
            first_liq_warn: Percent::from_permille(792),
            second_liq_warn: Percent::from_permille(815),
            third_liq_warn: Percent::from_permille(826),
            min_liquidation: CoinLpn::new(MIN_LIQ_AMOUNT),
            min_asset: CoinLpn::new(MIN_ASSET_AMOUNT),
            recalc_time: Duration::from_secs(20000),
        };
        assert_eq!(zone_of(&l, 0), Zone::no_warnings(l.first_liq_warn()));
        assert_eq!(zone_of(&l, 660), Zone::no_warnings(l.first_liq_warn()));
        assert_eq!(zone_of(&l, 791), Zone::no_warnings(l.first_liq_warn()));
        assert_eq!(
            zone_of(&l, 792),
            Zone::first(l.first_liq_warn(), l.second_liq_warn())
        );
        assert_eq!(
            zone_of(&l, 814),
            Zone::first(l.first_liq_warn(), l.second_liq_warn())
        );
        assert_eq!(
            zone_of(&l, 815),
            Zone::second(l.second_liq_warn(), l.third_liq_warn())
        );
        assert_eq!(
            zone_of(&l, 825),
            Zone::second(l.second_liq_warn(), l.third_liq_warn())
        );
        assert_eq!(zone_of(&l, 826), Zone::third(l.third_liq_warn(), l.max()));
        assert_eq!(zone_of(&l, 849), Zone::third(l.third_liq_warn(), l.max()));
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
        let liability = Liability::<TestLpn> {
            initial: Percent::from_percent(60),
            healthy: Percent::from_percent(healthy),
            max: Percent::from_percent(max),
            first_liq_warn: Percent::from_permille(860),
            second_liq_warn: Percent::from_permille(865),
            third_liq_warn: Percent::from_permille(870),
            min_liquidation: CoinLpn::new(MIN_LIQ_AMOUNT),
            min_asset: CoinLpn::new(MIN_ASSET_AMOUNT),
            recalc_time: Duration::from_secs(20000),
        };
        let lease_amount: Amount = 100;
        let healthy_amount = Percent::from_percent(healthy).of(lease_amount);
        let max_amount = Percent::from_percent(max).of(lease_amount);
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

    #[track_caller]
    fn amount_to_liquidate_int(
        liability: Liability<TestLpn>,
        lease: Amount,
        due: Amount,
        exp: Amount,
    ) {
        let liq = liability.amount_to_liquidate(lease, due);
        assert_eq!(exp, liq);
        if due.clamp(liability.max.of(lease), lease) == due {
            assert!(
                liability
                    .healthy_percent()
                    .of(lease - exp)
                    .abs_diff(due - exp)
                    <= 1,
                "Lease = {lease}, due = {due}, exp = {exp}"
            );
        }
    }

    fn zone_of(l: &Liability<TestLpn>, permilles: Units) -> Zone {
        l.zone_of(Percent::from_permille(permilles))
    }

    fn test_init_borrow_amount(d: u128, p: u16, exp: u128, max_p: Option<Percent>) {
        let downpayment = CoinLpn::new(d);
        let percent = Percent::from_percent(p);
        let calculated = Liability::<TestLpn> {
            initial: percent,
            healthy: Percent::from_percent(99),
            max: Percent::from_percent(100),
            first_liq_warn: Percent::from_permille(992),
            second_liq_warn: Percent::from_permille(995),
            third_liq_warn: Percent::from_permille(998),
            min_liquidation: CoinLpn::new(MIN_LIQ_AMOUNT),
            min_asset: CoinLpn::new(MIN_ASSET_AMOUNT),
            recalc_time: Duration::from_secs(20000),
        }
        .init_borrow_amount(downpayment, max_p);

        assert_eq!(calculated, CoinLpn::new(exp));
    }
}
