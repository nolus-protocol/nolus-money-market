use serde::{Deserialize, Serialize};

use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{
    batch::{Batch, Emit, Emitter},
    ica::HostAccount,
};
use sdk::{
    cosmwasm_std::{Addr, Deps, DepsMut, Env},
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

    fn enter_state(&self) -> Batch {
        Account::register_request(&self.new_lease.dex)
    }

    fn on_response(
        self,
        counterparty_version: String,
        deps: Deps<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        let contract = &env.contract.address;
        let dex_account = Account::from_register_response(
            &counterparty_version,
            contract.clone(),
            self.new_lease.dex,
        )?;

        let emitter = Self::emit_ok(contract.clone(), dex_account.dex_account().clone());
        let transfer_out = TransferOut::new(
            self.new_lease.form,
            dex_account,
            self.downpayment,
            self.loan,
            self.deps,
        );
        let batch = transfer_out.enter(deps, env)?;
        Ok(Response::from(batch.into_response(emitter), transfer_out))
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        let emitter = Self::emit_timeout(env.contract.address.clone());
        let batch = self.enter(deps, env)?;
        Ok(Response::from(batch.into_response(emitter), self))
    }

    fn emit_ok(contract: Addr, dex_account: HostAccount) -> Emitter {
        Emitter::of_type(Type::OpenIcaAccount)
            .emit("id", contract)
            .emit("dex_account", dex_account)
    }

    fn emit_timeout(contract: Addr) -> Emitter {
        Emitter::of_type(Type::OpenIcaAccount)
            .emit("id", contract)
            .emit("timeout", "")
    }
}

impl Controller for OpenIcaAccount {
    fn enter(&self, _deps: Deps<'_>, _env: Env) -> ContractResult<Batch> {
        Ok(self.enter_state())
    }

    fn sudo(self, deps: &mut DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::OpenAck {
                port_id: _,
                channel_id: _,
                counterparty_channel_id: _,
                counterparty_version,
            } => self.on_response(counterparty_version, deps.as_ref(), env),
            SudoMsg::Timeout { request: _ } => self.on_timeout(deps.as_ref(), env),
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
            in_progress: OngoingTrx::OpenIcaAccount {},
        })
    }
}
