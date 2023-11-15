use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    api::LpnCoin,
    contract::{state::Response, Lease},
    error::ContractResult,
};

pub(in crate::contract::state) use self::{
    close::{Close, CloseAlgo},
    repay::{Repay, RepayAlgo},
};

mod close;
mod repay;

pub(super) trait Repayable {
    fn try_repay(
        &self,
        lease: Lease,
        amount: LpnCoin,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<Response>;
}
