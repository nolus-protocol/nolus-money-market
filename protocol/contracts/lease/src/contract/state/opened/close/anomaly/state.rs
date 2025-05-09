use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::query::{
        StateResponse,
        opened::{PositionCloseTrx, Status},
    },
    contract::{
        Lease,
        state::{
            Handler, Response,
            opened::{self, close::Closable},
        },
    },
    error::ContractResult,
};

/// A **top-level** state that indicates a failed Lease asset swap
///
/// A Lease may go into this state if a swap anomaly, for example, a slippage protection is detected.
///
/// Only an anomaly manager may resolve that Lease.
#[derive(Serialize, Deserialize)]
pub(crate) struct SlippageAnomaly<RepayableT> {
    lease: Lease,
    repayable: RepayableT,
}

impl<RepayableT> SlippageAnomaly<RepayableT> {
    pub(in crate::contract::state) fn new(lease: Lease, repayable: RepayableT) -> Self {
        Self { lease, repayable }
    }
}

impl<RepayableT> Handler for SlippageAnomaly<RepayableT>
where
    RepayableT: Closable,
{
    fn state(
        self,
        now: Timestamp,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        let in_progress = self
            .repayable
            .transaction(&self.lease, PositionCloseTrx::Swap);

        opened::lease_state(
            self.lease,
            Status::SlippageProtectionActivated(in_progress),
            now,
            due_projection,
            querier,
        )
    }

    fn heal(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        todo!("TODO check for access permission, and then Lease::check_close_policy()")
    }
}
