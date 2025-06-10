use currency::{CurrencyDef, MemberOf};
use finance::coin::Coin;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies, position::PartialClose},
    error::ContractError,
    finance::{LpnCurrencies, LpnCurrency, OracleRef},
    lease::{Lease, with_lease::WithLease},
};

pub(in crate::contract) struct Cmd<'spec> {
    spec: &'spec PartialClose,
}

impl<'spec> Cmd<'spec> {
    pub fn new(spec: &'spec PartialClose) -> Self {
        Self { spec }
    }
}

impl WithLease for Cmd<'_> {
    type Output = ();

    type Error = ContractError;

    fn exec<Asset, LppLoan, Oracle>(
        self,
        lease: Lease<Asset, LppLoan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        LppLoan: LppLoanTrait<LpnCurrency>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>,
    {
        Coin::<Asset>::try_from(self.spec.amount)
            .map_err(Into::into)
            .and_then(|amount| lease.validate_close(amount))
    }
}
