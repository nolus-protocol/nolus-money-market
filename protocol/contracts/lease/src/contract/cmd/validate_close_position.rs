use currency::{Currency, MemberOf};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;

use crate::{
    api::{position::PartialClose, LeaseAssetCurrencies, LeasePaymentCurrencies},
    error::ContractError,
    finance::{LpnCurrencies, LpnCurrency, OracleRef},
    lease::{with_lease::WithLease, Lease},
};

pub(in crate::contract) struct Cmd<'spec> {
    spec: &'spec PartialClose,
}

impl<'spec> Cmd<'spec> {
    pub fn new(spec: &'spec PartialClose) -> Self {
        Self { spec }
    }
}

impl<'spec> WithLease for Cmd<'spec> {
    type Output = ();

    type Error = ContractError;

    fn exec<Asset, LppLoan, Oracle>(
        self,
        lease: Lease<Asset, LppLoan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: Currency + MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        LppLoan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>,
    {
        lease.validate_close(self.spec.amount.as_specific())
    }
}
