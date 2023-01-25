use cosmwasm_std::Deps;
use currency::lpn::Usdc;
use finance::coin::Coin;
use serde::{Deserialize, Serialize};

use platform::batch::Batch as LocalBatch;
use sdk::{
    cosmwasm_std::{DepsMut, Env, QuerierWrapper},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{opened::RepayTrx, PaymentCoin, StateQuery, StateResponse},
    contract::{
        state::{opened::repay, Controller, Response},
        Lease,
    },
    error::ContractResult,
};

use super::transfer_in::TransferIn;

#[derive(Serialize, Deserialize)]
pub struct BuyLpn {
    lease: Lease,
    payment: PaymentCoin,
}

impl BuyLpn {
    pub(super) fn new(lease: Lease, payment: PaymentCoin) -> Self {
        Self { lease, payment }
    }

    pub(super) fn enter_state(&self, querier: &QuerierWrapper) -> ContractResult<LocalBatch> {
        let mut swap_trx = self.lease.dex.swap(&self.lease.lease.oracle, querier);
        swap_trx.swap_exact_in(&self.payment, self.lease.lease.loan.lpp().currency())?;
        Ok(swap_trx.into())
    }
}

impl Controller for BuyLpn {
    fn sudo(self, _deps: &mut DepsMut, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => {
                // TODO init payment_lpn with the output of the swap
                let payment_lpn = Coin::<Usdc>::from(1).into();
                let next_state = TransferIn::new(self.lease, self.payment, payment_lpn);
                let batch = next_state.enter_state(_env.block.time)?;
                Ok(Response::from(batch, next_state))
            }
            SudoMsg::Timeout { request: _ } => todo!(),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => todo!(),
        }
    }

    fn query(self, deps: Deps, env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        repay::query(self.lease.lease, self.payment, RepayTrx::Swap, &deps, &env)
    }
}
