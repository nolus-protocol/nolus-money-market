use cosmwasm_std::Deps;
use serde::{Deserialize, Serialize};

use currency::lease::Osmo;
use finance::coin::Coin;
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{batch::Batch as LocalBatch, ica::HostAccount};
use sdk::{
    cosmwasm_std::{DepsMut, Env, QuerierWrapper},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{opening::OngoingTrx, DownpaymentCoin, NewLeaseForm, StateQuery, StateResponse},
    contract::{
        cmd::OpenLoanRespResult,
        state::{opened::active::Active, Controller, Response},
        Lease,
    },
    dex::Account,
    error::ContractResult,
    lease::IntoDTOResult,
};

#[derive(Serialize, Deserialize)]
pub struct BuyAsset {
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
}

impl BuyAsset {
    pub(super) fn new(
        form: NewLeaseForm,
        dex_account: Account,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppLenderRef, OracleRef),
    ) -> Self {
        Self {
            form,
            dex_account,
            downpayment,
            loan,
            deps,
        }
    }

    pub(super) fn enter_state(&self, querier: &QuerierWrapper) -> ContractResult<LocalBatch> {
        let mut swap_trx = self.dex_account.swap(&self.deps.1, querier);
        // TODO apply nls_swap_fee on the downpayment only!
        // TODO do not add a trx if the coin is of the same lease currency
        swap_trx.swap_exact_in(&self.downpayment, &self.form.currency)?;
        swap_trx.swap_exact_in(&self.loan.principal, &self.form.currency)?;
        Ok(swap_trx.into())
    }
}

impl Controller for BuyAsset {
    fn sudo(self, deps: &mut DepsMut, env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response { request: _, data } => {
                deps.api
                    .debug("!!!!!!!!!!       SWAP Result        !!!!!!!!!");
                deps.api.debug(
                    std::str::from_utf8(data.as_slice())
                        .expect("the data should be a valid string"),
                );
                // TODO transfer (downpayment - transferred_and_swapped), i.e. the nls_swap_fee to the profit
                // TODO parse the response to obtain the lease amount
                let amount =
                    Coin::<Osmo>::new(self.downpayment.amount() + self.loan.principal.amount())
                        .into();

                let IntoDTOResult { lease, batch } = self.form.into_lease(
                    &env.contract.address,
                    env.block.time,
                    &amount,
                    &deps.querier,
                    self.deps,
                )?;
                let next_state = Active::new(Lease {
                    lease,
                    dex: self.dex_account,
                });
                let emitter = next_state.enter_state(batch, &env, self.downpayment, self.loan);
                Ok(Response::from(emitter, next_state))
            }
            SudoMsg::Timeout { request: _ } => todo!(),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => todo!(),
        }
    }

    fn query(self, _deps: Deps, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        Ok(StateResponse::Opening {
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: OngoingTrx::BuyAsset {
                ica_account: HostAccount::from(self.dex_account).into(),
            },
        })
    }
}
