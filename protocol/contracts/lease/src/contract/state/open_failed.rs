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

    /// Absorbs late-after-terminal callbacks. The original packet's
    /// success ack may still land here after a timeout already moved us
    /// to this terminal.
    fn on_remote_lease_callback(
        self,
        _callback: RemoteLeaseCallback<LeasePaymentCurrencies>,
        _info: MessageInfo,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
    }
}
