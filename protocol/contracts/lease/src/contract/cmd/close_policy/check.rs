use currency::{CurrencyDef, MemberOf};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies},
    error::{ContractError, ContractResult},
    finance::{LpnCurrencies, LpnCurrency, OracleRef},
    lease::{Lease as LeaseDO, with_lease::WithLease},
};

use super::CloseStatusDTO;

pub(crate) fn check<Asset, Lpp, Oracle>(
    lease: &LeaseDO<Asset, Lpp, Oracle>,
    when: &Timestamp,
    time_alarms: &TimeAlarmsRef,
    price_alarms: &OracleRef,
) -> ContractResult<CloseStatusDTO>
where
    Asset: CurrencyDef,
    Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
    Lpp: LppLoanTrait<LpnCurrency>,
    Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>,
{
    lease
        .price_of_lease_currency()
        .map(|asset_in_lpns| lease.check_close_policy(asset_in_lpns, when))
        .and_then(|status| CloseStatusDTO::try_from_do(status, when, time_alarms, price_alarms))
}

pub(crate) struct CheckCmd<'a> {
    now: &'a Timestamp,
    time_alarms: &'a TimeAlarmsRef,
    price_alarms: &'a OracleRef,
}

impl<'a> CheckCmd<'a> {
    pub fn new(
        now: &'a Timestamp,
        time_alarms: &'a TimeAlarmsRef,
        price_alarms: &'a OracleRef,
    ) -> Self {
        Self {
            now,
            time_alarms,
            price_alarms,
        }
    }
}

impl WithLease for CheckCmd<'_> {
    type Output = CloseStatusDTO;

    type Error = ContractError;

    fn exec<Asset, Loan, Oracle>(
        self,
        lease: LeaseDO<Asset, Loan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        Loan: LppLoanTrait<LpnCurrency>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>,
    {
        check(&lease, self.now, self.time_alarms, self.price_alarms)
    }
}
