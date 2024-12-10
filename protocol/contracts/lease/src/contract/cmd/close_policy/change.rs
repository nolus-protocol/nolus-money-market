use currency::{CurrencyDef, MemberOf};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::batch::Batch;
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{position::ClosePolicyChange, LeaseAssetCurrencies, LeasePaymentCurrencies},
    contract::SplitDTOOut,
    error::ContractError,
    finance::{LpnCurrencies, LpnCurrency, OracleRef, ReserveRef},
    lease::{with_lease::WithLease, IntoDTOResult, Lease as LeaseDO, LeaseDTO},
};

pub(crate) struct ChangeCmd<'a> {
    change: ClosePolicyChange,
    now: &'a Timestamp,
    // LeaseDTO attributes
    profit: ProfitRef,
    reserve: ReserveRef,
    time_alarms: TimeAlarmsRef,
}

impl<'a> ChangeCmd<'a> {
    pub fn new(
        change: ClosePolicyChange,
        now: &'a Timestamp,
        // LeaseDTO attributes follow
        profit: ProfitRef,
        time_alarms: TimeAlarmsRef,
        reserve: ReserveRef,
    ) -> Self {
        Self {
            change,
            now,
            profit,
            reserve,
            time_alarms,
        }
    }
}

impl WithLease for ChangeCmd<'_> {
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
        lease
            .change_close_policy(self.change, self.now)
            .and_then(|()| {
                lease
                    .try_into_dto(self.profit, self.time_alarms, self.reserve)
                    .inspect(|res| {
                        debug_assert!(res.batch.is_empty());
                    })
            })
    }
}

impl SplitDTOOut for IntoDTOResult {
    type Other = Batch;

    fn split_into(self) -> (LeaseDTO, Self::Other) {
        (self.lease, self.batch)
    }
}
