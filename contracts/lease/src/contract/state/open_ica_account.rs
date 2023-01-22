use cosmwasm_std::Deps;
use serde::{Deserialize, Serialize};

use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{batch::Batch, ica};
use sdk::{
    cosmwasm_std::{DepsMut, Env},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{opening::OngoingTrx, DownpaymentCoin, NewLeaseForm, StateQuery, StateResponse},
    contract::{cmd::OpenLoanRespResult, state::transfer_out::Controller as TransferOutController},
    error::ContractResult,
};

use super::{opening::transfer_out::TransferOut, Controller, Response};

#[derive(Serialize, Deserialize)]
pub struct OpenIcaAccount {
    form: NewLeaseForm,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
}

impl OpenIcaAccount {
    pub(super) fn new(
        form: NewLeaseForm,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppLenderRef, OracleRef),
    ) -> Self {
        Self {
            form,
            downpayment,
            loan,
            deps,
        }
    }

    pub(super) fn enter_state(&self) -> Batch {
        ica::register_account(&self.form.dex.connection_id)
    }
}

impl Controller for OpenIcaAccount {
    fn sudo(self, _deps: &mut DepsMut, env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::OpenAck {
                port_id: _,
                channel_id: _,
                counterparty_channel_id: _,
                counterparty_version,
            } => {
                let this_addr = env.contract.address;
                let dex_account = ica::parse_register_response(&counterparty_version)?;

                let next_state = TransferOutController::new(TransferOut::new(
                    self.form,
                    self.downpayment,
                    self.loan,
                    dex_account,
                    self.deps,
                ));
                let batch = next_state.enter_state(this_addr, env.block.time)?;
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
            in_progress: OngoingTrx::OpenIcaAccount {},
        })
    }
}
