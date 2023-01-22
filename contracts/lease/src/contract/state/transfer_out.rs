use cosmwasm_std::{Addr, Deps, Timestamp};
use currency::native::Nls;
use serde::{Deserialize, Serialize};

use finance::{coin::Coin, duration::Duration};
use platform::{bank_ibc::local::Sender, batch::Batch, ica::HostAccount};
use sdk::{
    cosmwasm_std::{DepsMut, Env},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{StateQuery, StateResponse},
    error::ContractResult,
};

use super::{Controller as ContractController, Response};

const ICA_TRANSFER_TIMEOUT: Duration = Duration::from_secs(60);
const ICA_TRANSFER_ACK_TIP: Coin<Nls> = Coin::new(1);
const ICA_TRANSFER_TIMEOUT_TIP: Coin<Nls> = ICA_TRANSFER_ACK_TIP;

//TODO define a State trait with `fn enter(&self, deps: &Deps)` and
//simplify the TransferOut::on_success return type to `impl State`
pub trait TransferOut<'a> {
    fn channel<'b: 'a>(&'b self) -> &'a str;
    fn receiver(&self) -> HostAccount;
    fn send(&self, sender: &mut Sender) -> ContractResult<()>;
    fn on_success(self, platform: &Deps) -> ContractResult<Response>;
    fn into_state(self) -> StateResponse;
}

#[derive(Serialize, Deserialize)]
pub struct Controller<T> {
    transfer: T,
}

impl<T> Controller<T>
where
    T: for<'a> TransferOut<'a>,
{
    pub(super) fn new(transfer: T) -> Self {
        Self { transfer }
    }

    pub(super) fn enter_state(&self, sender: Addr, now: Timestamp) -> ContractResult<Batch> {
        let mut ibc_sender = Sender::new(
            self.transfer.channel(),
            sender,
            self.transfer.receiver(),
            now + ICA_TRANSFER_TIMEOUT,
            ICA_TRANSFER_ACK_TIP,
            ICA_TRANSFER_TIMEOUT_TIP,
        );
        self.transfer.send(&mut ibc_sender)?;

        Ok(ibc_sender.into())
    }
}

impl<T> ContractController for Controller<T>
where
    T: for<'a> TransferOut<'a>,
{
    fn sudo(self, deps: &mut DepsMut, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => self.transfer.on_success(&deps.as_ref()),
            SudoMsg::Timeout { request: _ } => todo!(),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => todo!(),
        }
    }

    fn query(self, _deps: Deps, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        Ok(self.transfer.into_state())
    }
}
