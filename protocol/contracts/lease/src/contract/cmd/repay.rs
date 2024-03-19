use currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::bank::FixedAddressSender;
use sdk::cosmwasm_std::Timestamp;

use crate::{
    api::{LpnCoin, LpnCurrencies, LpnCurrency},
    error::ContractResult,
    lease::Lease,
    loan::RepayReceipt,
};

use super::repayable::RepayFn;

pub(crate) struct RepayLeaseFn {}
impl RepayFn for RepayLeaseFn {
    fn do_repay<Asset, Lpp, Oracle, Profit>(
        self,
        lease: &mut Lease<Asset, Lpp, Oracle>,
        payment: LpnCoin,
        now: &Timestamp,
        profit: &mut Profit,
    ) -> ContractResult<RepayReceipt>
    where
        Lpp: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LpnCurrency>,
        Asset: Currency,
        Profit: FixedAddressSender,
    {
        lease.repay(payment, now, profit)
    }
}
