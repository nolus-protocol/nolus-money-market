use currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::bank::FixedAddressSender;
use sdk::cosmwasm_std::Timestamp;

use crate::{
    api::{LeaseCoin, LpnCoin, LpnCurrencies, LpnCurrency},
    error::ContractResult,
    lease::Lease,
    loan::RepayReceipt,
};

use super::repayable::RepayFn;

pub(crate) struct CloseFn {
    asset: LeaseCoin,
}
impl CloseFn {
    pub fn new(asset: LeaseCoin) -> Self {
        Self { asset }
    }
}
impl RepayFn for CloseFn {
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
        self.asset
            .try_into()
            .map_err(Into::into)
            .and_then(|asset| lease.close_partial(asset, payment, now, profit))
    }
}
