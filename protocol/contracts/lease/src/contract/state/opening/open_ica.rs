use finance::duration::Duration;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

use dex::{
    Account, Connectable, ConnectionParams, Contract as DexContract, DexResult, Error as DexError,
    IcaConnectee, MaxSlippage, RemoteSwap, TimeAlarm, TransferOut,
};
use finance::instant::Instant;
use platform::batch::Batch;
use remote_lease::response::RemoteLeaseId;
use sdk::cosmwasm_std::{MessageInfo, QuerierWrapper};
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
    max_slippage: MaxSlippage,
    deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
    start_opening_at: Instant,
    remote_lease_id: RemoteLeaseId,
}

impl OpenIcaAccount {
    pub(super) fn new(
        new_lease: NewLeaseContract,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        max_slippage: MaxSlippage,
        deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
        start_opening_at: Instant,
        remote_lease_id: RemoteLeaseId,
    ) -> Self {
        Self {
            new_lease,
            downpayment,
            loan,
            max_slippage,
            deps,
            start_opening_at,
            remote_lease_id,
        }
    }
}

impl IcaConnectee for OpenIcaAccount {
    type State = DexState;
    type NextState =
        TransferOut<BuyAsset, Self::State, SwapClient, RemoteSwap<BuyAsset, Self::State>>;

    fn connected(self, dex_account: Account) -> Self::NextState {
        let NewLeaseContract {
            form,
            dex: _,
            finalizer: _,
            remote_lease_controller,
            expected_instance_ordinal: _,
        } = self.new_lease;
        TransferOut::new(BuyAsset::new(
            form,
            dex_account,
            self.downpayment,
            self.loan,
            self.max_slippage,
            remote_lease_controller,
            self.deps,
            self.start_opening_at,
            self.remote_lease_id,
        ))
    }

    fn authz_remote_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> DexResult<()> {
        access_control::check(&self.deps.3.remote_lease_callback_permission(querier), info)
            .map_err(DexError::Unauthorized)
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
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        Ok(QueryStateResponse::Opening {
            currency: self.new_lease.form.currency,
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: OngoingTrx::OpenLease {
                remote_lease: self.remote_lease_id,
            },
        })
    }
}

impl Display for OpenIcaAccount {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("OpenIcaAccount"))
    }
}

impl TimeAlarm for OpenIcaAccount {
    fn setup_alarm(&self, r#for: Instant) -> DexResult<Batch> {
        self.deps.2.setup_alarm(r#for).map_err(Into::into)
    }
}
