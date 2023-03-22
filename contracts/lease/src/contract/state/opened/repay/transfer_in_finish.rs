use cosmwasm_std::{Deps, Timestamp};
use serde::{Deserialize, Serialize};

use platform::batch::{Emit, Emitter};
use sdk::cosmwasm_std::{DepsMut, Env, MessageInfo, QuerierWrapper};

use crate::{
    api::{opened::RepayTrx, ExecuteMsg, LpnCoin, PaymentCoin, StateResponse},
    contract::{
        state::{
            controller,
            opened::{active::Active, repay},
            transfer_in, Controller, Response,
        },
        Contract, Lease,
    },
    error::ContractResult,
    event::Type,
};

use super::transfer_in_init::TransferInInit;

#[derive(Serialize, Deserialize)]
pub struct TransferInFinish {
    lease: Lease,
    payment: PaymentCoin,
    payment_lpn: LpnCoin,
    timeout: Timestamp,
}

impl TransferInFinish {
    pub(super) fn new(
        lease: Lease,
        payment: PaymentCoin,
        payment_lpn: LpnCoin,
        timeout: Timestamp,
    ) -> Self {
        Self {
            lease,
            payment,
            payment_lpn,
            timeout,
        }
    }

    pub(super) fn try_complete(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        let querier = &deps.querier;
        let received =
            transfer_in::check_received(&self.payment_lpn, &env.contract.address, querier)?;

        if received {
            Active::try_repay_lpn(self.lease, self.payment_lpn, querier, &env)
        } else {
            let emitter = self.emit_ok();
            if env.block.time >= self.timeout {
                let transfer_in = TransferInInit::new(self.lease, self.payment, self.payment_lpn);
                Ok(Response::from(
                    transfer_in.enter(env.block.time)?.into_response(emitter),
                    transfer_in,
                ))
            } else {
                let batch =
                    transfer_in::setup_alarm(self.lease.lease.time_alarms.clone(), env.block.time)?;
                Ok(Response::from(batch.into_response(emitter), self))
            }
        }
    }

    fn on_alarm(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        self.try_complete(deps, env)
    }

    fn emit_ok(&self) -> Emitter {
        Emitter::of_type(Type::RepaymentTransferIn)
            .emit("id", self.lease.lease.addr.clone())
            .emit_coin_dto("payment", self.payment.clone())
            .emit_coin_dto("payment-stable", self.payment_lpn.clone())
    }
}

impl Controller for TransferInFinish {
    fn execute(
        self,
        deps: &mut DepsMut<'_>,
        env: Env,
        _info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        if matches!(msg, ExecuteMsg::TimeAlarm {}) {
            self.on_alarm(deps.as_ref(), env)
        } else {
            controller::err(&format!("{:?}", msg), deps.api)
        }
    }
}

impl Contract for TransferInFinish {
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        repay::query(
            self.lease.lease,
            self.payment,
            RepayTrx::TransferInFinish,
            now,
            querier,
        )
    }
}
