use platform::batch::Batch;
use serde::Deserialize;

use dex::{
    CoinsNb, IcaConnector, InRecovery, SwapExactIn as SwapExactInV3, SwapExactInPostRecoverIca,
    SwapExactInPreRecoverIca, TransferInFinish as TransferInFinishV3,
    TransferInInit as TransferInInitV3, TransferInInitPostRecoverIca, TransferInInitPreRecoverIca,
    TransferOut as TransferOutV3,
};
use sdk::cosmwasm_std::Timestamp;

use crate::{
    api::{LpnCoin, PaymentCoin},
    contract::state::{v2::Lease as LeaseV2, v2::Migrate, Response},
    error::ContractResult,
};

use super::buy_lpn::{BuyLpn as BuyLpnSpec, DexState};

pub(in crate::contract::state) type Swap = BuyLpn;

#[derive(Deserialize)]
pub struct TransferOut {
    lease: LeaseV2,
    payment: PaymentCoin,
}

impl Migrate for TransferOut {
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        let spec = BuyLpnSpec::migrate_to(self.lease.into(), self.payment);
        Ok(Response::no_msgs(DexState::from(
            TransferOutV3::migrate_from(spec, CoinsNb::default(), CoinsNb::default()),
        )))
    }
}

#[derive(Deserialize)]
pub(crate) struct BuyLpn {
    lease: LeaseV2,
    payment: PaymentCoin,
}

impl BuyLpn {
    pub fn into_recovery(self, now: Timestamp) -> ContractResult<(Batch, DexState)> {
        let timealarms = self.lease.lease().time_alarms.clone();
        let pre_recovery = SwapExactInPreRecoverIca::new_migrate(
            IcaConnector::new(InRecovery::new_migrate(self.into(), timealarms.clone())),
            timealarms,
        );
        pre_recovery
            .enter_migrate(now)
            .map(|msgs| (msgs, pre_recovery.into()))
            .map_err(Into::into)
    }

    pub fn into_post_recovery(self) -> DexState {
        let timealarms = self.lease.lease().time_alarms.clone();
        DexState::SwapExactInPostRecoverIca(SwapExactInPostRecoverIca::new_migrate(
            self.into(),
            timealarms,
        ))
    }
}

impl Migrate for BuyLpn {
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        Ok(Response::no_msgs(DexState::from(Into::<
            SwapExactInV3<BuyLpnSpec, DexState>,
        >::into(self))))
    }
}

impl From<BuyLpn> for SwapExactInV3<BuyLpnSpec, DexState> {
    fn from(value: BuyLpn) -> Self {
        SwapExactInV3::migrate_from(BuyLpnSpec::migrate_to(value.lease.into(), value.payment))
    }
}

#[derive(Deserialize)]
pub(crate) struct TransferInInit {
    lease: LeaseV2,
    payment: PaymentCoin,
    payment_lpn: LpnCoin,
}

impl TransferInInit {
    pub fn into_recovery(self, now: Timestamp) -> ContractResult<(Batch, DexState)> {
        let timealarms = self.lease.lease().time_alarms.clone();
        let pre_recovery = TransferInInitPreRecoverIca::new_migrate(
            IcaConnector::new(InRecovery::new_migrate(self.into(), timealarms.clone())),
            timealarms,
        );
        pre_recovery
            .enter_migrate(now)
            .map(|msgs| (msgs, pre_recovery.into()))
            .map_err(Into::into)
    }

    pub fn into_post_recovery(self) -> DexState {
        let timealarms = self.lease.lease().time_alarms.clone();
        DexState::TransferInInitPostRecoverIca(TransferInInitPostRecoverIca::new_migrate(
            self.into(),
            timealarms,
        ))
    }
}

impl Migrate for TransferInInit {
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        Ok(Response::no_msgs(DexState::from(Into::<
            TransferInInitV3<BuyLpnSpec>,
        >::into(self))))
    }
}

impl From<TransferInInit> for TransferInInitV3<BuyLpnSpec> {
    fn from(value: TransferInInit) -> Self {
        let spec = BuyLpnSpec::migrate_to(value.lease.into(), value.payment);
        TransferInInitV3::migrate_from(spec, value.payment_lpn)
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
    fn into_last_version(self, _now: Timestamp) -> ContractResult<Response> {
        let spec = BuyLpnSpec::migrate_to(self.lease.into(), self.payment);
        Ok(Response::no_msgs(DexState::from(
            TransferInFinishV3::migrate_from(spec, self.payment_lpn, self.timeout),
        )))
    }
}
