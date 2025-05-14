use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::query::{StateResponse, opened::Status},
    contract::{
        Lease,
        state::{
            Handler, Response,
            opened::{self, active::Active},
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
pub(crate) struct SlippageAnomaly {
    lease: Lease,
}

impl SlippageAnomaly {
    pub(in crate::contract::state) fn new(lease: Lease) -> Self {
        Self { lease }
    }
}

impl Handler for SlippageAnomaly {
    fn state(
        self,
        now: Timestamp,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        opened::lease_state(
            self.lease,
            Status::SlippageProtectionActivated(),
            now,
            due_projection,
            querier,
        )
    }

    fn heal(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.lease
            .leases
            .check_assess(info.sender, querier)
            .and_then(|()| Active::new(self.lease).assess_close_status(querier, &env))
    }
}
