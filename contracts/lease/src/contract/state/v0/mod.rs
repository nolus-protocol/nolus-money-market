use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize, Serializer};
use std::str;

use super::ica_connector::IcaConnector;
use super::{opened, opening, paid, State};

use super::{
    closed::Closed,
    opened::repay::buy_lpn::BuyLpn,
    opening::{buy_asset::BuyAsset, open_ica::OpenIcaAccount, request_loan::RequestLoan},
};

type OpeningTransferOut = opening::transfer_out::TransferOut;
type OpenedActive = opened::active::Active;
type RepaymentTransferOut = opened::repay::transfer_out::TransferOut;
type RepaymentTransferInInit = opened::repay::transfer_in_init::TransferInInit;
type RepaymentTransferInFinish = opened::repay::transfer_in_finish::TransferInFinish;
type PaidActive = paid::Active;
type ClosingTransferInInit = paid::transfer_in_init::TransferInInit;
type ClosingTransferInFinish = paid::transfer_in_finish::TransferInFinish;

#[enum_dispatch(Migrate)]
#[derive(Deserialize)]
pub enum StateV0 {
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

#[enum_dispatch]
pub(crate) trait Migrate
where
    Self: Sized,
{
    fn into_last_version(self) -> State;
}

impl Migrate for RequestLoan {
    fn into_last_version(self) -> State {
        self.into()
    }
}

impl Migrate for OpenIcaAccount {
    fn into_last_version(self) -> State {
        IcaConnector::new(self).into()
    }
}

impl Migrate for OpeningTransferOut {
    fn into_last_version(self) -> State {
        self.into()
    }
}

impl Migrate for BuyAsset {
    fn into_last_version(self) -> State {
        self.into()
    }
}

impl Migrate for OpenedActive {
    fn into_last_version(self) -> State {
        self.into()
    }
}

impl Migrate for RepaymentTransferOut {
    fn into_last_version(self) -> State {
        self.into()
    }
}

impl Migrate for BuyLpn {
    fn into_last_version(self) -> State {
        self.into()
    }
}

impl Migrate for RepaymentTransferInInit {
    fn into_last_version(self) -> State {
        self.into()
    }
}

impl Migrate for RepaymentTransferInFinish {
    fn into_last_version(self) -> State {
        self.into()
    }
}

impl Migrate for PaidActive {
    fn into_last_version(self) -> State {
        self.into()
    }
}

impl Migrate for ClosingTransferInInit {
    fn into_last_version(self) -> State {
        self.into()
    }
}

impl Migrate for ClosingTransferInFinish {
    fn into_last_version(self) -> State {
        self.into()
    }
}
impl Migrate for Closed {
    fn into_last_version(self) -> State {
        self.into()
    }
}

impl Serialize for StateV0 {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unreachable!(
            "Not intended for real use. Required by cw_storage_plus::Item::load trait bounds."
        );
    }
}
