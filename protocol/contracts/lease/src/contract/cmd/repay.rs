use currency::Currency;
use finance::coin::Coin;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::bank::FixedAddressSender;
use sdk::cosmwasm_std::Timestamp;

use crate::{error::ContractResult, lease::Lease, loan::RepayReceipt};

use super::repayable::RepayFn;

pub(crate) struct RepayLeaseFn {}
impl RepayFn for RepayLeaseFn {
    fn do_repay<Lpn, Asset, Lpp, Oracle, Profit>(
        self,
        lease: &mut Lease<Lpn, Asset, Lpp, Oracle>,
        payment: Coin<Lpn>,
        now: &Timestamp,
        profit: &mut Profit,
    ) -> ContractResult<RepayReceipt<Lpn>>
    where
        Lpn: Currency,
        Lpp: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Asset: Currency,
        Profit: FixedAddressSender,
    {
        lease.repay(payment, now, profit)
    }
}
