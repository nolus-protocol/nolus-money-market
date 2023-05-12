use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize, Serializer};

use dex::{Account as AccountV3, ConnectionParams, IcaConnector as IcaConnectorV3};
use platform::ica::HostAccount;
use sdk::cosmwasm_std::Addr;

use crate::{
    contract::{
        state::{
            BuyAsset as BuyAssetV3, BuyLpn as BuyLpnV3, Closed,
            ClosingTransferIn as ClosingTransferInV3,
        },
        Lease as LeaseV3,
    },
    lease::LeaseDTO,
};

use super::{
    opened, opening::v2::RequestLoan, OpenIcaAccount as OpenIcaAccountV3, State as StateV3,
};

type OpenIcaAccount = IcaConnector<super::opening::v2::OpenIcaAccount>;
type OpeningTransferOut = super::opening::v2::Transfer;
type BuyAsset = super::opening::v2::Swap;
type BuyAssetRecoverIca = IcaConnector<InRecovery<BuyAsset>>;
type BuyAssetPostRecoverIca = PostConnector<InRecovery<BuyAsset>>;

type OpenedActive = opened::v2::Active;

type RepaymentTransferOut = super::opened::repay::v2::TransferOut;
type BuyLpn = super::opened::repay::v2::Swap;
type BuyLpnRecoverIca = IcaConnector<InRecovery<BuyLpn>>;
type BuyLpnPostRecoverIca = PostConnector<InRecovery<BuyLpn>>;
type RepaymentTransferInInit = super::opened::repay::v2::TransferInInit;
type RepaymentTransferInInitRecoverIca = IcaConnector<InRecovery<RepaymentTransferInInit>>;
type RepaymentTransferInInitPostRecoverIca = PostConnector<InRecovery<RepaymentTransferInInit>>;

type RepaymentTransferInFinish = super::opened::repay::v2::TransferInFinish;

type PaidActive = super::paid::v2::Active;

type ClosingTransferInInit = super::paid::v2::TransferInInit;
type ClosingTransferInInitRecoverIca = IcaConnector<InRecovery<ClosingTransferInInit>>;
type ClosingTransferInInitPostRecoverIca = PostConnector<InRecovery<ClosingTransferInInit>>;
type ClosingTransferInFinish = super::paid::v2::TransferInFinish;

#[enum_dispatch]
pub(crate) trait Migrate
where
    Self: Sized,
{
    fn into_last_version(self) -> StateV3;
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
    fn into_last_version(self) -> StateV3 {
        OpenIcaAccountV3::new(IcaConnectorV3::new(self.connectee.into())).into()
    }
}

impl Migrate for BuyAssetRecoverIca {
    fn into_last_version(self) -> StateV3 {
        BuyAssetV3::new(self.connectee.state.into_recovery()).into()
    }
}

impl Migrate for BuyLpnRecoverIca {
    fn into_last_version(self) -> StateV3 {
        BuyLpnV3::new(self.connectee.state.into_recovery()).into()
    }
}

impl Migrate for RepaymentTransferInInitRecoverIca {
    fn into_last_version(self) -> StateV3 {
        BuyLpnV3::new(self.connectee.state.into_recovery()).into()
    }
}

impl Migrate for ClosingTransferInInitRecoverIca {
    fn into_last_version(self) -> StateV3 {
        ClosingTransferInV3::new(self.connectee.state.into_recovery()).into()
    }
}

#[derive(Deserialize)]
pub(crate) struct PostConnector<Connectee> {
    connectee: Connectee,
    // ica_account: Account, not used in migration
}

impl Migrate for BuyAssetPostRecoverIca {
    fn into_last_version(self) -> StateV3 {
        BuyAssetV3::new(self.connectee.state.into_post_recovery()).into()
    }
}

impl Migrate for BuyLpnPostRecoverIca {
    fn into_last_version(self) -> StateV3 {
        BuyLpnV3::new(self.connectee.state.into_post_recovery()).into()
    }
}

impl Migrate for RepaymentTransferInInitPostRecoverIca {
    fn into_last_version(self) -> StateV3 {
        BuyLpnV3::new(self.connectee.state.into_post_recovery()).into()
    }
}

impl Migrate for ClosingTransferInInitPostRecoverIca {
    fn into_last_version(self) -> StateV3 {
        ClosingTransferInV3::new(self.connectee.state.into_post_recovery()).into()
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
    fn into_last_version(self) -> StateV3 {
        self.into()
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
