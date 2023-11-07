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
    min_transaction: Coin<Lpn>,
}

impl<Lpn> Spec<Lpn>
where
    Lpn: Currency,
{
    pub fn new(liability: Liability, min_asset: Coin<Lpn>, min_transaction: Coin<Lpn>) -> Self {
        let obj = Self {
            liability,
            min_asset,
            min_transaction,
        };
        debug_assert_eq!(Ok(()), obj.invariant_held());
        obj
    }

    pub fn liability(&self) -> Liability {
        self.liability
    }

    /// Check if the amount can be used to close the position.
    /// Return `error::ContractError::PositionCloseAmountTooSmall` when a partial close is requested
    /// with amount less than the minimum sell asset position parameter sent on lease open. Refer to
    /// `NewLeaseForm::position_spec`.
    ///
    /// Return `error::ContractError::PositionCloseAmountTooBig` when a partial close is requested
    /// with amount that would decrease a position less than the minimum asset parameter sent on
    /// lease open. Refer to `NewLeaseForm::position_spec`.
    pub fn validate_close_amount<Asset>(
        &self,
        asset: Coin<Asset>,
        close_amount: Coin<Asset>,
        lpn_in_assets: Price<Lpn, Asset>,
    ) -> ContractResult<()>
    where
        Asset: Currency,
    {
        let transaction_currency_in_lpn = lpn_in_assets.inv();

        self.check_transaction_amount(close_amount, transaction_currency_in_lpn)
            .map_err(|err| match err {
                ContractError::InsufficientTransactionAmount(min_transaction) => {
                    ContractError::PositionCloseAmountTooSmall(min_transaction)
                }
                _ => err,
            })
            .and_then(|()| {
                self.check_asset_amount(
                    asset.saturating_sub(close_amount),
                    transaction_currency_in_lpn,
                )
            })
            .map_err(|err| match err {
                ContractError::InsufficientAssetAmount(min_asset) => {
                    ContractError::PositionCloseAmountTooBig(min_asset)
                }
                _ => err,
            })
    }

    /// Calculate the borrow amount.
    /// Return 'error::ContractError::InsufficientTransactionAmount' when either the downpayment
    /// or the borrow amount is less than the minimum transaction amount.
    /// Return 'error::ContractError::InsufficientAssetAmount' when the lease (downpayment + borrow)
    /// is less than the minimum asset amount.
    pub fn calc_borrow_amount(
        &self,
        downpayment: Coin<Lpn>,
        may_max_ltd: Option<Percent>,
    ) -> ContractResult<Coin<Lpn>> {
        self.check_transaction_amount(downpayment, Price::identity())
            .map(|()| self.liability.init_borrow_amount(downpayment, may_max_ltd))
            .and_then(|borrow| {
                self.check_transaction_amount(borrow, Price::identity())
                    .and_then(|()| {
                        self.check_asset_amount(downpayment.add(borrow), Price::identity())
                    })
                    .map(|()| borrow)
            })
    }

    fn invariant_held(&self) -> ContractResult<()> {
        Self::check(
            !self.min_asset.is_zero(),
            "Min asset amount should be positive",
        )
        .and(Self::check(
            !self.min_transaction.is_zero(),
            "Min transaction amount should be positive",
        ))
    }

    fn check(invariant: bool, msg: &str) -> ContractResult<()> {
        ContractError::broken_invariant_if::<Self>(!invariant, msg)
    }

    fn check_transaction_amount<TransactionC>(
        &self,
        amount: Coin<TransactionC>,
        transaction_currency_in_lpn: Price<TransactionC, Lpn>,
    ) -> ContractResult<()>
    where
        TransactionC: Currency,
    {
        let amount = price::total(amount, transaction_currency_in_lpn);

        if amount < self.min_transaction {
            Err(ContractError::InsufficientTransactionAmount(
                self.min_transaction.into(),
            ))
        } else {
            Ok(())
        }
    }

    fn check_asset_amount<TransactionC>(
        &self,
        asset_amount: Coin<TransactionC>,
        transaction_currency_in_lpn: Price<TransactionC, Lpn>,
    ) -> ContractResult<()>
    where
        TransactionC: Currency,
    {
        let asset_amount = price::total(asset_amount, transaction_currency_in_lpn);

        if asset_amount < self.min_asset {
            Err(ContractError::InsufficientAssetAmount(
                self.min_asset.into(),
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod test_validate_close {
    use currency::dex::test::{PaymentC3, StableC1};
    use finance::{
        coin::Coin,
        duration::Duration,
        liability::Liability,
        percent::Percent,
        price::{self, Price},
    };

    use crate::{error::ContractError, position::{Position, Spec}};

    type TestCurrency = PaymentC3;
    type TestLpn = StableC1;

    #[test]
    fn too_small_amount() {
        let spec = position(100, 75, 15);
        let result_1 = spec.validate_close_amount(14.into(), price(1, 1));
        assert!(matches!(
            result_1,
            Err(ContractError::PositionCloseAmountTooSmall(_))
        ));

        let result_2 = spec.validate_close_amount(6.into(), price(2, 1));
        assert!(matches!(
            result_2,
            Err(ContractError::PositionCloseAmountTooSmall(_))
        ));
    }

    #[test]
    fn amount_as_min_sell_asset() {
        let spec = position(100, 85, 15);
        let result_1 = spec.validate_close_amount(15.into(), price(1, 1));
        assert!(result_1.is_ok());

        let result_2 = spec.validate_close_amount(5.into(), price(3, 1));
        assert!(result_2.is_ok());
    }

    #[test]
    fn too_big_amount() {
        let spec = position(100, 25, 1);
        let result_1 = spec.validate_close_amount(76.into(), price(1, 1));
        assert!(matches!(
            result_1,
            Err(ContractError::PositionCloseAmountTooBig(_))
        ));

        let result_2 = spec.validate_close_amount(64.into(), price(2, 3));
        assert!(matches!(
            result_2,
            Err(ContractError::PositionCloseAmountTooBig(_))
        ));
    }

    #[test]
    fn amount_as_min_asset() {
        let spec = position(100, 25, 1);
        let result_1 = spec.validate_close_amount(75.into(), price(1, 1));
        assert!(result_1.is_ok());

        let result_2 = spec.validate_close_amount(62.into(), price(2, 3));
        assert!(result_2.is_ok());
    }

    #[test]
    fn valid_amount() {
        let spec = position(100, 40, 10);
        let result_1 = spec.validate_close_amount(53.into(), price(1, 1));
        assert!(result_1.is_ok());

        let result_2 = spec.validate_close_amount(89.into(), price(4, 1));
        assert!(result_2.is_ok());
    }

    fn position<Asset, Lpn>(
        amount: Asset,
        min_asset: Lpn,
        min_sell_asset: Lpn,
    ) -> Position<TestCurrency, TestLpn>
    where
        Asset: Into<Coin<TestCurrency>>,
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
        let spec = Spec::<TestLpn>::new(liability, min_asset.into(), min_sell_asset.into());

        Position::<TestCurrency, TestLpn>::new(amount.into(), spec)
    }

    fn price<Lpn, Asset>(price_lpn: Lpn, price_asset: Asset) -> Price<TestLpn, TestCurrency>
    where
        Lpn: Into<Coin<TestLpn>>,
        Asset: Into<Coin<TestCurrency>>,
    {
        price::total_of(price_lpn.into()).is(price_asset.into())
    }
}

#[cfg(test)]
mod test_calc_borrow {
    use currency::dex::test::StableC1;
    use finance::{
        coin::{Amount, Coin},
        duration::Duration,
        liability::Liability,
        percent::Percent,
    };

    use crate::error::ContractError;

    use super::Spec;

    type TestLpn = StableC1;

    #[test]
    fn downpayment_less_than_min() {
        let spec = spec(560, 300);

        let downpayment_less = spec.calc_borrow_amount(299.into(), None);
        assert!(matches!(
            downpayment_less,
            Err(ContractError::InsufficientTransactionAmount(_))
        ));

        let borrow = spec.calc_borrow_amount(300.into(), None);
        assert_eq!(coin_lpn(557), borrow.unwrap());
    }

    #[test]
    fn borrow_less_than_min() {
        let spec = spec(600, 300);

        let borrow_less = spec.calc_borrow_amount(300.into(), Some(Percent::from_percent(99)));
        assert!(matches!(
            borrow_less,
            Err(ContractError::InsufficientTransactionAmount(_))
        ));

        let borrow = spec.calc_borrow_amount(300.into(), Some(Percent::from_percent(100)));
        assert_eq!(coin_lpn(300), borrow.unwrap());
    }

    #[test]
    fn lease_less_than_min() {
        let spec = spec(1_000, 300);

        let borrow_1 = spec.calc_borrow_amount(349.into(), None);
        assert!(matches!(
            borrow_1,
            Err(ContractError::InsufficientAssetAmount(_))
        ));

        let borrow_2 = spec.calc_borrow_amount(350.into(), None);
        assert_eq!(coin_lpn(650), borrow_2.unwrap());

        let borrow_3 = spec.calc_borrow_amount(550.into(), Some(Percent::from_percent(81)));
        assert!(matches!(
            borrow_3,
            Err(ContractError::InsufficientAssetAmount(_))
        ));

        let borrow_3 = spec.calc_borrow_amount(550.into(), Some(Percent::from_percent(82)));
        assert_eq!(coin_lpn(451), borrow_3.unwrap());
    }

    #[test]
    fn valid_borrow_amount() {
        let spec = spec(1_000, 300);

        let borrow_1 = spec.calc_borrow_amount(540.into(), None);
        assert_eq!(coin_lpn(1002), borrow_1.unwrap());

        let borrow_2 = spec.calc_borrow_amount(870.into(), Some(Percent::from_percent(100)));
        assert_eq!(coin_lpn(870), borrow_2.unwrap());

        let borrow_3 = spec.calc_borrow_amount(650.into(), Some(Percent::from_percent(150)));
        assert_eq!(coin_lpn(975), borrow_3.unwrap());
    }

    fn spec<LpnAmount>(min_asset: LpnAmount, min_transaction: LpnAmount) -> Spec<TestLpn>
    where
        LpnAmount: Into<Coin<TestLpn>>,
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
        Spec::new(liability, min_asset.into(), min_transaction.into())
    }

    fn coin_lpn(amount: Amount) -> Coin<TestLpn> {
        Coin::<TestLpn>::new(amount)
    }
}
