use serde::{Deserialize, Serialize};

use finance::zero::Zero;
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{
    batch::{Batch, Emit, Emitter},
    ica::HostAccount,
};
use sdk::{
    cosmwasm_std::{Addr, Deps, DepsMut, Env, Timestamp},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{opening::OngoingTrx, DownpaymentCoin, NewLeaseForm, StateQuery, StateResponse},
    contract::{
        cmd::OpenLoanRespResult,
        state::{self, BuyAsset, Controller, Response},
    },
    dex::Account,
    error::ContractResult,
    event::Type,
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

    fn enter_state(&self, now: Timestamp) -> ContractResult<Batch> {
        debug_assert_eq!(self.nb_completed, TransfersNb::ZERO);
        let mut sender = self.dex_account.transfer_to(now);
        sender.send(&self.downpayment)?;
        sender.send(&self.loan.principal)?;
        Ok(sender.into())
    }

    fn emit_ok(&self, contract: Addr) -> Emitter {
        Emitter::of_type(Type::OpeningTransferOut)
            .emit("id", contract)
            .emit_coin_dto("downpayment", self.downpayment.clone())
    }

    fn on_response(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        match self.nb_completed {
            0 => {
                let transfer_out = Self {
                    nb_completed: self.nb_completed + 1,
                    ..self
                };
                Ok(Response::from(Batch::default(), transfer_out))
            }
            1 => {
                let emitter = self.emit_ok(env.contract.address.clone());
                let buy_asset = BuyAsset::new(
                    self.form,
                    self.dex_account,
                    self.downpayment,
                    self.loan,
                    self.deps,
                );
                let batch = buy_asset.enter(deps, env)?;
                let resp = batch.into_response(emitter);
                Ok(Response::from(resp, buy_asset))
            }
            _ => unreachable!(),
        }
    }
}

impl Controller for TransferOut {
    fn enter(&self, _deps: Deps<'_>, env: Env) -> ContractResult<Batch> {
        self.enter_state(env.block.time)
    }

    fn sudo(self, deps: &mut DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response { request: _, data } => {
                deps.api.debug(&format!(
                    "[Lease][Opening][TransferOut] receive ack '{}'",
                    data.to_base64()
                ));
                self.on_response(deps.as_ref(), env)
            }
            SudoMsg::Timeout { request: _ } => self.on_timeout(deps.as_ref(), env),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => unreachable!(),
        }
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        state::on_timeout_retry(self.into(), Type::OpeningTransferOut, deps, env)
    }

    fn query(self, _deps: Deps<'_>, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
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
