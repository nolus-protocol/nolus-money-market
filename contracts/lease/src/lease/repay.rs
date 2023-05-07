use serde::Serialize;

use finance::{coin::Coin, currency::Currency};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;

use crate::{error::ContractResult, lease::Lease, loan::RepayReceipt};

impl<Lpn, Asset, Lpp, Profit, Oracle> Lease<Lpn, Asset, Lpp, Profit, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppLoanTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
    Profit: ProfitTrait,
    Asset: Currency + Serialize,
{
    pub(crate) fn repay(
        &mut self,
        payment: Coin<Lpn>,
        now: Timestamp,
    ) -> ContractResult<RepayReceipt<Lpn>> {
        self.loan.repay(payment, now, self.addr.clone())
    }

    pub(crate) fn liquidate(
        &mut self,
        asset: Coin<Asset>,
        payment: Coin<Lpn>,
        now: Timestamp,
    ) -> ContractResult<RepayReceipt<Lpn>> {
        debug_assert!(
            asset <= self.amount,
            "Liquidating {asset} is greater than the available {0}",
            self.amount
        );
        self.amount -= asset;
        self.loan.repay(payment, now, self.addr.clone())
    }
}
