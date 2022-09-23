use std::fmt::Display;

use cosmwasm_std::{DepsMut, Env, Reply};
use serde::{Deserialize, Serialize};

use platform::bank::BankStub;

use crate::lease::stub;
use crate::{
    contract::open::OpenLoanResp,
    error::{ContractError, ContractResult},
    lease::DownpaymentDTO,
    msg::NewLeaseForm,
    repay_id::ReplyId,
};

use super::{Active, Controller, Response};

#[derive(Serialize, Deserialize)]
pub struct NoLeaseFinish {
    pub(super) form: NewLeaseForm,
    pub(super) downpayment: DownpaymentDTO,
}

impl Controller for NoLeaseFinish {
    fn reply(self, deps: &mut DepsMut, env: Env, msg: Reply) -> ContractResult<Response> {
        // TODO swap the received loan and the downpayment to lease.currency
        let lease = self
            .form
            .into_lease_dto(env.block.time, deps.api, &deps.querier)?;
        let lease_cloned = lease.clone();

        let account = BankStub::my_account(&env, &deps.querier);

        let id = ReplyId::try_from(msg.id)
            .map_err(|_| ContractError::InvalidParameters("Invalid reply ID passed!".into()))?;

        match id {
            ReplyId::OpenLoanReq => {
                let emitter = stub::execute(
                    lease,
                    OpenLoanResp::new(msg, self.downpayment, account, &env),
                    &env.contract.address,
                    &deps.querier,
                )?;

                Ok(Response::from(
                    emitter,
                    Active {
                        lease: lease_cloned,
                    },
                ))
            }
        }
    }
}

impl Display for NoLeaseFinish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("lease open finishing")
    }
}
