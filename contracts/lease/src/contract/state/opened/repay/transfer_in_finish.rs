use cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, QuerierWrapper};
use platform::batch::{Emit, Emitter};
use serde::{Deserialize, Serialize};

use crate::api::opened::RepayTrx;
use crate::api::{ExecuteMsg, LpnCoin, PaymentCoin, StateQuery, StateResponse};
use crate::contract::state::opened::active::Active;
use crate::contract::state::transfer_in;
use crate::contract::state::{opened::repay, Controller, Response};
use crate::contract::{state, Lease};
use crate::error::ContractResult;
use crate::event::Type;

use super::transfer_in_init::TransferInInit;

#[derive(Serialize, Deserialize)]
pub struct TransferInFinish {
    lease: Lease,
    payment: PaymentCoin,
    payment_lpn: LpnCoin,
}

impl TransferInFinish {
    pub(super) fn try_complete(
        self,
        querier: &QuerierWrapper,
        env: &Env,
    ) -> ContractResult<Response> {
        let received =
            transfer_in::check_received(&self.payment_lpn, &env.contract.address, querier)?;

        if received {
            Active::try_repay_lpn(self.lease, self.payment_lpn, querier, env)
        } else {
            let emitter = self.emit_ok();
            let batch =
                transfer_in::setup_alarm(self.lease.lease.time_alarms.clone(), env.block.time)?;
            Ok(Response::from(batch.into_response(emitter), self))
        }
    }

    fn on_alarm(self, querier: &QuerierWrapper, env: &Env) -> ContractResult<Response> {
        self.try_complete(querier, env)
    }

    fn emit_ok(&self) -> Emitter {
        Emitter::of_type(Type::RepaymentTransferIn)
            .emit("id", self.lease.lease.addr.clone())
            .emit_coin_dto("payment", self.payment.clone())
            .emit_coin_dto("payment-stable", self.payment_lpn.clone())
    }
}

impl From<TransferInInit> for TransferInFinish {
    fn from(init: TransferInInit) -> Self {
        Self {
            lease: init.lease,
            payment: init.payment,
            payment_lpn: init.payment_lpn,
        }
    }
}

impl Controller for TransferInFinish {
    fn execute(
        self,
        deps: &mut DepsMut,
        env: Env,
        _info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        if matches!(msg, ExecuteMsg::TimeAlarm {}) {
            self.on_alarm(&deps.querier, &env)
        } else {
            state::err(&format!("{:?}", msg))
        }
    }

    fn query(self, deps: Deps, env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        repay::query(
            self.lease.lease,
            self.payment,
            RepayTrx::TransferInFinish,
            &deps,
            &env,
        )
    }
}
