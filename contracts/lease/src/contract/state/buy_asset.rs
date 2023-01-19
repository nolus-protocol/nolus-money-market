use std::fmt::Display;

use cosmwasm_std::Deps;
use serde::{Deserialize, Serialize};

use currency::{lease::Osmo, native::Nls};
use finance::{
    coin::{Coin, CoinDTO},
    currency::Group,
    duration::Duration,
};
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{
    batch::Batch as LocalBatch,
    ica::{self, Batch, HostAccount},
};
use sdk::{
    cosmwasm_std::{DepsMut, Env, QuerierWrapper},
    neutron_sdk::sudo::msg::SudoMsg,
};
use swap::trx;

use crate::{
    api::{opening::OngoingTrx, DownpaymentCoin, NewLeaseForm, StateQuery, StateResponse},
    contract::cmd::OpenLoanRespResult,
    error::ContractResult,
    lease::IntoDTOResult,
};

use super::{active::Active, Controller, Response};

const ICA_TRX_TIMEOUT: Duration = Duration::from_days(1);
const ICA_TRX_ACK_TIP: Coin<Nls> = Coin::new(1);
const ICA_TRX_TIMEOUT_TIP: Coin<Nls> = ICA_TRX_ACK_TIP;

#[derive(Serialize, Deserialize)]
pub struct BuyAsset {
    form: NewLeaseForm,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    dex_account: HostAccount,
    deps: (LppLenderRef, OracleRef),
}

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

    pub(super) fn enter_state(&self, querier: &QuerierWrapper) -> ContractResult<LocalBatch> {
        let mut batch = Batch::default();
        // TODO apply nls_swap_fee on the downpayment only!
        self.add_swap_trx(&self.downpayment, querier, &mut batch)?;
        self.add_swap_trx(&self.loan.principal, querier, &mut batch)?;
        let local_batch = ica::submit_transaction(
            &self.form.dex.connection_id,
            batch,
            "memo",
            ICA_TRX_TIMEOUT,
            ICA_TRX_ACK_TIP,
            ICA_TRX_TIMEOUT_TIP,
        );

        Ok(local_batch)
    }

    fn add_swap_trx<G>(
        &self,
        coin: &CoinDTO<G>,
        querier: &QuerierWrapper,
        batch: &mut Batch,
    ) -> ContractResult<()>
    where
        G: Group,
    {
        //TODO do not add a trx if the coin is of the same lease currency
        let swap_path =
            self.deps
                .1
                .swap_path(coin.ticker().into(), self.form.currency.clone(), querier)?;
        trx::exact_amount_in(batch, self.dex_account.clone(), coin, &swap_path)?;
        Ok(())
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

    fn query(self, _deps: Deps, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        Ok(StateResponse::Opening {
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: OngoingTrx::BuyAsset {
                ica_account: self.dex_account.into(),
            },
        })
    }
}

impl Display for BuyAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("buying lease assets on behalf of the ICA account at the DEX")
    }
}
