use serde::Deserialize;

use dex::{
    InRecovery, TransferInFinish as TransferInFinishV3, TransferInInit as TransferInInitV3,
    TransferInInitPostRecoverIca, TransferInInitRecoverIca,
};
use sdk::cosmwasm_std::Timestamp;

use crate::contract::{
    state::{v2::Lease as LeaseV2, v2::Migrate, State as StateV3},
    Lease as LeaseV3,
};

use super::{
    transfer_in::{self, DexState, TransferIn as TransferInSpec},
    Active as ActiveV3,
};

#[derive(Deserialize)]
pub struct Active {
    lease: LeaseV2,
}

impl Migrate for Active {
    fn into_last_version(self) -> StateV3 {
        ActiveV3::new(self.lease.into()).into()
    }
}

#[derive(Deserialize)]
pub(crate) struct TransferInInit {
    lease: LeaseV2,
}

impl TransferInInit {
    pub fn into_recovery(self) -> DexState {
        let timealarms = self.lease.lease().time_alarms.clone();
        let swap_v3 = self.into();
        DexState::TransferInInitRecoverIca(TransferInInitRecoverIca::new(InRecovery::new_migrate(
            swap_v3, timealarms,
        )))
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
    fn into_last_version(self) -> StateV3 {
        DexState::from(Into::<TransferInInitV3<TransferInSpec>>::into(self)).into()
    }
}

impl From<TransferInInit> for TransferInInitV3<TransferInSpec> {
    fn from(value: TransferInInit) -> Self {
        transfer_in::start(value.lease.into())
    }
}

#[derive(Deserialize)]
pub struct TransferInFinish {
    lease: LeaseV2,
    timeout: Timestamp,
}

impl Migrate for TransferInFinish {
    fn into_last_version(self) -> StateV3 {
        let lease_v3: LeaseV3 = self.lease.into();
        let amount_in = lease_v3.lease.amount.clone();
        let dex_state = TransferInFinishV3::migrate_from(
            TransferInSpec::new(lease_v3),
            amount_in,
            self.timeout,
        );
        DexState::from(dex_state).into()
    }
}
