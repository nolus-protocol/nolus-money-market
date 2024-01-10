use currencies::LeaseGroup;
use currency::Currency;
use finance::{coin::Coin, price::Price, zero::Zero};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use sdk::cosmwasm_std::Timestamp;

use crate::{error::ContractResult, loan::LiabilityStatus, position::Status};

use super::Lease;

impl<Lpn, Asset, Lpp, Oracle> Lease<Lpn, Asset, Lpp, Oracle>
where
    Lpn: Currency,
    Lpp: LppLoanTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
    Asset: Currency,
{
    pub(crate) fn liquidation_status(&self, now: Timestamp) -> ContractResult<Status<Asset>> {
        let LiabilityStatus { total_due, overdue } = self.loan.liability_status(now);

        let overdue = if self.loan.grace_period_end() <= now {
            overdue
        } else {
            Coin::ZERO
        };

        self.price_of_lease_currency().map(|asset_in_lpns| {
            self.position
                .check_liability(total_due, overdue, asset_in_lpns)
        })
    }

    pub(super) fn price_of_lease_currency(&self) -> ContractResult<Price<Asset, Lpn>> {
        self.oracle
            .price_of::<Asset, LeaseGroup>()
            .map_err(Into::into)
    }
}
