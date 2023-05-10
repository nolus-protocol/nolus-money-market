use serde::Serialize;

use finance::{coin::Coin, currency::Currency};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;

use crate::{error::ContractResult, lease::Lease, loan::RepayReceipt};

impl<Lpn, Asset, Lpp, Oracle> Lease<Lpn, Asset, Lpp, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppLoanTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
    Asset: Currency + Serialize,
{
    pub(crate) fn repay<Profit>(
        &mut self,
        payment: Coin<Lpn>,
        now: Timestamp,
        profit: &mut Profit,
    ) -> ContractResult<RepayReceipt<Lpn>>
    where
        Profit: ProfitTrait,
    {
        self.loan.repay(payment, now, profit)
    }

    pub(crate) fn liquidate_partial<Profit>(
        &mut self,
        asset: Coin<Asset>,
        payment: Coin<Lpn>,
        now: Timestamp,
        profit: &mut Profit,
    ) -> ContractResult<RepayReceipt<Lpn>>
    where
        Profit: ProfitTrait,
    {
        debug_assert!(
            asset < self.amount,
            "Liquidated asset {asset} should be less than the available {0}",
            self.amount
        );
        self.amount -= asset;
        self.loan.repay(payment, now, profit)
    }
}
