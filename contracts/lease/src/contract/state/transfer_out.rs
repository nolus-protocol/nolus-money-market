use std::fmt::Display;

use cosmwasm_std::{Addr, Timestamp};
use currency::payment::PaymentGroup;
use serde::{Deserialize, Serialize};

use finance::{coin::CoinDTO, duration::Duration};
use lpp::stub::lender::LppLenderRef;
use market_price_oracle::stub::OracleRef;
use platform::{bank_ibc::LocalChainSender, batch::Batch};
use sdk::{
    cosmwasm_std::{DepsMut, Env},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{api::NewLeaseForm, contract::cmd::OpenLoanRespResult, error::ContractResult};

use super::{buy_asset::BuyAsset, Controller, Response};

#[derive(Serialize, Deserialize)]
pub struct TransferOut {
    form: NewLeaseForm,
    downpayment: CoinDTO,
    loan: OpenLoanRespResult,
    dex_account: Addr,
    deps: (LppLenderRef, OracleRef),
}

const ICA_TRANSFER_TIMEOUT: Duration = Duration::from_secs(60);

impl TransferOut {
    pub(super) fn new(
        form: NewLeaseForm,
        downpayment: CoinDTO,
        loan: OpenLoanRespResult,
        dex_account: Addr,
        deps: (LppLenderRef, OracleRef),
        now: Timestamp,
    ) -> ContractResult<(Batch, Self)> {
        let mut ibc_sender = LocalChainSender::new(
            &form.dex.transfer_channel.local_endpoint,
            &dex_account,
            now + ICA_TRANSFER_TIMEOUT,
        );
        // TODO apply nls_swap_fee on the downpayment only!
        ibc_sender.send::<PaymentGroup>(&downpayment)?;
        ibc_sender.send::<PaymentGroup>(&loan.principal)?;

        Ok((
            ibc_sender.into(),
            Self {
                form,
                downpayment,
                loan,
                dex_account,
                deps,
            },
        ))
    }
}

impl Controller for TransferOut {
    fn sudo(self, _deps: &mut DepsMut, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => {
                let (batch, next_state) = BuyAsset::new(
                    self.form,
                    self.downpayment,
                    self.loan,
                    self.dex_account,
                    self.deps,
                )?;
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

impl Display for TransferOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("transferring assets to the ICA account at the DEX")
    }
}
