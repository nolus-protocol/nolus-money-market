use cosmwasm_std::Addr;
use enum_dispatch::enum_dispatch;
use finance::liability::Liability;
use oracle::stub::OracleRef;
use serde::{Deserialize, Serialize, Serializer};
use timealarms::stub::TimeAlarmsRef;

use super::{
    closed::Closed,
    opened::{
        active::Active as OpenedActiveV1,
        repay::{
            buy_lpn::BuyLpn as BuyLpnV1,
            transfer_in_init::TransferInInit as RepaymentTransferInInitV1,
            transfer_out::TransferOut as RepaymentTransferOutV1,
        },
    },
    opening::{buy_asset::BuyAsset, open_ica_account::OpenIcaAccount, request_loan::RequestLoan},
    paid::{transfer_in_init::TransferInInit as ClosingTransferInInitV1, Active as PaidActiveV1},
    State as Statev1,
};
use crate::{
    api::{LeaseCoin, LpnCoin, PaymentCoin},
    contract::Lease as Leasev1,
    dex::Account,
    lease::LeaseDTO as LeaseDTOv1,
    loan::LoanDTO,
};

type OpeningTransferOut = super::opening::transfer_out::TransferOut;

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
    fn into_last_version(self, lease_addr: Addr) -> Statev1;
}

#[derive(Serialize, Deserialize)]
struct LeaseDTO {
    customer: Addr,
    amount: LeaseCoin,
    liability: Liability,
    loan: LoanDTO,
    time_alarms: TimeAlarmsRef,
    oracle: OracleRef,
}
impl LeaseDTO {
    fn into_last_version(self, lease_addr: Addr) -> LeaseDTOv1 {
        LeaseDTOv1 {
            addr: lease_addr,
            customer: self.customer,
            amount: self.amount,
            liability: self.liability,
            loan: self.loan,
            time_alarms: self.time_alarms,
            oracle: self.oracle,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Lease {
    lease: LeaseDTO,
    dex: Account,
}
impl Lease {
    fn into_last_version(self, lease_addr: Addr) -> Leasev1 {
        Leasev1 {
            lease: self.lease.into_last_version(lease_addr),
            dex: self.dex,
        }
    }
}

impl Migrate for RequestLoan {
    fn into_last_version(self, _lease_addr: Addr) -> Statev1 {
        Statev1::RequestLoan(self)
    }
}
impl Migrate for OpenIcaAccount {
    fn into_last_version(self, _lease_addr: Addr) -> Statev1 {
        Statev1::OpenIcaAccount(self)
    }
}
impl Migrate for OpeningTransferOut {
    fn into_last_version(self, _lease_addr: Addr) -> Statev1 {
        Statev1::OpeningTransferOut(self)
    }
}
impl Migrate for BuyAsset {
    fn into_last_version(self, _lease_addr: Addr) -> Statev1 {
        Statev1::BuyAsset(self)
    }
}
#[derive(Deserialize)]
pub struct OpenedActive {
    lease: Lease,
}
impl Migrate for OpenedActive {
    fn into_last_version(self, lease_addr: Addr) -> Statev1 {
        Statev1::OpenedActive(OpenedActiveV1::new(
            self.lease.into_last_version(lease_addr),
        ))
    }
}

#[derive(Deserialize)]
pub struct RepaymentTransferOut {
    lease: Lease,
    payment: PaymentCoin,
}
impl Migrate for RepaymentTransferOut {
    fn into_last_version(self, lease_addr: Addr) -> Statev1 {
        Statev1::RepaymentTransferOut(RepaymentTransferOutV1::new(
            self.lease.into_last_version(lease_addr),
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
    fn into_last_version(self, lease_addr: Addr) -> Statev1 {
        Statev1::BuyLpn(BuyLpnV1::new(
            self.lease.into_last_version(lease_addr),
            self.payment,
        ))
    }
}

#[derive(Deserialize)]
pub struct RepaymentTransferIn {
    lease: Lease,
    payment: PaymentCoin,
    payment_lpn: LpnCoin,
}
impl Migrate for RepaymentTransferIn {
    fn into_last_version(self, lease_addr: Addr) -> Statev1 {
        Statev1::RepaymentTransferInInit(RepaymentTransferInInitV1::new(
            self.lease.into_last_version(lease_addr),
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
    fn into_last_version(self, lease_addr: Addr) -> Statev1 {
        Statev1::PaidActive(PaidActiveV1::new(self.lease.into_last_version(lease_addr)))
    }
}

#[derive(Deserialize)]
pub struct ClosingTransferIn {
    lease: Lease,
}
impl Migrate for ClosingTransferIn {
    fn into_last_version(self, lease_addr: Addr) -> Statev1 {
        Statev1::ClosingTransferInInit(ClosingTransferInInitV1::new(
            self.lease.into_last_version(lease_addr),
        ))
    }
}

impl Migrate for Closed {
    fn into_last_version(self, _lease_addr: Addr) -> Statev1 {
        Statev1::Closed(self)
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
