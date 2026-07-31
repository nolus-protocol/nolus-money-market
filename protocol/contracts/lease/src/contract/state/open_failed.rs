use crate::api::LeasePaymentCurrencies;
use finance::{duration::Duration, instant::Instant};
use remote_lease::callback::{RemoteErrorMessage, RemoteLeaseCallback};
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper};
use serde::{Deserialize, Serialize};

use crate::{
    api::query::StateResponse as QueryStateResponse, contract::api::Contract, error::ContractResult,
};

use super::Response;

#[derive(Serialize, Deserialize)]
pub(crate) struct OpenFailed {
    reason: RemoteErrorMessage,
}

impl OpenFailed {
    pub(crate) fn new(reason: RemoteErrorMessage) -> Self {
        Self { reason }
    }
}

impl Contract for OpenFailed {
    fn state(
        self,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> ContractResult<QueryStateResponse> {
        Ok(QueryStateResponse::OpenFailed {
            reason: self.reason,
        })
    }

    /// Accepts the callback and drops it: the Lease is left unchanged and no
    /// messages or events are produced
    fn on_remote_lease_callback(
        self,
        _callback: RemoteLeaseCallback<LeasePaymentCurrencies>,
        _info: MessageInfo,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> ContractResult<Response> {
        // A success ack for the OpenLease packet may still arrive after a
        // timeout already drove the Lease into this terminal.
        super::ignore_msg(self)
    }
}
