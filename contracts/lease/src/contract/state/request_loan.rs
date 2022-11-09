use std::fmt::Display;

use cosmwasm_std::MessageInfo;
use platform::batch::Batch;
use serde::{Deserialize, Serialize};

use finance::coin::CoinDTO;
use lpp::stub::lender::LppLenderRef;
use market_price_oracle::stub::OracleRef;
use sdk::cosmwasm_std::{DepsMut, Env, Reply};

use crate::{
    api::NewLeaseForm,
    contract::cmd::{OpenLoanReq, OpenLoanReqResult, OpenLoanResp},
    error::{ContractError, ContractResult},
    reply_id::ReplyId,
};

use super::{Controller, OpenIcaAccount, Response};

#[derive(Serialize, Deserialize)]
pub struct RequestLoan {
    form: NewLeaseForm,
    downpayment: CoinDTO,
    deps: (LppLenderRef, OracleRef),
}

impl RequestLoan {
    pub fn new(
        deps: &mut DepsMut,
        info: MessageInfo,
        form: NewLeaseForm,
    ) -> ContractResult<(Batch, Self)> {
        let lpp = LppLenderRef::try_new(
            deps.api.addr_validate(&form.loan.lpp)?,
            &deps.querier,
            ReplyId::OpenLoanReq.into(),
        )?;

        let oracle = OracleRef::try_from(
            deps.api.addr_validate(&form.market_price_oracle)?,
            &deps.querier,
        )
        .expect("Market Price Oracle is not deployed, or wrong address is passed!");

        let OpenLoanReqResult { batch, downpayment } = lpp.clone().execute(
            OpenLoanReq::new(&form, info.funds, oracle.clone(), &deps.querier),
            &deps.querier,
        )?;
        Ok((
            batch,
            RequestLoan {
                form,
                downpayment,
                deps: (lpp, oracle),
            },
        ))
    }
}
impl Controller for RequestLoan {
    fn reply(self, deps: &mut DepsMut, _env: Env, msg: Reply) -> ContractResult<Response> {
        let id = ReplyId::try_from(msg.id)
            .map_err(|_| ContractError::InvalidParameters("Invalid reply ID passed!".into()))?;

        match id {
            ReplyId::OpenLoanReq => {
                let loan = self
                    .deps
                    .0
                    .clone()
                    .execute(OpenLoanResp::new(msg), &deps.querier)?;

                let (batch, next_state) =
                    OpenIcaAccount::new(self.form, self.downpayment, loan, self.deps);
                Ok(Response::from(batch, next_state))
            }
        }
    }
}

impl Display for RequestLoan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("requesting a loan")
    }
}
