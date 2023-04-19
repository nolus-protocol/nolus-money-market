use cosmwasm_std::Timestamp;
use dex::TransferInFinish as TransferInFinishV3;
use serde::Deserialize;

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
pub struct TransferInInit {
    lease: LeaseV2,
}

impl Migrate for TransferInInit {
    fn into_last_version(self) -> StateV3 {
        let start = transfer_in::start(self.lease.into());
        DexState::from(start).into()
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
