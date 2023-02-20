use cosmwasm_std::{Addr, Api};
use enum_dispatch::enum_dispatch;
use platform::batch::{Batch, Emit, Emitter};
use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, Reply},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{ExecuteMsg, StateQuery, StateResponse},
    error::{ContractError as Err, ContractResult},
    event::Type,
};

pub use self::opening::request_loan::RequestLoan;
use self::{closed::Closed, opened::repay::buy_lpn::BuyLpn, opening::buy_asset::BuyAsset};

mod closed;
mod opened;
mod opening;
mod paid;
// mod recover_ica;
mod ica_connector;
mod transfer_in;

type OpenIcaAccount = ica_connector::IcaConnector<opening::open_ica::OpenIcaAccount>;
// type RecoverIcaAccount = register_ica::RegisterIca<opening::open_ica::OpenIca>;
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
pub(crate) enum State {
    RequestLoan,
    OpenIcaAccount,
    // RecoverIcaAccount,
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

pub(crate) struct Response {
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
pub(crate) trait Controller
where
    Self: Sized,
{
    fn enter(&self, deps: Deps<'_>, _env: Env) -> ContractResult<Batch> {
        err("enter", deps.api)
    }

    fn reply(self, deps: &mut DepsMut<'_>, _env: Env, _msg: Reply) -> ContractResult<Response> {
        err("reply", deps.api)
    }

    fn execute(
        self,
        deps: &mut DepsMut<'_>,
        _env: Env,
        _info: MessageInfo,
        _msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        err("execute", deps.api)
    }

    fn sudo(self, deps: &mut DepsMut<'_>, _env: Env, _msg: SudoMsg) -> ContractResult<Response> {
        err("sudo", deps.api)
    }

    fn on_timeout(self, deps: Deps<'_>, _env: Env) -> ContractResult<Response> {
        err("timeout", deps.api)
    }

    fn query(self, _deps: Deps<'_>, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse>;
}

fn err<R>(op: &str, api: &dyn Api) -> ContractResult<R> {
    let err = Err::unsupported_operation(op);
    api.debug(&format!("{:?}", op));

    Err(err)
}

fn on_timeout_retry<L>(
    current_state: L,
    event_type: Type,
    deps: Deps<'_>,
    env: Env,
) -> ContractResult<Response>
where
    L: Controller,
    L: Into<State>,
{
    let emitter = emit_timeout(event_type, env.contract.address.clone());
    let batch = current_state.enter(deps, env)?;
    Ok(Response::from(batch.into_response(emitter), current_state))
}

// fn on_timeout_repair_channel(
//     current_state: State,
//     event_type: Type,
//     deps: Deps<'_>,
//     env: Env,
// ) -> ContractResult<Response> {
//     let emitter = emit_timeout(event_type, env.contract.address.clone());
//     // register_ica::RegisterIca::new(lease)
//     let batch = current_state.enter(deps, env)?;
//     Ok(Response::from(batch.into_response(emitter), current_state))
// }

fn emit_timeout(event_type: Type, contract: Addr) -> Emitter {
    Emitter::of_type(event_type)
        .emit("id", contract)
        .emit("timeout", "")
}
