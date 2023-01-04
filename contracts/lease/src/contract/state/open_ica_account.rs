use std::fmt::Display;

use cosmwasm_std::{DepsMut, Env};
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{batch::Batch, ica};
use sdk::neutron_sdk::sudo::msg::SudoMsg;
use serde::{Deserialize, Serialize};

use crate::{
    api::{DownpaymentCoin, NewLeaseForm},
    contract::{cmd::OpenLoanRespResult, state::transfer_out::TransferOut},
    error::ContractResult,
};

use super::{Controller, Response};

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
                let dex_account = ica::parse_register_response(&counterparty_version)?;

                let next_state = TransferOut::new(
                    self.form,
                    self.downpayment,
                    self.loan,
                    dex_account,
                    self.deps,
                );
                let batch = next_state.enter_state(env.block.time)?;
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
}

impl Display for OpenIcaAccount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("opening an ICA account")
    }
}
