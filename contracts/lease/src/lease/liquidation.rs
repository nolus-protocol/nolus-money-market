use serde::Serialize;

use currency::Currency;
use finance::{
    coin::Coin,
    liability::{self, Status},
    price::{self, Price},
    zero::Zero,
};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use sdk::cosmwasm_std::Timestamp;

use crate::{error::ContractResult, loan::LiabilityStatus};

use super::Lease;

impl<Lpn, Asset, Lpp, Oracle> Lease<Lpn, Asset, Lpp, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppLoanTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
    Asset: Currency + Serialize,
{
    const MIN_LIQUIDATION_AMOUNT: Coin<Lpn> = Coin::new(10_000); // $0.01 TODO issue #40
    const MIN_ASSET_AMOUNT_BEFORE_LIQUIDATION: Coin<Lpn> = Coin::new(15_000_000); // TODO issue #50

    pub(crate) fn liquidation_status(&self, now: Timestamp) -> ContractResult<Status<Asset>> {
        let price_in_asset = self.price_of_lease_currency()?.inv();

        let LiabilityStatus {
            total: total_due,
            previous_interest,
        } = self.loan.liability_status(now);

        let overdue = if self.loan.grace_period_end() <= now {
            previous_interest
        } else {
            Coin::ZERO
        };

        let status = liability::check_liability(
            &self.liability,
            self.amount,
            price::total(total_due, price_in_asset),
            price::total(overdue, price_in_asset),
            price::total(Self::MIN_LIQUIDATION_AMOUNT, price_in_asset),
            price::total(Self::MIN_ASSET_AMOUNT_BEFORE_LIQUIDATION, price_in_asset),
        );
        #[cfg(debug_assertion)]
        debug_assert!(status.amount() <= self.amount());
        Ok(status)
    }

    fn price_of_lease_currency(&self) -> ContractResult<Price<Asset, Lpn>> {
        Ok(self.oracle.price_of::<Asset>()?)
    }
}
