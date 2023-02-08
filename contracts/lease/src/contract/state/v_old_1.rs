use cosmwasm_std::Addr;
use enum_dispatch::enum_dispatch;
use platform::ica::HostAccount;
use serde::{Deserialize, Serialize, Serializer};

use super::{
    closed::Closed,
    opened::{
        active::Active as OpenedActiveNew,
        repay::{
            buy_lpn::BuyLpn as BuyLpnNew,
            transfer_in_init::TransferInInit as RepaymentTransferInInitNew,
            transfer_out::TransferOut as RepaymentTransferOutNew,
        },
    },
    opening::{buy_asset::BuyAsset, open_ica_account::OpenIcaAccount, request_loan::RequestLoan},
    paid::{transfer_in_init::TransferInInit as ClosingTransferInInitNew, Active as PaidActiveNew},
    State as StateNew,
};
use crate::{
    api::{dex::ConnectionParams, LpnCoin, PaymentCoin},
    contract::Lease as LeaseNew,
    dex::Account as AccountNew,
    lease::LeaseDTO,
};

type OpeningTransferOut = super::opening::transfer_out::TransferOut;

#[enum_dispatch(Migrate)]
#[derive(Deserialize)]
pub enum StateV1 {
    RequestLoan,
    OpenIcaAccount,
    OpeningTransferOut,
    BuyAsset,
    OpenedActive,
    RepaymentTransferOut,
    BuyLpn,
    RepaymentTransferIn,
    PaidActive,
    ClosingTransferIn,
    Closed,
}

#[enum_dispatch]
pub trait Migrate
where
    Self: Sized,
{
    fn into_last_version(self) -> StateNew;
}

#[derive(Serialize, Deserialize)]
struct Account {
    /// The contract at Nolus that owns the account
    owner: Addr,
    ica_account: HostAccount,
    dex: ConnectionParams,
}
impl Account {
    fn into_last_version(self) -> AccountNew {
        //TODO remove the fn once that migration completes
        AccountNew::new_migrated(self.owner, self.ica_account, self.dex)
    }
}

#[derive(Serialize, Deserialize)]
struct Lease {
    lease: LeaseDTO,
    dex: Account,
}
impl Lease {
    fn into_last_version(self) -> LeaseNew {
        LeaseNew {
            lease: self.lease,
            dex: self.dex.into_last_version(),
        }
    }
}

impl Migrate for RequestLoan {
    fn into_last_version(self) -> StateNew {
        StateNew::RequestLoan(self)
    }
}
impl Migrate for OpenIcaAccount {
    fn into_last_version(self) -> StateNew {
        StateNew::OpenIcaAccount(self)
    }
}
impl Migrate for OpeningTransferOut {
    fn into_last_version(self) -> StateNew {
        StateNew::OpeningTransferOut(self)
    }
}
impl Migrate for BuyAsset {
    fn into_last_version(self) -> StateNew {
        StateNew::BuyAsset(self)
    }
}
#[derive(Deserialize)]
pub struct OpenedActive {
    lease: Lease,
}
impl Migrate for OpenedActive {
    fn into_last_version(self) -> StateNew {
        StateNew::OpenedActive(OpenedActiveNew::new(self.lease.into_last_version()))
    }
}

#[derive(Deserialize)]
pub struct RepaymentTransferOut {
    lease: Lease,
    payment: PaymentCoin,
}
impl Migrate for RepaymentTransferOut {
    fn into_last_version(self) -> StateNew {
        StateNew::RepaymentTransferOut(RepaymentTransferOutNew::new(
            self.lease.into_last_version(),
            self.payment,
        ))
    }
}

#[derive(Deserialize)]
pub struct BuyLpn {
    lease: Lease,
    payment: PaymentCoin,
}
impl Migrate for BuyLpn {
    fn into_last_version(self) -> StateNew {
        StateNew::BuyLpn(BuyLpnNew::new(self.lease.into_last_version(), self.payment))
    }
}

#[derive(Deserialize)]
pub struct RepaymentTransferIn {
    lease: Lease,
    payment: PaymentCoin,
    payment_lpn: LpnCoin,
}
impl Migrate for RepaymentTransferIn {
    fn into_last_version(self) -> StateNew {
        StateNew::RepaymentTransferInInit(RepaymentTransferInInitNew::new(
            self.lease.into_last_version(),
            self.payment,
            self.payment_lpn,
        ))
    }
}

#[derive(Deserialize)]
pub struct PaidActive {
    lease: Lease,
}
impl Migrate for PaidActive {
    fn into_last_version(self) -> StateNew {
        StateNew::PaidActive(PaidActiveNew::new(self.lease.into_last_version()))
    }
}

#[derive(Deserialize)]
pub struct ClosingTransferIn {
    lease: Lease,
}
impl Migrate for ClosingTransferIn {
    fn into_last_version(self) -> StateNew {
        StateNew::ClosingTransferInInit(ClosingTransferInInitNew::new(
            self.lease.into_last_version(),
        ))
    }
}

impl Migrate for Closed {
    fn into_last_version(self) -> StateNew {
        StateNew::Closed(self)
    }
}

impl Serialize for StateV1 {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unreachable!(
            "Not intended for real use. Required by cw_storage_plus::Item::load trait bounds."
        );
    }
}
