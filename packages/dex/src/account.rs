use serde::{Deserialize, Serialize};

use oracle::stub::OracleRef;
use platform::{
    batch::Batch as LocalBatch,
    ica::{self, HostAccount},
};
use sdk::cosmwasm_std::{Addr, QuerierWrapper, Timestamp};

use crate::{
    connectable::DexConnectable,
    connection::ConnectionParams,
    error::Result,
    trx::{SwapTrx, TransferInTrx, TransferOutTrx},
};

#[derive(Serialize, Deserialize)]
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
        Ok(Self {
            owner,
            host,
            dex,
        })
    }

    pub(super) fn transfer_to(&self, now: Timestamp) -> TransferOutTrx<'_> {
        TransferOutTrx::new(
            &self.dex.transfer_channel.local_endpoint,
            &self.owner,
            &self.host,
            now,
        )
    }

    pub(super) fn swap<'a>(
        &'a self,
        oracle: &'a OracleRef,
        querier: &'a QuerierWrapper<'a>,
    ) -> SwapTrx<'a> {
        SwapTrx::new(&self.dex.connection_id, &self.host, oracle, querier)
    }

    pub(super) fn transfer_from(&self, now: Timestamp) -> TransferInTrx<'_> {
        TransferInTrx::new(
            &self.dex.connection_id,
            &self.dex.transfer_channel.remote_endpoint,
            &self.host,
            &self.owner,
            now,
        )
    }
}

impl From<Account> for HostAccount {
    fn from(account: Account) -> Self {
        account.host
    }
}

impl DexConnectable for Account {
    fn dex(&self) -> &ConnectionParams {
        &self.dex
    }
}
