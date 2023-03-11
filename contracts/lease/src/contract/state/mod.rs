use cosmwasm_std::{Addr, Binary, StdResult, Storage};
use enum_dispatch::enum_dispatch;
use platform::batch::{Emit, Emitter};
use serde::{Deserialize, Serialize};
use std::str;

use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, Reply},
    cw_storage_plus::Item,
};

use crate::{api::ExecuteMsg, error::ContractResult, event::Type};

use self::{
    closed::Closed,
    controller::Controller,
    ica_connector::{Enterable, IcaConnector},
    ica_recover::InRecovery,
    opened::repay::buy_lpn::BuyLpn,
    opening::buy_asset::BuyAsset,
    opening::request_loan::RequestLoan,
};
pub use controller::{execute, instantiate, migrate, query, reply, sudo};

use super::dex::DexConnectable;

mod closed;
mod controller;
mod ica_connector;
mod ica_recover;
mod opened;
mod opening;
mod paid;
mod transfer_in;

type OpenIcaAccount = ica_connector::IcaConnector<opening::open_ica::OpenIcaAccount>;
type OpeningTransferOut = opening::transfer_out::TransferOut;
type BuyAssetRecoverIca = ica_connector::IcaConnector<ica_recover::InRecovery<BuyAsset>>;
type OpenedActive = opened::active::Active;
type RepaymentTransferOut = opened::repay::transfer_out::TransferOut;
type BuyLpnRecoverIca = ica_connector::IcaConnector<ica_recover::InRecovery<BuyLpn>>;
type RepaymentTransferInInit = opened::repay::transfer_in_init::TransferInInit;
type RepaymentTransferInInitRecoverIca =
    ica_connector::IcaConnector<ica_recover::InRecovery<RepaymentTransferInInit>>;
type RepaymentTransferInFinish = opened::repay::transfer_in_finish::TransferInFinish;
type PaidActive = paid::Active;
type ClosingTransferInInit = paid::transfer_in_init::TransferInInit;
type ClosingTransferInInitRecoverIca =
    ica_connector::IcaConnector<ica_recover::InRecovery<ClosingTransferInInit>>;
type ClosingTransferInFinish = paid::transfer_in_finish::TransferInFinish;

#[enum_dispatch(Controller, Contract)]
#[derive(Serialize, Deserialize)]
pub(crate) enum State {
    RequestLoan,
    OpenIcaAccount,
    OpeningTransferOut,
    BuyAsset,
    BuyAssetRecoverIca,
    OpenedActive,
    RepaymentTransferOut,
    BuyLpn,
    BuyLpnRecoverIca,
    RepaymentTransferInInit,
    RepaymentTransferInInitRecoverIca,
    RepaymentTransferInFinish,
    PaidActive,
    ClosingTransferInInit,
    ClosingTransferInInitRecoverIca,
    ClosingTransferInFinish,
    Closed,
}

const STATE_DB_ITEM: Item<'static, State> = Item::new("state");

pub(super) fn load(storage: &dyn Storage) -> StdResult<State> {
    STATE_DB_ITEM.load(storage)
}

pub(super) fn save(storage: &mut dyn Storage, next_state: &State) -> StdResult<()> {
    STATE_DB_ITEM.save(storage, next_state)
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

fn on_timeout_retry<L>(
    current_state: L,
    event_type: Type,
    deps: Deps<'_>,
    env: Env,
) -> ContractResult<Response>
where
    L: Enterable + Into<State>,
{
    let emitter = emit_timeout(
        event_type,
        env.contract.address.clone(),
        TimeoutPolicy::Retry,
    );
    let batch = current_state.enter(deps, env)?;
    Ok(Response::from(batch.into_response(emitter), current_state))
}

fn on_timeout_repair_channel<L>(
    current_state: L,
    event_type: Type,
    _deps: Deps<'_>,
    env: Env,
) -> ContractResult<Response>
where
    L: Enterable + Controller + DexConnectable + Into<State>,
    IcaConnector<InRecovery<L>>: Into<State>,
{
    let emitter = emit_timeout(
        event_type,
        env.contract.address,
        TimeoutPolicy::RepairICS27Channel,
    );
    let recover_ica = IcaConnector::new(InRecovery::new(current_state));
    let batch = recover_ica.enter();
    Ok(Response::from(batch.into_response(emitter), recover_ica))
}

#[derive(Debug)]
enum TimeoutPolicy {
    Retry,
    RepairICS27Channel,
}

fn emit_timeout(event_type: Type, contract: Addr, policy: TimeoutPolicy) -> Emitter {
    Emitter::of_type(event_type)
        .emit("id", contract)
        .emit("timeout", format!("{:?}", policy))
}

fn ignore_msg<S>(state: S) -> ContractResult<Response>
where
    S: Into<State>,
{
    Ok(Response::from(CwResponse::new(), state))
}
