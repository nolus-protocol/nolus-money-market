use serde::{Deserialize, Serialize};

use finance::coin::{self};
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{batch::Batch as LocalBatch, ica::HostAccount, trx};
use sdk::{
    cosmwasm_std::{Binary, Deps, DepsMut, Env, QuerierWrapper},
    neutron_sdk::sudo::msg::SudoMsg,
};
use swap::trx as swap_trx;

use crate::{
    api::{
        opening::OngoingTrx, DownpaymentCoin, LeaseCoin, NewLeaseForm, StateQuery, StateResponse,
    },
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

    fn enter_state(&self, querier: &QuerierWrapper<'_>) -> ContractResult<LocalBatch> {
        // TODO define struct Trx with functions build_request and decode_response -> LpnCoin
        let mut swap_trx = self.dex_account.swap(&self.deps.1, querier);
        // TODO apply nls_swap_fee on the downpayment only!
        // TODO do not add a trx if the coin is of the same lease currency
        swap_trx.swap_exact_in(&self.downpayment, &self.form.currency)?;
        swap_trx.swap_exact_in(&self.loan.principal, &self.form.currency)?;
        Ok(swap_trx.into())
    }

    fn on_response(
        self,
        resp: Binary,
        env: &Env,
        querier: &QuerierWrapper<'_>,
    ) -> ContractResult<Response> {
        // TODO transfer (downpayment - transferred_and_swapped), i.e. the nls_swap_fee to the profit
        let amount = self.decode_response(resp.as_slice())?;
        let IntoDTOResult { lease, batch } = self.form.into_lease(
            env.contract.address.clone(),
            env.block.time,
            &amount,
            querier,
            self.deps,
        )?;
        let active = Active::new(Lease {
            lease,
            dex: self.dex_account,
        });
        let emitter = active.emit_ok(env, self.downpayment, self.loan);
        Ok(Response::from(batch.into_response(emitter), active))
    }

    fn decode_response(&self, resp: &[u8]) -> ContractResult<LeaseCoin> {
        let mut resp_msgs = trx::decode_msg_responses(resp)?;
        let downpayment_amount = swap_trx::exact_amount_in_resp(&mut resp_msgs)?;
        let borrowed_amount = swap_trx::exact_amount_in_resp(&mut resp_msgs)?;

        coin::from_amount_ticker(downpayment_amount + borrowed_amount, &self.form.currency)
            .map_err(Into::into)
    }
}

impl Controller for BuyAsset {
    fn enter(&self, deps: Deps<'_>, _env: Env) -> ContractResult<LocalBatch> {
        self.enter_state(&deps.querier)
    }

    fn sudo(self, deps: &mut DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response { request: _, data } => self.on_response(data, &env, &deps.querier),
            SudoMsg::Timeout { request: _ } => todo!(),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => unreachable!(),
        }
    }

    fn query(self, _deps: Deps<'_>, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
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
