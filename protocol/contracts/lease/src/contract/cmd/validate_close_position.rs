use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;

use crate::{
    api::position::PartialClose,
    error::ContractError,
    finance::{LpnCurrencies, LpnCurrency},
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
        Asset: currency::Currency,
        LppLoan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LpnCurrency, LpnCurrencies>,
    {
        (&self.spec.amount)
            .try_into()
            .map_err(Into::into)
            .and_then(|amount| lease.validate_close(amount))
    }
}
