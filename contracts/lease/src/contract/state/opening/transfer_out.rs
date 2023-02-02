use cosmwasm_std::{Deps, DepsMut, Env, QuerierWrapper, Timestamp};
use sdk::neutron_sdk::sudo::msg::SudoMsg;
use serde::{Deserialize, Serialize};

use finance::zero::Zero;
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

type TransfersNb = u8;

#[derive(Serialize, Deserialize)]
pub struct TransferOut {
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
    nb_completed: TransfersNb, // have to track the responses because each transfer is a separate msg
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
            nb_completed: TransfersNb::ZERO,
        }
    }

    //TODO define a State trait with `fn enter(&self, deps: &Deps)` and
    //simplify the TransferOut::on_success return type to `impl State`
    pub(super) fn enter_state(&self, now: Timestamp) -> ContractResult<Batch> {
        debug_assert_eq!(self.nb_completed, TransfersNb::ZERO);
        let mut sender = self.dex_account.transfer_to(now);
        sender.send(&self.downpayment)?;
        sender.send(&self.loan.principal)?;
        Ok(sender.into())
    }

    fn on_response(self, querier: &QuerierWrapper) -> ContractResult<Response> {
        match self.nb_completed {
            0 => {
                let next_state = Self {
                    nb_completed: self.nb_completed + 1,
                    ..self
                };
                Ok(Response::from(Batch::default(), next_state))
            }
            1 => {
                let next_state = BuyAsset::new(
                    self.form,
                    self.dex_account,
                    self.downpayment,
                    self.loan,
                    self.deps,
                );
                let batch = next_state.enter_state(querier)?;
                Ok(Response::from(batch, next_state))
            }
            _ => unreachable!(),
        }
    }
}

impl Controller for TransferOut {
    fn sudo(self, deps: &mut DepsMut, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response { request: _, data } => {
                deps.api.debug(&format!(
                    "[Lease][TransferOut] receive ack '{}'",
                    data.to_base64()
                ));
                self.on_response(&deps.querier)
            }
            SudoMsg::Timeout { request: _ } => todo!(),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => unreachable!(),
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
