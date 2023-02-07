use cosmwasm_std::{Addr, Deps, Timestamp};
use serde::{Deserialize, Serialize};

use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{
    batch::{Batch, Emit, Emitter},
    ica::HostAccount,
};
use sdk::{
    cosmwasm_std::{DepsMut, Env},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{opening::OngoingTrx, DownpaymentCoin, NewLeaseContract, StateQuery, StateResponse},
    contract::{
        cmd::OpenLoanRespResult,
        state::{Controller, Response},
    },
    dex::Account,
    error::ContractResult,
    event::Type,
};

use super::transfer_out::TransferOut;

#[derive(Serialize, Deserialize)]
pub struct OpenIcaAccount {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppLenderRef, OracleRef),
}

impl OpenIcaAccount {
    pub(super) fn new(
        new_lease: NewLeaseContract,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppLenderRef, OracleRef),
    ) -> Self {
        Self {
            new_lease,
            downpayment,
            loan,
            deps,
        }
    }

    pub(super) fn enter_state(&self) -> Batch {
        Account::register_request(&self.new_lease.dex)
    }

    fn on_response(
        self,
        counterparty_version: String,
        contract: Addr,
        now: Timestamp,
    ) -> ContractResult<Response> {
        let dex_account = Account::from_register_response(
            &counterparty_version,
            contract.clone(),
            self.new_lease.dex,
        )?;

        let emitter = Self::emit_ok(contract, dex_account.dex_account().clone());
        let next_state = TransferOut::new(
            self.new_lease.form,
            dex_account,
            self.downpayment,
            self.loan,
            self.deps,
        );
        let batch = next_state.enter_state(now)?;
        Ok(Response::from(batch.into_response(emitter), next_state))
    }

    fn emit_ok(contract: Addr, dex_account: HostAccount) -> Emitter {
        Emitter::of_type(Type::OpenIcaAccount)
            .emit("id", contract)
            .emit("dex_account", dex_account)
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
            } => self.on_response(counterparty_version, env.contract.address, env.block.time),
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
            in_progress: OngoingTrx::OpenIcaAccount {},
        })
    }
}
