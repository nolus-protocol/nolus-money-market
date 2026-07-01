use serde::{Deserialize, Serialize};

use platform::ica::HostAccount;
use sdk::cosmwasm_std::Addr;

use crate::{Connectable, ConnectionParams};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Account {
    /// The contract at Nolus that owns the account
    owner: Addr,
    /// The Interchain Account host on the DEX chain.
    ///
    /// Always `None` since the legacy ICA path was retired — remote-lease
    /// leases fund and drain over the paired ICS-20 transfer channel addressed
    /// to the Solana-side `LeaseAuthority` and never open an ICA. The field is
    /// retained because `Account` is persisted on-chain state and
    /// `deny_unknown_fields` deserialization must still accept stored records
    /// that carry it.
    host: Option<HostAccount>,
    dex: ConnectionParams,
}

impl Account {
    pub fn owner(&self) -> &Addr {
        &self.owner
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
