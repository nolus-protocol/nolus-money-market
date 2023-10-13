use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

use dex::{
    Account, ConnectionParams, Contract as DexContract, DexConnectable, DexResult, IcaConnectee,
    TimeAlarm, TransferOut,
};
use lpp::stub::LppRef;
use oracle::stub::OracleRef;
use platform::batch::Batch;
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{self, opening::OngoingTrx, DownpaymentCoin, NewLeaseContract, StateResponse},
    contract::{cmd::OpenLoanRespResult, finalize::FinalizerRef},
    error::ContractResult,
};

use super::buy_asset::{BuyAsset, DexState};

#[derive(Serialize, Deserialize)]
pub(crate) struct OpenIcaAccount {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef, FinalizerRef),
    start_opening_at: Timestamp,
}

impl OpenIcaAccount {
    pub(super) fn new(
        new_lease: NewLeaseContract,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppRef, OracleRef, TimeAlarmsRef, FinalizerRef),
        start_opening_at: Timestamp,
    ) -> Self {
        Self {
            new_lease,
            downpayment,
            loan,
            deps,
            start_opening_at,
        }
    }
}

impl IcaConnectee for OpenIcaAccount {
    type State = DexState;
    type NextState = TransferOut<BuyAsset, Self::State>;

    fn connected(self, dex_account: Account) -> Self::NextState {
        TransferOut::new(BuyAsset::new(
            self.new_lease.form,
            dex_account,
            self.downpayment,
            self.loan,
            self.deps,
            self.start_opening_at,
        ))
    }
}

impl DexConnectable for OpenIcaAccount {
    fn dex(&self) -> &ConnectionParams {
        &self.new_lease.dex
    }
}

impl DexContract for OpenIcaAccount {
    type StateResponse = ContractResult<api::StateResponse>;

    fn state(self, _now: Timestamp, _querier: &QuerierWrapper<'_>) -> Self::StateResponse {
        Ok(StateResponse::Opening {
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: OngoingTrx::OpenIcaAccount {},
        })
    }
}

impl Display for OpenIcaAccount {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("OpenIcaAccount"))
    }
}

impl TimeAlarm for OpenIcaAccount {
    fn setup_alarm(&self, forr: Timestamp) -> DexResult<Batch> {
        self.deps.2.setup_alarm(forr).map_err(Into::into)
    }
}
