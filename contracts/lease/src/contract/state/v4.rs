use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize, Serializer};

use dex::{Account as AccountV3, ConnectionParams};
use platform::{ica::HostAccount, message::Response as MessageResponse};
use sdk::cosmwasm_std::{Addr, Timestamp};

use crate::{
    contract::{
        state::{
            BuyAsset as BuyAssetV3, BuyLpn as BuyLpnV3, Closed,
            ClosingTransferIn as ClosingTransferInV3,
        },
        Lease as LeaseV3,
    },
    error::ContractResult,
    lease::LeaseDTO,
};

use super::{opened, opening::v4::RequestLoan, Response};

type OpenIcaAccount = IcaConnector<super::opening::v4::OpenIcaAccount>;
type OpeningTransferOut = super::opening::v4::Transfer;
type BuyAsset = super::opening::v4::Swap;
type BuyAssetRecoverIca = IcaConnector<InRecovery<BuyAsset>>;
type BuyAssetPostRecoverIca = PostConnector<InRecovery<BuyAsset>>;

type OpenedActive = opened::v4::Active;

type RepaymentTransferOut = super::opened::repay::v4::TransferOut;
type BuyLpn = super::opened::repay::v4::Swap;
type BuyLpnRecoverIca = IcaConnector<InRecovery<BuyLpn>>;
type BuyLpnPostRecoverIca = PostConnector<InRecovery<BuyLpn>>;
type RepaymentTransferInInit = super::opened::repay::v4::TransferInInit;
type RepaymentTransferInInitRecoverIca = IcaConnector<InRecovery<RepaymentTransferInInit>>;
type RepaymentTransferInInitPostRecoverIca = PostConnector<InRecovery<RepaymentTransferInInit>>;

type RepaymentTransferInFinish = super::opened::repay::v4::TransferInFinish;

type PaidActive = super::paid::v4::Active;

type ClosingTransferInInit = super::paid::v4::TransferInInit;
type ClosingTransferInInitRecoverIca = IcaConnector<InRecovery<ClosingTransferInInit>>;
type ClosingTransferInInitPostRecoverIca = PostConnector<InRecovery<ClosingTransferInInit>>;
type ClosingTransferInFinish = super::paid::v4::TransferInFinish;

#[enum_dispatch]
pub(crate) trait Migrate
where
    Self: Sized,
{
    fn into_last_version(self, now: Timestamp) -> ContractResult<Response>;
}

#[enum_dispatch(Migrate)]
#[derive(Deserialize)]
pub(in crate::contract) enum StateV2 {
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

#[derive(Deserialize)]
pub(in crate::contract) struct IcaConnector<Connectee> {
    connectee: Connectee,
}

impl Migrate for OpenIcaAccount {
    // Into::<            opening::buy_asset::DexState,
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        Ok(Response::no_msgs(BuyAssetV3::new(self.connectee.into())))
    }
}

impl Migrate for BuyAssetRecoverIca {
    fn into_last_version(self, now: Timestamp) -> ContractResult<Response> {
        let (time_alarm_msg, dex_pre_recovery) = self.connectee.state.into_recovery(now)?;
        let next_state = BuyAssetV3::new(dex_pre_recovery);
        Ok(Response::from(
            Into::<MessageResponse>::into(time_alarm_msg),
            next_state,
        ))
    }
}

impl Migrate for BuyLpnRecoverIca {
    fn into_last_version(self, now: Timestamp) -> ContractResult<Response> {
        let (time_alarm_msg, dex_pre_recovery) = self.connectee.state.into_recovery(now)?;
        let next_state = BuyLpnV3::new(dex_pre_recovery);
        Ok(Response::from(
            Into::<MessageResponse>::into(time_alarm_msg),
            next_state,
        ))
    }
}

impl Migrate for RepaymentTransferInInitRecoverIca {
    fn into_last_version(self, now: Timestamp) -> ContractResult<Response> {
        let (time_alarm_msg, dex_pre_recovery) = self.connectee.state.into_recovery(now)?;
        let next_state = BuyLpnV3::new(dex_pre_recovery);
        Ok(Response::from(
            Into::<MessageResponse>::into(time_alarm_msg),
            next_state,
        ))
    }
}

impl Migrate for ClosingTransferInInitRecoverIca {
    fn into_last_version(self, now: Timestamp) -> ContractResult<Response> {
        let (time_alarm_msg, dex_pre_recovery) = self.connectee.state.into_recovery(now)?;
        let next_state = ClosingTransferInV3::new(dex_pre_recovery);
        Ok(Response::from(
            Into::<MessageResponse>::into(time_alarm_msg),
            next_state,
        ))
    }
}

#[derive(Deserialize)]
pub(crate) struct PostConnector<Connectee> {
    connectee: Connectee,
    // ica_account: Account, not used in migration
}

impl Migrate for BuyAssetPostRecoverIca {
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        Ok(Response::no_msgs(BuyAssetV3::new(
            self.connectee.state.into_post_recovery(),
        )))
    }
}

impl Migrate for BuyLpnPostRecoverIca {
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        Ok(Response::no_msgs(BuyLpnV3::new(
            self.connectee.state.into_post_recovery(),
        )))
    }
}

impl Migrate for RepaymentTransferInInitPostRecoverIca {
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        Ok(Response::no_msgs(BuyLpnV3::new(
            self.connectee.state.into_post_recovery(),
        )))
    }
}

impl Migrate for ClosingTransferInInitPostRecoverIca {
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        Ok(Response::no_msgs(ClosingTransferInV3::new(
            self.connectee.state.into_post_recovery(),
        )))
    }
}

#[derive(Deserialize)]
pub struct InRecovery<S> {
    state: S,
}

#[derive(Deserialize)]
pub(super) struct Lease {
    lease: LeaseDTO,
    dex: Account,
}

impl Lease {
    pub fn lease(&self) -> &LeaseDTO {
        &self.lease
    }
}

impl From<Lease> for LeaseV3 {
    fn from(value: Lease) -> Self {
        Self {
            lease: value.lease,
            dex: value.dex.into(),
        }
    }
}

#[derive(Deserialize)]
pub(super) struct Account {
    owner: Addr,
    dex_account: HostAccount,
    dex: ConnectionParams,
}

impl From<Account> for AccountV3 {
    fn from(value: Account) -> Self {
        Self::migrate_to(value.owner, value.dex_account, value.dex)
    }
}

impl Migrate for Closed {
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        Ok(Response::no_msgs(self))
    }
}

impl Serialize for StateV2 {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unreachable!(
            "Not intended for real use. Required by cw_storage_plus::Item::load trait bounds."
        );
    }
}
