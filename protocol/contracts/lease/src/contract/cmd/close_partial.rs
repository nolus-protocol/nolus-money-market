use currency::{CurrencyDef, MemberOf};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::bank::FixedAddressSender;
use sdk::cosmwasm_std::Timestamp;

use crate::{
    api::{LeaseAssetCurrencies, LeaseCoin, LeasePaymentCurrencies},
    error::ContractResult,
    finance::{LpnCoin, LpnCurrencies, LpnCurrency, OracleRef},
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
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        Lpp: LppLoanTrait<LpnCurrency>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>,
        Profit: FixedAddressSender,
    {
        self.asset
            .try_into()
            .map_err(Into::into)
            .and_then(|asset| lease.close_partial(asset, payment, now, profit))
    }
}
