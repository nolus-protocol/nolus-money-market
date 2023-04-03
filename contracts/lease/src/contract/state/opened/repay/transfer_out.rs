use serde::{Deserialize, Serialize};

use platform::batch::{Batch, Emit, Emitter};
use sdk::cosmwasm_std::{Binary, Deps, Env, QuerierWrapper, Timestamp};

use crate::{
    api::{opened::RepayTrx, PaymentCoin, StateResponse},
    contract::{
        state::{self, ica_connector::Enterable, opened::repay, Controller, Response},
        Contract, Lease,
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

    pub(in crate::contract::state::opened) fn enter(
        &self,
        now: Timestamp,
    ) -> ContractResult<Batch> {
        let mut sender = self.lease.dex.transfer_to(now);
        // TODO apply nls_swap_fee on the payment!
        sender.send(&self.payment)?;
        Ok(sender.into())
    }

    fn on_response(self, deps: Deps<'_>, _env: Env) -> ContractResult<Response> {
        let emitter = self.emit_ok();
        let buy_lpn = BuyLpn::new(self.lease, self.payment);
        let batch = buy_lpn.enter(&deps.querier)?;

        Ok(Response::from(batch.into_response(emitter), buy_lpn))
    }

    fn emit_ok(&self) -> Emitter {
        Emitter::of_type(Type::RepaymentTransferOut)
            .emit("id", self.lease.lease.addr.clone())
            .emit_coin_dto("payment", self.payment.clone())
    }
}

impl Enterable for TransferOut {
    fn enter(&self, _deps: Deps<'_>, env: &Env) -> ContractResult<Batch> {
        self.enter(env.block.time)
    }
}

impl Controller for TransferOut {
    fn on_response(self, _data: Binary, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        self.on_response(deps, env)
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        state::on_timeout_retry(self, Type::RepaymentTransferOut, deps, env)
    }
}

impl Contract for TransferOut {
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
