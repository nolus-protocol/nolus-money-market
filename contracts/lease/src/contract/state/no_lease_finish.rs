use std::fmt::Display;

use cosmwasm_std::{DepsMut, Env, Reply};
use finance::coin::CoinDTO;
use lpp::stub::lender::LppLenderRef;
use platform::batch::{Batch, Emit, Emitter};
use serde::{Deserialize, Serialize};

use crate::{
    contract::cmd::{OpenLoanResp, OpenLoanRespResult},
    error::{ContractError, ContractResult},
    event::TYPE,
    lease::LeaseDTO,
    msg::NewLeaseForm,
    reply_id::ReplyId,
};

use super::{Active, Controller, Response};

#[derive(Serialize, Deserialize)]
pub struct NoLeaseFinish {
    pub(super) form: NewLeaseForm,
    pub(super) lpp: LppLenderRef,
    pub(super) downpayment: CoinDTO,
}

impl Controller for NoLeaseFinish {
    fn reply(self, deps: &mut DepsMut, env: Env, msg: Reply) -> ContractResult<Response> {
        let id = ReplyId::try_from(msg.id)
            .map_err(|_| ContractError::InvalidParameters("Invalid reply ID passed!".into()))?;

        match id {
            ReplyId::OpenLoanReq => {
                let open_result = self
                    .lpp
                    .execute(OpenLoanResp::new(msg, self.downpayment), &deps.querier)?;

                // TODO pass the lpp ref
                let lease = self.form.into_lease(
                    &env.contract.address,
                    env.block.time,
                    deps.api,
                    &deps.querier,
                    open_result.lpp.clone(),
                )?;
                let emitter = build_emitter(lease.batch, &env, &lease.dto, open_result);
                Ok(Response::from(emitter, Active { lease: lease.dto }))
            }
        }
    }
}

impl Display for NoLeaseFinish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("lease open finishing")
    }
}

fn build_emitter(
    batch: Batch,
    env: &Env,
    dto: &LeaseDTO,
    open_result: OpenLoanRespResult,
) -> Emitter {
    batch
        .into_emitter(TYPE::Open)
        .emit_tx_info(env)
        .emit("id", env.contract.address.clone())
        .emit("customer", dto.customer.clone())
        .emit_percent_amount(
            "air",
            open_result.annual_interest_rate + dto.loan.annual_margin_interest(),
        )
        .emit("currency", dto.currency.clone())
        .emit("loan-pool-id", dto.loan.lpp().addr())
        .emit_coin_dto("loan", open_result.principal)
        .emit_coin_dto("downpayment", open_result.downpayment)
}
