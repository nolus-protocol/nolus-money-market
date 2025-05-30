use currency::{CurrencyDef, MemberOf};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::batch::Batch;
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies, position::ClosePolicyChange},
    contract::SplitDTOOut,
    error::ContractError,
    finance::{LpnCurrencies, LpnCurrency, OracleRef, ReserveRef},
    lease::{IntoDTOResult, Lease as LeaseDO, LeaseDTO, with_lease::WithLease},
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
    type Output = IntoDTOResult;

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
        lease.price_of_lease_currency()
        .and_then(|asset_in_lpns|
        lease
            .change_close_policy(self.change, asset_in_lpns, self.now)
            .map(|()| lease.check_close_policy(asset_in_lpns, self.now)))
            .and_then(|status|CloseStatusDTO::try_from_do(status, self.now, &self.time_alarms, self.price_alarms) )
            .and_then(|status_dto| {
                let alarms = match status_dto {
                    CloseStatusDTO::Paid => unimplemented!("changing an Active Opened Lease is only permitted"),
                    CloseStatusDTO::None { current_liability: _, alarms  } => alarms,
                    CloseStatusDTO::CloseAsked(_) => unimplemented!("triggering a close with a policy change should have already resulted in an error"),
                    CloseStatusDTO::NeedLiquidation(_) => unimplemented!("triggering a liquidation with a policy change should have already resulted in an error"),
                };
                lease
                    .try_into_dto(self.profit, self.time_alarms, self.reserve)
                    .inspect(|res| {
                        debug_assert!(res.batch.is_empty());
                    }).map(|res| IntoDTOResult{batch: res.batch.merge(alarms), lease: res.lease})
            })
    }
}

impl SplitDTOOut for IntoDTOResult {
    type Other = Batch;

    fn split_into(self) -> (LeaseDTO, Self::Other) {
        (self.lease, self.batch)
    }
}
