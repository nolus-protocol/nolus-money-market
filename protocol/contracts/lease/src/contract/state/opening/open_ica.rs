use finance::duration::Duration;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

use dex::{
    Account, Connectable, ConnectionParams, Contract as DexContract, DexResult, IcaConnectee,
    TimeAlarm, TransferOut,
};
use platform::batch::Batch;
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        DownpaymentCoin,
        open::NewLeaseContract,
        query::{StateResponse as QueryStateResponse, opening::OngoingTrx},
    },
    contract::{cmd::OpenLoanRespResult, finalize::LeasesRef, state::SwapClient},
    error::ContractResult,
    finance::{LppRef, OracleRef},
};

use super::buy_asset::{BuyAsset, DexState};

#[derive(Serialize, Deserialize)]
pub(crate) struct OpenIcaAccount {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
    start_opening_at: Timestamp,
}

impl OpenIcaAccount {
    pub(super) fn new(
        new_lease: NewLeaseContract,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
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
    type NextState = TransferOut<BuyAsset, Self::State, SwapClient>;

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

impl Connectable for OpenIcaAccount {
    fn dex(&self) -> &ConnectionParams {
        &self.new_lease.dex
    }
}

impl DexContract for OpenIcaAccount {
    type StateResponse = ContractResult<QueryStateResponse>;

    fn state(
        self,
        _now: Timestamp,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        Ok(QueryStateResponse::Opening {
            currency: self.new_lease.form.currency,
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
    fn setup_alarm(&self, r#for: Timestamp) -> DexResult<Batch> {
        self.deps.2.setup_alarm(r#for).map_err(Into::into)
    }
}
