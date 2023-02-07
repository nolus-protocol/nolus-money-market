use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp};
use finance::coin::{Coin, WithCoin, WithCoinResult};
use finance::currency::Currency;
use finance::duration::Duration;
use platform::bank::{self};
use platform::batch::Batch;
use sdk::cosmwasm_ext::Response as CwResponse;
use serde::{Deserialize, Serialize};
use timealarms::stub::{TimeAlarms, TimeAlarmsRef, WithTimeAlarms};

use crate::api::opened::RepayTrx;
use crate::api::{ExecuteMsg, LpnCoin, PaymentCoin, StateQuery, StateResponse};
use crate::contract::state::opened::active::Active;
use crate::contract::state::{opened::repay, Controller, Response};
use crate::contract::{state, Lease};
use crate::error::{ContractError, ContractResult};

use super::transfer_in_init::TransferInInit;

const POLLING_INTERVAL: Duration = Duration::from_secs(5);

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
        struct CheckBalance<'a> {
            account: &'a Addr,
            querier: &'a QuerierWrapper<'a>,
        }
        impl<'a> WithCoin for CheckBalance<'a> {
            type Output = bool;
            type Error = ContractError;

            fn on<C>(&self, expected_payment: Coin<C>) -> WithCoinResult<Self>
            where
                C: Currency,
            {
                let received = bank::balance(self.account, self.querier)? >= expected_payment;
                Ok(received)
            }
        }
        let received = self.payment_lpn.with_coin(CheckBalance {
            account: &env.contract.address,
            querier,
        })?;

        if received {
            Active::try_repay_lpn(self.lease, self.payment_lpn, querier, env)
        } else {
            let batch = self.enter_state(self.lease.lease.time_alarms.clone(), env.block.time)?;
            Ok(Response::from::<CwResponse, _>(batch.into(), self))
        }
    }

    fn enter_state(&self, time_alarms: TimeAlarmsRef, now: Timestamp) -> ContractResult<Batch> {
        struct SetupAlarm(Timestamp);
        impl WithTimeAlarms for SetupAlarm {
            type Output = Batch;
            type Error = ContractError;

            fn exec<TA>(self, mut time_alarms: TA) -> Result<Self::Output, Self::Error>
            where
                TA: TimeAlarms,
            {
                time_alarms.add_alarm(self.0 + POLLING_INTERVAL)?;
                Ok(time_alarms.into().batch)
            }
        }

        time_alarms.execute(SetupAlarm(now))
    }

    fn on_alarm(self, querier: &QuerierWrapper, env: &Env) -> ContractResult<Response> {
        self.try_complete(querier, env)
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
        if let ExecuteMsg::TimeAlarm(_) = msg {
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
