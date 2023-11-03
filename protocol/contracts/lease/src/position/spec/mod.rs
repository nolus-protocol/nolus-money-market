use std::ops::Add;

use currency::Currency;
use finance::{
    coin::Coin,
    liability::Liability,
    percent::Percent,
    price::{self, Price},
};

use crate::error::{ContractError, ContractResult};

mod dto;

#[cfg_attr(test, derive(Debug))]
pub struct Spec<Lpn> {
    liability: Liability,
    min_asset: Coin<Lpn>,
    min_trasaction_amount: Coin<Lpn>,
}

impl<Lpn> Spec<Lpn>
where
    Lpn: Currency,
{
    pub fn new(
        liability: Liability,
        min_asset: Coin<Lpn>,
        min_trasaction_amount: Coin<Lpn>,
    ) -> Self {
        let obj = Self {
            liability,
            min_asset,
            min_trasaction_amount,
        };
        debug_assert_eq!(Ok(()), obj.invariant_held());
        obj
    }

    pub fn liability(&self) -> Liability {
        self.liability
    }

    pub fn check_trasaction_amount<Trasaction>(
        &self,
        amount: Coin<Trasaction>,
        trasaction_in_lpn: Price<Trasaction, Lpn>,
    ) -> ContractResult<()>
    where
        Trasaction: Currency,
    {
        let amount = price::total(amount, trasaction_in_lpn);

        if amount < self.min_trasaction_amount {
            Err(ContractError::InsufficientTrasactionAmount(
                self.min_trasaction_amount.into(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn check_asset_amount<Trasaction>(
        &self,
        asset_amount: Coin<Trasaction>,
        trasaction_in_lpn: Price<Trasaction, Lpn>,
    ) -> ContractResult<()>
    where
        Trasaction: Currency,
    {
        let asset_amount = price::total(asset_amount, trasaction_in_lpn);

        if asset_amount < self.min_asset {
            Err(ContractError::InsufficientAssetAmount(
                self.min_asset.into(),
            ))
        } else {
            Ok(())
        }
    }

    /// Calculate the borrow amount.
    /// Return 'error::ContractError::InsufficientTrasactionAmount' when either the downpayment
    /// or the borrow amount is with amount less than the minimum trasaction amount.
    /// Return 'error::ContractError::InsufficientAssetAmount' when the lease (downpayment + borrow)
    /// is with amount less than the minimum asset amount.
    pub fn calc_borrow_amount(
        &self,
        downpayment: Coin<Lpn>,
        may_max_ltd: Option<Percent>,
    ) -> ContractResult<Coin<Lpn>> {
        let amount_in_lpn: Price<Lpn, Lpn> = Price::identity();

        self.check_trasaction_amount(downpayment, amount_in_lpn)?;

        let borrow = self.liability.init_borrow_amount(downpayment, may_max_ltd);
        self.check_trasaction_amount(borrow, amount_in_lpn)
            .and_then(|_| self.check_asset_amount(downpayment.add(borrow), amount_in_lpn))
            .map(|_| borrow)
    }

    fn invariant_held(&self) -> ContractResult<()> {
        Self::check(
            !self.min_asset.is_zero(),
            "Min asset amount should be positive",
        )
        .and(Self::check(
            !self.min_trasaction_amount.is_zero(),
            "Min trasaction amount should be positive",
        ))
    }

    fn check(invariant: bool, msg: &str) -> ContractResult<()> {
        ContractError::broken_invariant_if::<Self>(!invariant, msg)
    }
}

#[cfg(test)]
mod test_calc_borrow {
    use currency::dex::test::StableC1;
    use finance::{coin::Coin, duration::Duration, liability::Liability, percent::Percent};

    use crate::error::ContractError;

    use super::Spec;

    type TestLpn = StableC1;

    #[test]
    fn downpayment_less_than_min() {
        let spec = spec(1_000, 100);
        let borrow = spec.calc_borrow_amount(99.into(), None);

        assert!(matches!(
            borrow,
            Err(ContractError::InsufficientTrasactionAmount(_))
        ));
    }

    #[test]
    fn borrow_less_than_min() {
        let spec = spec(1_000, 100);
        let borrow = spec.calc_borrow_amount(100.into(), Some(Percent::from_percent(43)));

        assert!(matches!(
            borrow,
            Err(ContractError::InsufficientTrasactionAmount(_))
        ));
    }

    #[test]
    fn lease_less_than_min() {
        let spec = spec(1_000, 200);
        let borrow_1 = spec.calc_borrow_amount(250.into(), None);
        assert!(matches!(
            borrow_1,
            Err(ContractError::InsufficientAssetAmount(_))
        ));

        let borrow_2 = spec.calc_borrow_amount(550.into(), Some(Percent::from_percent(81)));
        assert!(matches!(
            borrow_2,
            Err(ContractError::InsufficientAssetAmount(_))
        ));
    }

    #[test]
    fn valid_borrow_amount() {
        let spec = spec(1_000, 300);
        let borrow_1 = spec.calc_borrow_amount(540.into(), None);
        assert_eq!(Coin::<TestLpn>::new(1002), borrow_1.unwrap());

        let borrow_2 = spec.calc_borrow_amount(550.into(), Some(Percent::from_percent(82)));
        assert_eq!(Coin::<TestLpn>::new(451), borrow_2.unwrap());

        let borrow_3 = spec.calc_borrow_amount(870.into(), Some(Percent::from_percent(100)));
        assert_eq!(Coin::<TestLpn>::new(870), borrow_3.unwrap());

        let borrow_4 = spec.calc_borrow_amount(650.into(), Some(Percent::from_percent(150)));
        assert_eq!(Coin::<TestLpn>::new(975), borrow_4.unwrap());
    }

    fn spec<Lpn>(min_asset: Lpn, min_trasaction_amount: Lpn) -> Spec<TestLpn>
    where
        Lpn: Into<Coin<TestLpn>>,
    {
        let liability = Liability::new(
            Percent::from_percent(65),
            Percent::from_percent(5),
            Percent::from_percent(10),
            Percent::from_percent(2),
            Percent::from_percent(3),
            Percent::from_percent(2),
            Duration::from_hours(1),
        );
        Spec::new(liability, min_asset.into(), min_trasaction_amount.into())
    }
}
