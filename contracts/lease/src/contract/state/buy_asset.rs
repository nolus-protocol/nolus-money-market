use std::fmt::Display;

use currency::lease::Osmo;
use serde::{Deserialize, Serialize};

use finance::{coin::Coin, duration::Duration};
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{
    self,
    batch::Batch as LocalBatch,
    ica::{self, Batch, HostAccount},
};
use sdk::{
    cosmwasm_std::{DepsMut, Env},
    neutron_sdk::sudo::msg::SudoMsg,
};
use swap::trx;

use crate::{
    api::{DownpaymentCoin, NewLeaseForm},
    contract::cmd::OpenLoanRespResult,
    error::ContractResult,
    lease::IntoDTOResult,
};

use super::{active::Active, Controller, Response};

#[derive(Serialize, Deserialize)]
pub struct BuyAsset {
    form: NewLeaseForm,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    dex_account: HostAccount,
    deps: (LppLenderRef, OracleRef),
}

const ICA_TRX_TIMEOUT: Duration = Duration::from_days(1);

impl BuyAsset {
    pub(super) fn new(
        form: NewLeaseForm,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        dex_account: HostAccount,
        deps: (LppLenderRef, OracleRef),
    ) -> Self {
        Self {
            form,
            downpayment,
            loan,
            dex_account,
            deps,
        }
    }

    pub(super) fn enter_state(&self) -> ContractResult<LocalBatch> {
        // querier: &QuerierWrapper,
        // deps.1.execute(cmd, querier)
        let swap_path = vec![];
        let mut batch = Batch::default();
        // TODO apply nls_swap_fee on the downpayment only!
        trx::exact_amount_in(
            &mut batch,
            self.dex_account.clone(),
            &self.downpayment,
            &swap_path,
        )?;
        trx::exact_amount_in(
            &mut batch,
            self.dex_account.clone(),
            &self.loan.principal,
            &swap_path,
        )?;
        let local_batch =
            ica::submit_transaction(&self.form.dex.connection_id, batch, "memo", ICA_TRX_TIMEOUT);

        Ok(local_batch)
    }
}

impl Controller for BuyAsset {
    fn sudo(self, deps: &mut DepsMut, env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => {
                // TODO transfer (downpayment - transferred_and_swapped), i.e. the nls_swap_fee to the profit
                // TODO parse the response to obtain the lease amount
                let amount =
                    Coin::<Osmo>::new(self.downpayment.amount() + self.loan.principal.amount())
                        .into();
                debug_assert_eq!(self.downpayment.ticker(), self.loan.principal.ticker());

                let IntoDTOResult { lease, batch } = self.form.into_lease(
                    &env.contract.address,
                    env.block.time,
                    &amount,
                    &deps.querier,
                    self.deps,
                )?;

                let next_state = Active::new(lease);
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
}

impl Display for BuyAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("buying lease assets on behalf of the ICA account at the DEX")
    }
}
