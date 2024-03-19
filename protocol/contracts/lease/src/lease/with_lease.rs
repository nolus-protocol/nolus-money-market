use currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;

use crate::api::{LpnCurrencies, LpnCurrency};

use super::Lease;

pub trait WithLease {
    type Output;
    type Error;

    fn exec<Asset, LppLoan, Oracle>(
        self,
        lease: Lease<Asset, LppLoan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: Currency,
        LppLoan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LpnCurrency>;
}
