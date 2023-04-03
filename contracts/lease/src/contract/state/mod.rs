use std::str;

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

pub use controller::{execute, instantiate, migrate, query, reply, sudo};
use platform::batch::{Emit, Emitter};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, StdResult, Storage,
    },
    cw_storage_plus::Item,
};

use crate::{api::ExecuteMsg, error::ContractResult};

use super::dex::DexConnectable;

use self::{
    closed::Closed,
    controller::Controller,
    ica_connector::{Enterable, IcaConnectee, IcaConnector},
    ica_recover::InRecovery,
    opened::repay::buy_lpn::BuyLpn,
    opening::request_loan::RequestLoan,
};

mod closed;
mod controller;
mod ica_connector;
mod ica_post_connector;
mod ica_recover;
mod opened;
mod opening;
mod paid;
mod transfer_in;

type OpenIcaAccount = IcaConnector<
    { opening::open_ica::OpenIcaAccount::PRECONNECTABLE },
    opening::open_ica::OpenIcaAccount,
>;
type OpeningTransferOut = opening::buy_asset::Transfer;

type BuyAsset = opening::buy_asset::Swap;
type BuyAssetRecoverIca =
    IcaConnector<{ InRecovery::<BuyAsset>::PRECONNECTABLE }, InRecovery<BuyAsset>>;
type BuyAssetPostRecoverIca = ica_post_connector::PostConnector<InRecovery<BuyAsset>>;

type OpenedActive = opened::active::Active;

type RepaymentTransferOut = opened::repay::transfer_out::TransferOut;

type BuyLpnRecoverIca = IcaConnector<{ InRecovery::<BuyLpn>::PRECONNECTABLE }, InRecovery<BuyLpn>>;
type BuyLpnPostRecoverIca = ica_post_connector::PostConnector<InRecovery<BuyLpn>>;

type RepaymentTransferInInit = opened::repay::transfer_in_init::TransferInInit;
type RepaymentTransferInInitRecoverIca = IcaConnector<
    { InRecovery::<RepaymentTransferInInit>::PRECONNECTABLE },
    InRecovery<RepaymentTransferInInit>,
>;
type RepaymentTransferInInitPostRecoverIca =
    ica_post_connector::PostConnector<InRecovery<RepaymentTransferInInit>>;

type RepaymentTransferInFinish = opened::repay::transfer_in_finish::TransferInFinish;

type PaidActive = paid::Active;

type ClosingTransferInInit = paid::transfer_in_init::TransferInInit;
type ClosingTransferInInitRecoverIca = IcaConnector<
    { InRecovery::<RepaymentTransferInInit>::PRECONNECTABLE },
    InRecovery<ClosingTransferInInit>,
>;
type ClosingTransferInInitPostRecoverIca =
    ica_post_connector::PostConnector<InRecovery<ClosingTransferInInit>>;

type ClosingTransferInFinish = paid::transfer_in_finish::TransferInFinish;

#[enum_dispatch(Controller, Contract)]
#[derive(Serialize, Deserialize)]
pub(crate) enum State {
    RequestLoan,
    OpenIcaAccount,
    OpeningTransferOut,
    BuyAsset,
    BuyAssetRecoverIca,
    BuyAssetPostRecoverIca,
    OpenedActive,
    RepaymentTransferOut,
    BuyLpn,
    BuyLpnRecoverIca,
    BuyLpnPostRecoverIca,
    RepaymentTransferInInit,
    RepaymentTransferInInitRecoverIca,
    RepaymentTransferInInitPostRecoverIca,
    RepaymentTransferInFinish,
    PaidActive,
    ClosingTransferInInit,
    ClosingTransferInInitRecoverIca,
    ClosingTransferInInitPostRecoverIca,
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

    pub fn attach_alarm_response(mut self, env: &Env) -> ContractResult<Self> {
        self.cw_response = self.cw_response.set_data(to_binary(&env.contract.address)?);
        Ok(self)
    }
}

fn on_timeout_retry<S, L>(
    current_state: S,
    state_label: L,
    deps: Deps<'_>,
    env: Env,
) -> ContractResult<Response>
where
    S: Enterable + Into<State>,
    L: Into<String>,
{
    let emitter = emit_timeout(
        state_label,
        env.contract.address.clone(),
        TimeoutPolicy::Retry,
    );
    let batch = current_state.enter(deps, &env)?;
    Ok(Response::from(batch.into_response(emitter), current_state))
}

fn on_timeout_repair_channel<S, L>(
    current_state: S,
    state_label: L,
    env: Env,
) -> ContractResult<Response>
where
    S: Enterable + Controller + DexConnectable + Into<State>,
    IcaConnector<false, InRecovery<S>>: Into<State>,
    L: Into<String>,
{
    let emitter = emit_timeout(
        state_label,
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

fn emit_timeout<L>(state_label: L, contract: Addr, policy: TimeoutPolicy) -> Emitter
where
    L: Into<String>,
{
    Emitter::of_type(state_label)
        .emit("id", contract)
        .emit("timeout", format!("{:?}", policy))
}

fn ignore_msg<S>(state: S) -> ContractResult<Response>
where
    S: Into<State>,
{
    Ok(Response::from(CwResponse::new(), state))
}
