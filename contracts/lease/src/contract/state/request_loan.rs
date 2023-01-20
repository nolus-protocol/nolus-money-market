use serde::{Deserialize, Serialize};

use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::batch::Batch;
use sdk::{
    cosmwasm_std::MessageInfo,
    cosmwasm_std::{DepsMut, Env, Reply},
};

use crate::{
    api::{DownpaymentCoin, NewLeaseForm},
    contract::cmd::{OpenLoanReq, OpenLoanReqResult, OpenLoanResp},
    error::{ContractError, ContractResult},
    reply_id::ReplyId,
};

use super::{Controller, OpenIcaAccount, Response};

#[derive(Serialize, Deserialize)]
pub struct RequestLoan {
    form: NewLeaseForm,
    downpayment: DownpaymentCoin,
    deps: (LppLenderRef, OracleRef),
}

impl RequestLoan {
    pub fn new(
        deps: &mut DepsMut,
        info: MessageInfo,
        form: NewLeaseForm,
    ) -> ContractResult<(Batch, Self)> {
        let lpp = LppLenderRef::try_new(
            form.loan.lpp.clone(),
            &deps.querier,
            ReplyId::OpenLoanReq.into(),
        )?;

        let oracle = OracleRef::try_from(form.market_price_oracle.clone(), &deps.querier)
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

                let next_state = OpenIcaAccount::new(self.form, self.downpayment, loan, self.deps);
                let batch = next_state.enter_state();
                Ok(Response::from(batch, next_state))
            }
        }
    }

    fn query(self,_deps:cosmwasm_std::Deps,_env:Env,_msg:crate::api::StateQuery) -> ContractResult<crate::api::StateResponse> {
        unreachable!()
    }
}
