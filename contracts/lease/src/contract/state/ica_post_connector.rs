use cosmwasm_std::{DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp};
use finance::duration::Duration;
use platform::batch::Batch;
use serde::{Deserialize, Serialize};

use crate::{
    api::ExecuteMsg,
    contract::{dex::Account, Contract},
    error::ContractResult,
};

use super::{
    controller::{self, Controller},
    ica_connector::{Enterable, IcaConnectee},
    Response, State,
};

pub(crate) trait Postpone {
    fn setup_alarm(&self, when: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<Batch>;
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PostConnector<Connectee> {
    connectee: Connectee,
    ica_account: Account,
}

impl<Connectee> PostConnector<Connectee>
where
    Connectee: Postpone,
{
    const RIGHT_AFTER_NOW: Duration = Duration::from_nanos(1);

    pub(super) fn new(connectee: Connectee, ica_account: Account) -> Self {
        Self {
            connectee,
            ica_account,
        }
    }

    pub(super) fn enter(
        &self,
        now: Timestamp,
        querier: &QuerierWrapper<'_>,
    ) -> ContractResult<Batch> {
        self.connectee
            .setup_alarm(now + Self::RIGHT_AFTER_NOW, querier)
    }
}

impl<Connectee> Controller for PostConnector<Connectee>
where
    Self: Into<State>,
    Connectee: IcaConnectee,
{
    fn execute(
        self,
        deps: &mut DepsMut<'_>,
        env: Env,
        _info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        match msg {
            ExecuteMsg::Repay() => controller::err("repay", deps.api),
            ExecuteMsg::Close() => controller::err("close", deps.api),
            ExecuteMsg::PriceAlarm() => super::ignore_msg(self)?.attach_alarm_response(&env),
            ExecuteMsg::TimeAlarm {} => {
                let next_state = self.connectee.connected(self.ica_account);
                let batch = next_state.enter(deps.as_ref(), &env)?;
                Response::from(batch, next_state).attach_alarm_response(&env)
            }
        }
    }
}

impl<Connectee> Contract for PostConnector<Connectee>
where
    Self: Into<State>,
    Connectee: Contract,
{
    fn state(
        self,
        now: cosmwasm_std::Timestamp,
        querier: &cosmwasm_std::QuerierWrapper<'_>,
    ) -> ContractResult<crate::api::StateResponse> {
        self.connectee.state(now, querier)
    }
}
