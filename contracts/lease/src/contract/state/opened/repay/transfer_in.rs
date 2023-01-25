use cosmwasm_std::{Deps, DepsMut, Env, Timestamp};
use platform::batch::Batch;
use sdk::neutron_sdk::sudo::msg::SudoMsg;
use serde::{Deserialize, Serialize};

use crate::api::opened::RepayTrx;
use crate::api::{LpnCoin, PaymentCoin, StateQuery, StateResponse};
use crate::contract::state::opened::active::Active;
use crate::contract::state::{opened::repay, Controller, Response};
use crate::contract::Lease;
use crate::error::ContractResult;

#[derive(Serialize, Deserialize)]
pub struct TransferIn {
    lease: Lease,
    payment: PaymentCoin,
    payment_lpn: LpnCoin,
}

impl TransferIn {
    pub(in crate::contract::state::opened) fn new(
        lease: Lease,
        payment: PaymentCoin,
        payment_lpn: LpnCoin,
    ) -> Self {
        Self {
            lease,
            payment,
            payment_lpn,
        }
    }

    pub(in crate::contract::state::opened) fn enter_state(
        &self,
        now: Timestamp,
    ) -> ContractResult<Batch> {
        let mut sender = self.lease.dex.transfer_from(now);
        sender.send(&self.payment_lpn)?;
        Ok(sender.into())
    }
}

impl Controller for TransferIn {
    fn sudo(self, deps: &mut DepsMut, env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => Active::try_repay_lpn(
                self.lease,
                self.payment_lpn,
                &env.contract.address,
                &deps.querier,
                &env,
            ),
            SudoMsg::Timeout { request: _ } => todo!(),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => todo!(),
        }
    }

    fn query(self, deps: Deps, env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        repay::query(
            self.lease.lease,
            self.payment,
            RepayTrx::TransferIn,
            &deps,
            &env,
        )
    }
}
