use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, Reply},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{ExecuteMsg, StateQuery, StateResponse},
    error::{ContractError as Err, ContractResult},
};

pub use self::opening::request_loan::RequestLoan;
use self::{
    closed::Closed,
    opened::repay::buy_lpn::BuyLpn,
    opening::{buy_asset::BuyAsset, open_ica_account::OpenIcaAccount},
};

mod closed;
mod opened;
mod opening;
mod paid;
mod transfer_in;
pub(super) mod v_old_1;

type OpeningTransferOut = opening::transfer_out::TransferOut;
type OpenedActive = opened::active::Active;
type RepaymentTransferOut = opened::repay::transfer_out::TransferOut;
type RepaymentTransferInInit = opened::repay::transfer_in_init::TransferInInit;
type RepaymentTransferInFinish = opened::repay::transfer_in_finish::TransferInFinish;
type PaidActive = paid::Active;
type ClosingTransferInInit = paid::transfer_in_init::TransferInInit;
type ClosingTransferInFinish = paid::transfer_in_finish::TransferInFinish;

#[enum_dispatch(Controller)]
#[derive(Serialize, Deserialize)]
pub enum State {
    RequestLoan,
    OpenIcaAccount,
    OpeningTransferOut,
    BuyAsset,
    OpenedActive,
    RepaymentTransferOut,
    BuyLpn,
    RepaymentTransferInInit,
    RepaymentTransferInFinish,
    PaidActive,
    ClosingTransferInInit,
    ClosingTransferInFinish,
    Closed,
}

pub struct Response {
    pub(super) cw_response: CwResponse,
    pub(super) next_state: State,
}

impl Response {
    pub fn from<R, S>(resp: R, next_state: S) -> Self
    where
        R: Into<CwResponse>,
        S: Into<State>,
    {
        Self {
            cw_response: resp.into(),
            next_state: next_state.into(),
        }
    }
}

#[enum_dispatch]
pub trait Controller
where
    Self: Sized,
{
    fn reply(self, _deps: &mut DepsMut<'_>, _env: Env, _msg: Reply) -> ContractResult<Response> {
        err("reply")
    }

    fn execute(
        self,
        _deps: &mut DepsMut<'_>,
        _env: Env,
        _info: MessageInfo,
        _msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        err("execute")
    }

    fn query(self, _deps: Deps<'_>, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse>;

    fn sudo(self, _deps: &mut DepsMut<'_>, _env: Env, _msg: SudoMsg) -> ContractResult<Response> {
        err("sudo")
    }
}

fn err<R>(op: &str) -> ContractResult<R> {
    Err(Err::unsupported_operation(op))
}
