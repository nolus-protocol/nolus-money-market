use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    contract::{Lease, state::Response},
    error::ContractResult,
    finance::LpnCoinDTO,
};

pub(in super::super) use self::{
    close::{Close, CloseAlgo},
    repay::{Repay, RepayAlgo},
};

mod close;
mod repay;

pub(super) trait Repayable {
    fn try_repay(
        &self,
        lease: Lease,
        amount: LpnCoinDTO,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<Response>;
}
