use cosmwasm_std::{Deps, DepsMut, Env, Timestamp};
use sdk::neutron_sdk::sudo::msg::SudoMsg;
use serde::{Deserialize, Serialize};

use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{batch::Batch, ica::HostAccount};

use crate::{
    api::{opening::OngoingTrx, DownpaymentCoin, NewLeaseForm, StateQuery, StateResponse},
    contract::{
        cmd::OpenLoanRespResult,
        state::{BuyAsset, Controller, Response},
    },
    dex::Account,
    error::ContractResult,
};

#[derive(Serialize, Deserialize)]
pub struct TransferOut {
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
}

impl TransferOut {
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

    //TODO define a State trait with `fn enter(&self, deps: &Deps)` and
    //simplify the TransferOut::on_success return type to `impl State`
    pub(super) fn enter_state(&self, now: Timestamp) -> ContractResult<Batch> {
        let mut sender = self.dex_account.transfer_to(now);
        sender.send(&self.downpayment)?;
        sender.send(&self.loan.principal)?;
        Ok(sender.into())
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
                    self.dex_account,
                    self.downpayment,
                    self.loan,
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
                ica_account: HostAccount::from(self.dex_account).into(),
            },
        })
    }
}
