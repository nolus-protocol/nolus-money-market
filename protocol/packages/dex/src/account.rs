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
    /// The Interchain Account host on the DEX chain.
    ///
    /// `None` for remote-lease leases — they fund and drain over the paired
    /// ICS-20 transfer channel addressed to the Solana-side `LeaseAuthority`
    /// and never open an ICA. Set only on the legacy ICA path
    /// (`from_register_response`), whose machinery #649 removes.
    host: Option<HostAccount>,
    dex: ConnectionParams,
}

impl Account {
    pub fn owner(&self) -> &Addr {
        &self.owner
    }

    pub(super) fn host(&self) -> Option<&HostAccount> {
        self.host.as_ref()
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
        Ok(Self {
            owner,
            host: Some(host),
            dex,
        })
    }

    /// Build an account for a remote-lease lease, which has no Interchain
    /// Account: funding and draining ride the paired ICS-20 transfer channel
    /// addressed to the lease's Solana-side `LeaseAuthority`.
    pub fn funding(owner: Addr, dex: ConnectionParams) -> Self {
        Self {
            owner,
            host: None,
            dex,
        }
    }

    #[cfg(feature = "testing")]
    pub fn unchecked(owner: Addr, host: HostAccount, dex: ConnectionParams) -> Self {
        Self {
            owner,
            host: Some(host),
            dex,
        }
    }
}

impl Connectable for Account {
    fn dex(&self) -> &ConnectionParams {
        &self.dex
    }
}
