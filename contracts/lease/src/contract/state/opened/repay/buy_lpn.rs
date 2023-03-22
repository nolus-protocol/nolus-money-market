use cosmwasm_std::Timestamp;
use serde::{Deserialize, Serialize};

use finance::{
    coin::{self},
    currency::Symbol,
};
use platform::{
    batch::{Batch as LocalBatch, Emit, Emitter},
    trx,
};
use sdk::cosmwasm_std::{Binary, Deps, Env, QuerierWrapper};
use swap::trx as swap_trx;

use crate::{
    api::{dex::ConnectionParams, opened::RepayTrx, LpnCoin, PaymentCoin, StateResponse},
    contract::{
        dex::DexConnectable,
        state::{
            self, ica_connector::Enterable, ica_post_connector::Postpone, opened::repay,
            Controller, Response,
        },
        Contract, Lease,
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

    pub(super) fn enter(&self, querier: &QuerierWrapper<'_>) -> ContractResult<LocalBatch> {
        let mut swap_trx = self.lease.dex.swap(&self.lease.lease.oracle, querier);
        swap_trx.swap_exact_in(&self.payment, self.target_currency())?;
        Ok(swap_trx.into())
    }

    fn on_response(self, resp: Binary, _deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        let emitter = self.emit_ok();
        let payment_lpn = self.decode_response(resp.as_slice())?;

        let transfer_in = TransferInInit::new(self.lease, self.payment, payment_lpn);
        let batch = transfer_in.enter(env.block.time)?;

        Ok(Response::from(batch.into_response(emitter), transfer_in))
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

impl DexConnectable for BuyLpn {
    fn dex(&self) -> &ConnectionParams {
        self.lease.dex()
    }
}

impl Enterable for BuyLpn {
    fn enter(&self, deps: Deps<'_>, _env: Env) -> ContractResult<LocalBatch> {
        self.enter(&deps.querier)
    }
}

impl Controller for BuyLpn {
    fn on_response(self, data: Binary, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        self.on_response(data, deps, env)
    }

    fn on_timeout(self, _deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        state::on_timeout_repair_channel(self, Type::BuyLpn, env)
    }
}

impl Contract for BuyLpn {
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        repay::query(self.lease.lease, self.payment, RepayTrx::Swap, now, querier)
    }
}

impl Postpone for BuyLpn {
    fn setup_alarm(
        &self,
        when: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<LocalBatch> {
        let time_alarms = self.lease.lease.time_alarms.clone();
        time_alarms.setup_alarm(when).map_err(Into::into)
    }
}
