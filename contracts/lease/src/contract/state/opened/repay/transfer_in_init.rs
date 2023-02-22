use cosmwasm_std::Binary;
use serde::{Deserialize, Serialize};

use platform::batch::Batch;
use sdk::cosmwasm_std::{Deps, Env, QuerierWrapper, Timestamp};

use crate::{
    api::{dex::ConnectionParams, opened::RepayTrx, LpnCoin, PaymentCoin, StateResponse},
    contract::{
        dex::DexConnectable,
        state::{self, opened::repay, Controller, Response},
        Contract, Lease,
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

    fn on_response(self, _data: Binary, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        self.on_response(&deps.querier, &env)
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        state::on_timeout_repair_channel(self, Type::RepaymentTransferIn, deps, env)
    }
}

impl Contract for TransferInInit {
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
