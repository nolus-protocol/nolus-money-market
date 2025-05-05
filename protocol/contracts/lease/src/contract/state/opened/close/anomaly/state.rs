use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::query::StateResponse,
    contract::{
        Lease,
        state::{Handler, Response},
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

// RepayableT: Closable + Repayable
impl<RepayableT> Handler for SlippageAnomaly<RepayableT> {
    fn state(
        self,
        _now: Timestamp,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        todo!()
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
