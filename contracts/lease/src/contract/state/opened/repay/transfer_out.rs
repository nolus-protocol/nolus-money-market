use cosmwasm_std::QuerierWrapper;
use serde::{Deserialize, Serialize};

use platform::batch::{Batch, Emit, Emitter};
use sdk::{
    cosmwasm_std::{Deps, DepsMut, Env, Timestamp},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{opened::RepayTrx, PaymentCoin, StateResponse},
    contract::{
        state::{self, opened::repay, Controller, Response},
        Lease,
    },
    error::ContractResult,
    event::Type,
};

use super::buy_lpn::BuyLpn;

#[derive(Serialize, Deserialize)]
pub struct TransferOut {
    lease: Lease,
    payment: PaymentCoin,
}

impl TransferOut {
    pub(in crate::contract::state) fn new(lease: Lease, payment: PaymentCoin) -> Self {
        Self { lease, payment }
    }

    fn enter_state(&self, now: Timestamp) -> ContractResult<Batch> {
        let mut sender = self.lease.dex.transfer_to(now);
        // TODO apply nls_swap_fee on the payment!
        sender.send(&self.payment)?;
        Ok(sender.into())
    }

    fn on_response(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        let emitter = self.emit_ok();
        let buy_lpn = BuyLpn::new(self.lease, self.payment);
        let batch = buy_lpn.enter(deps, env)?;

        Ok(Response::from(batch.into_response(emitter), buy_lpn))
    }

    fn emit_ok(&self) -> Emitter {
        Emitter::of_type(Type::RepaymentTransferOut)
            .emit("id", self.lease.lease.addr.clone())
            .emit_coin_dto("payment", self.payment.clone())
    }
}

impl Controller for TransferOut {
    fn enter(&self, _deps: Deps<'_>, env: Env) -> ContractResult<Batch> {
        self.enter_state(env.block.time)
    }

    fn sudo(self, deps: &mut DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response { request: _, data } => {
                deps.api.debug(&format!(
                    "[Lease][Repay][TransferOut] receive ack '{}'",
                    data.to_base64()
                ));

                self.on_response(deps.as_ref(), env)
            }
            SudoMsg::Timeout { request: _ } => self.on_timeout(deps.as_ref(), env),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => unreachable!(),
        }
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        state::on_timeout_retry(self, Type::RepaymentTransferOut, deps, env)
    }

    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        repay::query(
            self.lease.lease,
            self.payment,
            RepayTrx::TransferOut,
            now,
            querier,
        )
    }
}
