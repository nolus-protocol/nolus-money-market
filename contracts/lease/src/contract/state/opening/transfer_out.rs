use cosmwasm_std::{Addr, Deps, DepsMut, Env, Timestamp};
use currency::native::Nls;
use finance::{coin::Coin, duration::Duration};
use sdk::neutron_sdk::sudo::msg::SudoMsg;
use serde::{Deserialize, Serialize};

use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{bank_ibc::local::Sender, batch::Batch, ica::HostAccount};

use crate::{
    api::{opening::OngoingTrx, DownpaymentCoin, NewLeaseForm, StateQuery, StateResponse},
    contract::{
        cmd::OpenLoanRespResult,
        state::{BuyAsset, Controller, Response},
    },
    error::ContractResult,
};

const ICA_TRANSFER_TIMEOUT: Duration = Duration::from_secs(60);
const ICA_TRANSFER_ACK_TIP: Coin<Nls> = Coin::new(1);
const ICA_TRANSFER_TIMEOUT_TIP: Coin<Nls> = ICA_TRANSFER_ACK_TIP;

#[derive(Serialize, Deserialize)]
pub struct TransferOut {
    form: NewLeaseForm,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    dex_account: HostAccount,
    deps: (LppLenderRef, OracleRef),
}

impl TransferOut {
    //TODO change to super or crate::contract::state::opening once the other opening states have moved to opening module
    pub(in crate::contract::state) fn new(
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

    //TODO define a State trait with `fn enter(&self, deps: &Deps)` and
    //simplify the TransferOut::on_success return type to `impl State`
    pub(in crate::contract::state) fn enter_state(
        &self,
        sender: Addr,
        now: Timestamp,
    ) -> ContractResult<Batch> {
        let mut ibc_sender = Sender::new(
            &self.form.dex.transfer_channel.local_endpoint,
            sender,
            self.dex_account.clone(),
            now + ICA_TRANSFER_TIMEOUT,
            ICA_TRANSFER_ACK_TIP,
            ICA_TRANSFER_TIMEOUT_TIP,
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
