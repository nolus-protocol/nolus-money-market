use serde::{Deserialize, Serialize};

use platform::{
    batch::Batch as LocalBatch,
    ica::{self, HostAccount},
};
use sdk::cosmwasm_std::Addr;

use crate::{Connectable, ConnectionParams};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Account {
    /// The contract at Nolus that owns the account
    owner: Addr,
    // converted from the Remote Lease Id, used as destination for outgoing transfers
    // cannot use Remote Account Id because `dex` is protocol-agnostic
    remote: HostAccount,
    dex: ConnectionParams,
}

impl Account {
    pub fn owner(&self) -> &Addr {
        &self.owner
    }

    pub(super) fn remote(&self) -> &HostAccount {
        &self.remote
    }

    pub(super) fn register_request(dex: &ConnectionParams) -> LocalBatch {
        ica::register_account(&dex.connection_id)
    }

    pub fn new(owner: Addr, remote: HostAccount, dex: ConnectionParams) -> Self {
        Self { owner, remote, dex }
    }
}

impl Connectable for Account {
    fn dex(&self) -> &ConnectionParams {
        &self.dex
    }
}
