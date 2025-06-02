use currency::{CurrencyDef, MemberOf};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies, position::ClosePolicyChange},
    contract::LeaseDTOResult,
    error::ContractError,
    finance::{LpnCurrencies, LpnCurrency, OracleRef, ReserveRef},
    lease::{Lease as LeaseDO, with_lease::WithLease},
};

use super::CloseStatusDTO;

pub(crate) struct ChangeCmd<'now, 'price_alarms> {
    change: ClosePolicyChange,
    now: &'now Timestamp,
    // LeaseDTO attributes
    profit: ProfitRef,
    reserve: ReserveRef,
    time_alarms: TimeAlarmsRef,
    // alarms setup
    price_alarms: &'price_alarms OracleRef,
}

pub(crate) type ChangePolicyResult = LeaseDTOResult<CloseStatusDTO>;

impl<'now, 'price_alarms> ChangeCmd<'now, 'price_alarms> {
    pub fn new(
        change: ClosePolicyChange,
        now: &'now Timestamp,
        // LeaseDTO attributes follow
        profit: ProfitRef,
        time_alarms: TimeAlarmsRef,
        price_alarms: &'price_alarms OracleRef,
        reserve: ReserveRef,
    ) -> Self {
        Self {
            change,
            now,
            profit,
            reserve,
            time_alarms,
            price_alarms,
        }
    }
}

impl WithLease for ChangeCmd<'_, '_> {
    type Output = ChangePolicyResult;

    type Error = ContractError;

    fn exec<Asset, Loan, Oracle>(
        self,
        mut lease: LeaseDO<Asset, Loan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        Loan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>,
    {
        lease
            .price_of_lease_currency()
            .and_then(|asset_in_lpns| {
                lease
                    .change_close_policy(self.change, asset_in_lpns, self.now)
                    .map(|()| lease.check_close_policy(asset_in_lpns, self.now))
            })
            .and_then(|status| {
                CloseStatusDTO::try_from_do(status, self.now, &self.time_alarms, self.price_alarms)
            })
            .map(|status_dto| Self::Output {
                lease: lease.into_dto(self.profit, self.time_alarms, self.reserve),
                result: status_dto,
            })
    }
}
