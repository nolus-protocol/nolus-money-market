use serde::Deserialize;

use dex::{
    CoinsNb, SwapExactIn as SwapExactInV3, TransferInFinish as TransferInFinishV3,
    TransferInInit as TransferInInitV3, TransferOut as TransferOutV3,
};
use sdk::cosmwasm_std::Timestamp;

use crate::{
    api::{LpnCoin, PaymentCoin},
    contract::state::{v2::Lease as LeaseV2, v2::Migrate, State},
};

use super::buy_lpn::{BuyLpn as BuyLpnSpec, DexState};

pub(in crate::contract::state) type Swap = BuyLpn;

#[derive(Deserialize)]
pub struct TransferOut {
    lease: LeaseV2,
    payment: PaymentCoin,
}

impl Migrate for TransferOut {
    fn into_last_version(self) -> State {
        let spec = BuyLpnSpec::migrate_to(self.lease.into(), self.payment);
        DexState::from(TransferOutV3::migrate_from(
            spec,
            CoinsNb::default(),
            CoinsNb::default(),
        ))
        .into()
    }
}

#[derive(Deserialize)]
pub struct BuyLpn {
    lease: LeaseV2,
    payment: PaymentCoin,
}

impl Migrate for BuyLpn {
    fn into_last_version(self) -> State {
        let spec = BuyLpnSpec::migrate_to(self.lease.into(), self.payment);
        DexState::from(SwapExactInV3::migrate_from(spec)).into()
    }
}

#[derive(Deserialize)]
pub struct TransferInInit {
    lease: LeaseV2,
    payment: PaymentCoin,
    payment_lpn: LpnCoin,
}

impl Migrate for TransferInInit {
    fn into_last_version(self) -> State {
        let spec = BuyLpnSpec::migrate_to(self.lease.into(), self.payment);
        DexState::from(TransferInInitV3::new(spec, self.payment_lpn)).into()
    }
}

#[derive(Deserialize)]
pub struct TransferInFinish {
    lease: LeaseV2,
    payment: PaymentCoin,
    payment_lpn: LpnCoin,
    timeout: Timestamp,
}

impl Migrate for TransferInFinish {
    fn into_last_version(self) -> State {
        let spec = BuyLpnSpec::migrate_to(self.lease.into(), self.payment);
        DexState::from(TransferInFinishV3::migrate_from(
            spec,
            self.payment_lpn,
            self.timeout,
        ))
        .into()
    }
}
