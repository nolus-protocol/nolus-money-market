use std::fmt::Display;

use cosmwasm_std::{DepsMut, Env, MessageInfo};
use cw2::set_contract_version;

use crate::{
    contract::open::{OpenLoanReq, OpenLoanReqResult},
    error::ContractResult,
    lease,
    msg::NewLeaseForm,
};

use super::{Controller, Response};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct NoLease {}

impl Controller for NoLease {
    fn instantiate(
        self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        form: NewLeaseForm,
    ) -> ContractResult<Response> {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        let lease = form.into_lease_dto(env.block.time, deps.api, &deps.querier)?;
        lease.store(deps.storage)?;

        let OpenLoanReqResult { batch, downpayment } = lease::execute(
            lease,
            OpenLoanReq::new(&info.funds),
            &env.contract.address,
            &deps.querier,
        )?;

        downpayment.store(deps.storage)?;

        Ok(Response::from(batch, self))
    }
}

impl Display for NoLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("lease not opened")
    }
}
