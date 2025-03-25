use serde::{Deserialize, Serialize};

use platform::{
    batch::Batch as LocalBatch,
    ica::{self, HostAccount},
};
use sdk::cosmwasm_std::Addr;

use crate::{Connectable, ConnectionParams, error::Result};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Account {
    /// The contract at Nolus that owns the account
    owner: Addr,
    host: HostAccount,
    dex: ConnectionParams,
}

impl Account {
    pub fn owner(&self) -> &Addr {
        &self.owner
    }

    pub(super) fn host(&self) -> &HostAccount {
        &self.host
    }

    pub(super) fn register_request(dex: &ConnectionParams) -> LocalBatch {
        ica::register_account(&dex.connection_id)
    }

    pub(super) fn from_register_response(
        response: &str,
        owner: Addr,
        dex: ConnectionParams,
    ) -> Result<Self> {
        let host = ica::parse_register_response(response)?;
        Ok(Self { owner, host, dex })
    }

    #[cfg(feature = "testing")]
    pub fn unchecked(owner: Addr, host: HostAccount, dex: ConnectionParams) -> Self {
        Self { owner, host, dex }
    }
}

impl From<Account> for HostAccount {
    fn from(account: Account) -> Self {
        account.host
    }
}

impl Connectable for Account {
    fn dex(&self) -> &ConnectionParams {
        &self.dex
    }
}
