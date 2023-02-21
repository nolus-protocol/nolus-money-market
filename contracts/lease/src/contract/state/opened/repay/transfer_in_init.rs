use serde::{Deserialize, Serialize};

use platform::batch::Batch;
use sdk::{
    cosmwasm_std::{Deps, DepsMut, Env, QuerierWrapper, Timestamp},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{dex::ConnectionParams, opened::RepayTrx, LpnCoin, PaymentCoin, StateResponse},
    contract::{
        dex::DexConnectable,
        state::{self, opened::repay, Controller, Response},
        Lease,
    },
    error::ContractResult,
    event::Type,
};

use super::transfer_in_finish::TransferInFinish;

#[derive(Serialize, Deserialize)]
pub struct TransferInInit {
    pub(super) lease: Lease,
    pub(super) payment: PaymentCoin,
    pub(super) payment_lpn: LpnCoin,
}

impl TransferInInit {
    pub(in crate::contract::state) fn new(
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

    fn enter_state(&self, now: Timestamp) -> ContractResult<Batch> {
        let mut sender = self.lease.dex.transfer_from(now);
        sender.send(&self.payment_lpn)?;
        Ok(sender.into())
    }

    fn on_response(self, querier: &QuerierWrapper<'_>, env: &Env) -> ContractResult<Response> {
        TransferInFinish::from(self).try_complete(querier, env)
    }
}

impl DexConnectable for TransferInInit {
    fn dex(&self) -> &ConnectionParams {
        self.lease.dex()
    }
}

impl Controller for TransferInInit {
    fn enter(&self, _deps: Deps<'_>, env: Env) -> ContractResult<Batch> {
        self.enter_state(env.block.time)
    }

    fn sudo(self, deps: &mut DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => self.on_response(&deps.querier, &env),
            SudoMsg::Timeout { request: _ } => self.on_timeout(deps.as_ref(), env),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => unreachable!(),
        }
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        state::on_timeout_repair_channel(self, Type::RepaymentTransferIn, deps, env)
    }

    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        repay::query(
            self.lease.lease,
            self.payment,
            RepayTrx::TransferInInit,
            now,
            querier,
        )
    }
}
