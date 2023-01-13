use std::fmt::Display;

use cosmwasm_std::{Deps, Timestamp};
use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{bank_ibc::local::Sender, batch::Batch, ica::HostAccount};
use sdk::{
    cosmwasm_std::{DepsMut, Env},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{opening::OngoingTrx, DownpaymentCoin, NewLeaseForm, StateQuery, StateResponse},
    contract::cmd::OpenLoanRespResult,
    error::ContractResult,
};

use super::{buy_asset::BuyAsset, Controller, Response};

#[derive(Serialize, Deserialize)]
pub struct TransferOut {
    form: NewLeaseForm,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    dex_account: HostAccount,
    deps: (LppLenderRef, OracleRef),
}

const ICA_TRANSFER_TIMEOUT: Duration = Duration::from_secs(60);

impl TransferOut {
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

    pub(super) fn enter_state(&self, now: Timestamp) -> ContractResult<Batch> {
        let mut ibc_sender = Sender::new(
            &self.form.dex.transfer_channel.local_endpoint,
            self.dex_account.clone(),
            now + ICA_TRANSFER_TIMEOUT,
        );
        // TODO apply nls_swap_fee on the downpayment only!
        ibc_sender.send(&self.downpayment)?;
        ibc_sender.send(&self.loan.principal)?;

        Ok(ibc_sender.into())
    }
}

impl Controller for TransferOut {
    fn sudo(self, deps: &mut DepsMut, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => {
                let next_state = BuyAsset::new(
                    self.form,
                    self.downpayment,
                    self.loan,
                    self.dex_account,
                    self.deps,
                );
                let batch = next_state.enter_state(&deps.querier)?;
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

    fn query(self, _deps: Deps, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        Ok(StateResponse::Opening {
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: OngoingTrx::TransferOut {
                ica_account: self.dex_account.into(),
            },
        })
    }
}

impl Display for TransferOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("transferring assets to the ICA account at the DEX")
    }
}
