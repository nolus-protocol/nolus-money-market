use std::fmt::Display;

use cosmwasm_std::{DepsMut, Env, MessageInfo};
use cw2::set_contract_version;
use lpp::stub::lender::LppLenderRef;
use market_price_oracle::stub::OracleRef;
use serde::{Deserialize, Serialize};

use crate::{
    contract::cmd::{OpenLoanReq, OpenLoanReqResult},
    error::ContractResult,
    msg::NewLeaseForm,
    reply_id::ReplyId,
};

use super::{Controller, RequestLoan, Response};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize)]
pub struct NoLease {}

impl Controller for NoLease {
    fn instantiate(
        self,
        deps: &mut DepsMut,
        _env: Env,
        info: MessageInfo,
        form: NewLeaseForm,
    ) -> ContractResult<Response> {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

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

        Ok(Response::from(
            batch,
            RequestLoan {
                form,
                lpp,
                oracle,
                downpayment,
            },
        ))
    }
}

impl Display for NoLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("lease not opened")
    }
}
