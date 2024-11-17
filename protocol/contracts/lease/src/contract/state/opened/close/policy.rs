use currency::{CurrencyDef, MemberOf};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::Oracle as OracleTrait;
use platform::batch::Batch;
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{position::ClosePolicyChange, LeaseAssetCurrencies, LeasePaymentCurrencies},
    contract::{cmd::CloseStatusDTO, SplitDTOOut},
    error::ContractError,
    finance::{LpnCurrencies, LpnCurrency, OracleRef, ReserveRef},
    lease::{with_lease::WithLease, IntoDTOResult, Lease, LeaseDTO},
};

pub(in crate::contract) struct ChangeCmd {
    spec: ClosePolicyChange,
    now: Timestamp,
    profit: ProfitRef, // though necessary only past a repay we pass it to keep uniformity on DO -> (DTO, Batch) transformation
    // in all usecases where a Lease is mutated
    time_alarms: TimeAlarmsRef,
    price_alarms: OracleRef,
    reserve: ReserveRef,
}

impl ChangeCmd {
    pub fn new(
        spec: ClosePolicyChange,
        now: Timestamp,
        profit: ProfitRef,
        time_alarms: TimeAlarmsRef,
        price_alarms: OracleRef,
        reserve: ReserveRef,
    ) -> Self {
        Self {
            spec,
            now,
            profit,
            time_alarms,
            price_alarms,
            reserve,
        }
    }
}

impl WithLease for ChangeCmd {
    type Output = LeaseChangePolicyResult;

    type Error = ContractError;

    fn exec<Asset, LppLoan, Oracle>(
        self,
        mut lease: Lease<Asset, LppLoan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
        LppLoan: LppLoanTrait<LpnCurrency, LpnCurrencies>,
        Oracle: OracleTrait<LeasePaymentCurrencies, QuoteC = LpnCurrency, QuoteG = LpnCurrencies>
            + Into<OracleRef>,
    {
        lease
            .change_close_policy(self.spec, &self.now)
            .and_then(|()| lease.check_close(&self.now, &self.time_alarms, &self.price_alarms))
            .map(Into::into)
            .and_then(|close_status| {
                lease
                    .try_into_dto(self.profit, self.time_alarms, self.reserve)
                    .map(
                        |IntoDTOResult { lease, batch: msgs }| LeaseChangePolicyResult {
                            lease,
                            change_result: ChangePolicyResult { close_status, msgs },
                        },
                    )
            })
    }
}

struct LeaseChangePolicyResult {
    lease: LeaseDTO,
    change_result: ChangePolicyResult,
}

pub(in crate::contract::state::opened) struct ChangePolicyResult {
    pub close_status: CloseStatusDTO,
    pub msgs: Batch,
}

impl SplitDTOOut for LeaseChangePolicyResult {
    type Other = ChangePolicyResult;

    fn split_into(self) -> (LeaseDTO, Self::Other) {
        (self.lease, self.change_result)
    }
}
