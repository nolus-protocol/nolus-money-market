use std::fmt::Display;

use cosmwasm_std::{DepsMut, Env, Reply};
use platform::bank::BankStub;

use crate::{
    contract::open::OpenLoanResp,
    error::{ContractError, ContractResult},
    lease::{self, DownpaymentDTO, LeaseDTO},
    repay_id::ReplyId,
};

use super::{Controller, Response};

pub struct NoLeaseFinish {}

impl Controller for NoLeaseFinish {
    fn reply(self, deps: DepsMut, env: Env, msg: Reply) -> ContractResult<Response> {
        // TODO swap the received loan and the downpayment to lease.currency
        let lease = LeaseDTO::load(deps.storage)?;

        let account = BankStub::my_account(&env, &deps.querier);

        let id = ReplyId::try_from(msg.id)
            .map_err(|_| ContractError::InvalidParameters("Invalid reply ID passed!".into()))?;

        match id {
            ReplyId::OpenLoanReq => {
                let downpayment = DownpaymentDTO::remove(deps.storage)?;

                let emitter = lease::execute(
                    lease,
                    OpenLoanResp::new(msg, downpayment, account, &env),
                    &env.contract.address,
                    &deps.querier,
                )?;

                Ok(Response::from(emitter, self))
            }
        }
    }
}

impl Display for NoLeaseFinish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("lease open finishing")
    }
}
