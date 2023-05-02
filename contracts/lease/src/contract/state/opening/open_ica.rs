use serde::{Deserialize, Serialize};

use dex::{Account, ConnectionParams, Contract as DexContract, DexConnectable, IcaConnectee};
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{self, opening::OngoingTrx, DownpaymentCoin, NewLeaseContract, StateResponse},
    contract::cmd::OpenLoanRespResult,
    error::ContractResult,
};

use super::buy_asset::{self, DexState, StartState};

#[derive(Serialize, Deserialize)]
pub(crate) struct OpenIcaAccount {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef, TimeAlarmsRef),
}

impl OpenIcaAccount {
    pub(super) fn new(
        new_lease: NewLeaseContract,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppLenderRef, OracleRef, TimeAlarmsRef),
    ) -> Self {
        Self {
            new_lease,
            downpayment,
            loan,
            deps,
        }
    }
}

impl IcaConnectee for OpenIcaAccount {
    type State = DexState;
    type NextState = StartState;

    fn connected(self, dex_account: Account) -> Self::NextState {
        buy_asset::start(
            self.new_lease.form,
            dex_account,
            self.downpayment,
            self.loan,
            self.deps,
        )
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
