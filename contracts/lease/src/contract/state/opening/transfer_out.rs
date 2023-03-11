use cosmwasm_std::{Binary, QuerierWrapper};
use serde::{Deserialize, Serialize};

use finance::zero::Zero;
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{
    batch::{Batch, Emit, Emitter},
    ica::HostAccount,
};
use sdk::cosmwasm_std::{Addr, Deps, Env, Timestamp};

use crate::{
    api::{opening::OngoingTrx, DownpaymentCoin, NewLeaseForm, StateResponse},
    contract::{
        cmd::OpenLoanRespResult,
        dex::Account,
        state::{self, ica_connector::Enterable, BuyAsset, Controller, Response},
        Contract,
    },
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

    pub(super) fn enter(&self, now: Timestamp) -> ContractResult<Batch> {
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
                let emitter = self.emit_ok(env.contract.address);
                let buy_asset = BuyAsset::new(
                    self.form,
                    self.dex_account,
                    self.downpayment,
                    self.loan,
                    self.deps,
                );
                let batch = buy_asset.enter(&deps.querier)?;
                let resp = batch.into_response(emitter);
                Ok(Response::from(resp, buy_asset))
            }
            _ => unreachable!(),
        }
    }
}

impl Enterable for TransferOut {
    fn enter(&self, _deps: Deps<'_>, env: Env) -> ContractResult<Batch> {
        self.enter(env.block.time)
    }
}

impl Controller for TransferOut {
    fn on_response(self, _data: Binary, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        self.on_response(deps, env)
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        state::on_timeout_retry(self, Type::OpeningTransferOut, deps, env)
    }
}

impl Contract for TransferOut {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
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
