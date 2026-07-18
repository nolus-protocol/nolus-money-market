use serde::{Deserialize, Serialize};

use platform::remote::Account as RemoteAccount;
use sdk::cosmwasm_std::Addr;

use crate::{Connectable, ConnectionParams};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Account {
    /// The contract at Nolus that owns the account
    owner: Addr,
    // converted from the Remote Lease Id, used as destination for outgoing transfers
    // cannot use Remote Account Id because `dex` is protocol-agnostic
    remote: RemoteAccount,
    dex: ConnectionParams,
}

impl Account {
    pub fn owner(&self) -> &Addr {
        &self.owner
    }

    pub fn remote(&self) -> &RemoteAccount {
        &self.remote
    }

    pub fn new(owner: Addr, remote: RemoteAccount, dex: ConnectionParams) -> Self {
        Self { owner, remote, dex }
    }
}

impl Connectable for Account {
    fn dex(&self) -> &ConnectionParams {
        &self.dex
    }
}

impl From<Account> for RemoteAccount {
    fn from(value: Account) -> Self {
        value.remote
    }
}
