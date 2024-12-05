use currency::{CurrencyDef, MemberOf};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies},
    finance::{LpnCurrencies, LpnCurrency, OracleRef},
};

use super::Lease;

pub trait WithLease {
    type Output;
    type Error;

    // TODO pass a TimeAlarmsRef, a ReserveRef, and a ProfitRef to avoid
    // 1) passing them through the implementers, and
    // 2) cloning them from the LeaseDTO

    fn exec<Asset, LppLoan, Oracle>(
        self,
        lease: Lease<Asset, LppLoan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        LppLoan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>;
}
