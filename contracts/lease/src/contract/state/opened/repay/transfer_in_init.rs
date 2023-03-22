use cosmwasm_std::Binary;
use serde::{Deserialize, Serialize};

use platform::batch::Batch;
use sdk::cosmwasm_std::{Deps, Env, QuerierWrapper, Timestamp};

use crate::{
    api::{dex::ConnectionParams, opened::RepayTrx, LpnCoin, PaymentCoin, StateResponse},
    contract::{
        dex::{self, DexConnectable},
        state::{
            self, ica_connector::Enterable, ica_post_connector::Postpone, opened::repay,
            Controller, Response,
        },
        Contract, Lease,
    },
    error::ContractResult,
    event::Type,
};

use super::transfer_in_finish::TransferInFinish;

#[derive(Serialize, Deserialize)]
pub struct TransferInInit {
    lease: Lease,
    payment: PaymentCoin,
    payment_lpn: LpnCoin,
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

    pub(super) fn enter(&self, now: Timestamp) -> ContractResult<Batch> {
        let mut sender = self.lease.dex.transfer_from(now);
        sender.send(&self.payment_lpn)?;
        Ok(sender.into())
    }

    fn on_response(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        let finish = TransferInFinish::new(
            self.lease,
            self.payment,
            self.payment_lpn,
            env.block.time + dex::IBC_TIMEOUT,
        );
        finish.try_complete(deps, env)
    }
}

impl DexConnectable for TransferInInit {
    fn dex(&self) -> &ConnectionParams {
        self.lease.dex()
    }
}

impl Enterable for TransferInInit {
    fn enter(&self, _deps: Deps<'_>, env: Env) -> ContractResult<Batch> {
        self.enter(env.block.time)
    }
}

impl Controller for TransferInInit {
    fn on_response(self, _data: Binary, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        self.on_response(deps, env)
    }

    fn on_timeout(self, _deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        state::on_timeout_repair_channel(self, Type::RepaymentTransferIn, env)
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

impl Postpone for TransferInInit {
    fn setup_alarm(&self, when: Timestamp, _querier: &QuerierWrapper<'_>) -> ContractResult<Batch> {
        let time_alarms = self.lease.lease.time_alarms.clone();
        time_alarms.setup_alarm(when).map_err(Into::into)
    }
}
