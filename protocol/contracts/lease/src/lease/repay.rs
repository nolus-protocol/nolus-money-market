use sdk::cosmwasm_std::Timestamp;

use currency::Currency;
use finance::coin::Coin;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::bank::FixedAddressSender;

use crate::{error::ContractResult, lease::Lease, loan::RepayReceipt};

impl<Lpn, Asset, Lpp, Oracle> Lease<Lpn, Asset, Lpp, Oracle>
where
    Lpn: Currency,
    Lpp: LppLoanTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
    Asset: Currency,
{
    pub(crate) fn repay<Profit>(
        &mut self,
        payment: Coin<Lpn>,
        now: Timestamp,
        profit: &mut Profit,
    ) -> ContractResult<RepayReceipt<Lpn>>
    where
        Profit: FixedAddressSender,
    {
        self.loan.repay(payment, now, profit)
    }
}
