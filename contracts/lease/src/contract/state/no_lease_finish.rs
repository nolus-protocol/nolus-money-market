use std::fmt::Display;

use cosmwasm_std::{DepsMut, Env, Reply};
use lpp::stub::lender::LppLenderRef;
use serde::{Deserialize, Serialize};

use crate::{
    contract::cmd::OpenLoanResp,
    error::{ContractError, ContractResult},
    lease::DownpaymentDTO,
    msg::NewLeaseForm,
    repay_id::ReplyId,
};

use super::{Active, Controller, Response};

#[derive(Serialize, Deserialize)]
pub struct NoLeaseFinish {
    pub(super) form: NewLeaseForm,
    pub(super) lpp: LppLenderRef,
    pub(super) downpayment: DownpaymentDTO,
}

impl Controller for NoLeaseFinish {
    fn reply(self, deps: &mut DepsMut, env: Env, msg: Reply) -> ContractResult<Response> {
        let id = ReplyId::try_from(msg.id)
            .map_err(|_| ContractError::InvalidParameters("Invalid reply ID passed!".into()))?;

        match id {
            ReplyId::OpenLoanReq => {
                let emitter = self.lpp.execute(
                    OpenLoanResp::new(msg, &self.form, self.downpayment, &env),
                    &deps.querier,
                )?;

                let lease = self
                    .form
                    .into_lease_dto(env.block.time, deps.api, &deps.querier)?;
                //TODO form -> Lease, self.initial_alarm_schedule(account.balance()?, now)?;
                Ok(Response::from(emitter, Active { lease }))
            }
        }
    }
}

impl Display for NoLeaseFinish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("lease open finishing")
    }
}
