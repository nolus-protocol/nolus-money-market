use serde::{Deserialize, Serialize};

use finance::{
    coin::{self},
    currency::Symbol,
};
use platform::{
    batch::{Batch as LocalBatch, Emit, Emitter},
    trx,
};
use sdk::{
    cosmwasm_std::{Binary, Deps, DepsMut, Env, QuerierWrapper, Timestamp},
    neutron_sdk::sudo::msg::SudoMsg,
};
use swap::trx as swap_trx;

use crate::{
    api::{opened::RepayTrx, LpnCoin, PaymentCoin, StateQuery, StateResponse},
    contract::{
        state::{opened::repay, Controller, Response},
        Lease,
    },
    error::ContractResult,
    event::Type,
};

use super::transfer_in_init::TransferInInit;

#[derive(Serialize, Deserialize)]
pub struct BuyLpn {
    lease: Lease,
    payment: PaymentCoin,
}

impl BuyLpn {
    pub(in crate::contract::state) fn new(lease: Lease, payment: PaymentCoin) -> Self {
        Self { lease, payment }
    }

    pub(super) fn enter_state(&self, querier: &QuerierWrapper<'_>) -> ContractResult<LocalBatch> {
        let mut swap_trx = self.lease.dex.swap(&self.lease.lease.oracle, querier);
        swap_trx.swap_exact_in(&self.payment, self.target_currency())?;
        Ok(swap_trx.into())
    }

    fn on_response(self, resp: Binary, now: Timestamp) -> ContractResult<Response> {
        let emitter = self.emit_ok();
        let payment_lpn = self.decode_response(resp.as_slice())?;

        let next_state = TransferInInit::new(self.lease, self.payment, payment_lpn);
        let batch = next_state.enter_state(now)?;

        Ok(Response::from(batch.into_response(emitter), next_state))
    }

    fn decode_response(&self, resp: &[u8]) -> ContractResult<LpnCoin> {
        let mut resp_msgs = trx::decode_msg_responses(resp)?;
        let payment_amount = swap_trx::exact_amount_in_resp(&mut resp_msgs)?;

        coin::from_amount_ticker(payment_amount, self.target_currency()).map_err(Into::into)
    }

    fn target_currency(&self) -> Symbol<'_> {
        self.lease.lease.loan.lpp().currency()
    }

    fn emit_ok(&self) -> Emitter {
        Emitter::of_type(Type::BuyLpn)
            .emit("id", self.lease.lease.addr.clone())
            .emit_coin_dto("payment", self.payment.clone())
    }
}

impl Controller for BuyLpn {
    fn sudo(self, _deps: &mut DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response { request: _, data } => self.on_response(data, env.block.time),
            SudoMsg::Timeout { request: _ } => todo!(),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => unreachable!(),
        }
    }

    fn query(self, deps: Deps<'_>, env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        repay::query(self.lease.lease, self.payment, RepayTrx::Swap, &deps, &env)
    }
}
