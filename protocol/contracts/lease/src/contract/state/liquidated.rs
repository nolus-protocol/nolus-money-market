use currencies::PaymentGroup;
use platform::bank;
use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{api::query::StateResponse, error::ContractResult};

use super::{Handler, Response};

#[derive(Serialize, Deserialize, Default)]
pub struct Liquidated {}

impl Handler for Liquidated {
    fn state(self, _now: Timestamp, _querier: QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        Ok(StateResponse::Liquidated())
    }

    fn on_time_alarm(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
    }
    fn on_price_alarm(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
    }

    fn heal(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        bank::bank_send_all::<PaymentGroup>(&env.contract.address, info.sender, querier)
            .map_err(Into::into)
            .map(|msgs| Response::from(msgs, self))
    }
}
