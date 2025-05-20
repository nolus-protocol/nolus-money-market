use access_control::GrantedAddress;
use serde::{Deserialize, Serialize};

use platform::batch::Batch;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    api::{
        FinalizerExecuteMsg,
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
    pub(super) fn try_new(addr: Addr, querier: QuerierWrapper<'_>) -> ContractResult<Self> {
        use platform::contract;

        contract::validate_addr(querier, &addr)
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

    pub(super) fn check_assess(
        &self,
        caller: Addr,
    ) -> ContractResult<()> {
        access_control::check(&GrantedAddress::new(&self.addr), &caller)
        .map_err(ContractError::Unauthorized)
    }
}
