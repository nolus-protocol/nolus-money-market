use serde::{Deserialize, Serialize};

use platform::{batch::Batch, contract::Validator};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    api::{
        FinalizerExecuteMsg,
        authz::{AccessCheck, AccessGranted},
        limits::{MaxSlippage, PositionLimits},
    },
    error::{ContractError, ContractResult},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LeasesRef {
    addr: Addr,
}

impl LeasesRef {
    pub(super) fn try_new<V>(addr: Addr, addr_validator: &V) -> ContractResult<Self>
    where
        V: Validator,
    {
        addr_validator
            .check_contract(&addr)
            .map(|()| Self { addr })
            .map_err(Into::into)
    }

    pub(super) fn finalize_lease(&self, customer: Addr) -> ContractResult<Batch> {
        let mut msgs = Batch::default();
        msgs.schedule_execute_wasm_no_reply_no_funds(
            self.addr.clone(),
            &FinalizerExecuteMsg::FinalizeLease { customer },
        )
        .map(|()| msgs)
        .map_err(Into::into)
    }

    pub(super) fn max_slippage(&self, querier: QuerierWrapper<'_>) -> ContractResult<MaxSlippages> {
        let query = PositionLimits::MaxSlippages {};
        querier
            .query_wasm_smart(self.addr.clone(), &query)
            .map_err(ContractError::PositionLimitsQuery)
    }

    pub(super) fn check_access(
        &self,
        caller: Addr,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<()> {
        let query = AccessCheck::AnomalyResolution { by: caller };
        querier
            .query_wasm_smart(self.addr.clone(), &query)
            .map_err(ContractError::CheckAccessQuery)
            .and_then(|access: AccessGranted| match access {
                AccessGranted::No => Err(ContractError::Unauthorized(
                    access_control::error::Error::Unauthorized {},
                )),
                AccessGranted::Yes => Ok(()),
            })
    }
}
