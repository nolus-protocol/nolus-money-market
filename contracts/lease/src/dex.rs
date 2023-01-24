use crate::{api::dex::ConnectionParams, error::ContractResult};
use cosmwasm_std::{Addr, QuerierWrapper, Timestamp};
use currency::native::Nls;
use finance::{
    coin::{Coin, CoinDTO},
    currency::{Group, Symbol},
    duration::Duration,
};
use oracle::stub::OracleRef;
use platform::{
    bank_ibc::local::Sender,
    batch::Batch as LocalBatch,
    ica::{self, Batch as RemoteBatch, HostAccount},
};
use serde::{Deserialize, Serialize};
use swap::trx;

const IBC_TIMEOUT: Duration = Duration::from_secs(60);

//TODO take them as input from the client
const ICA_TRANSFER_ACK_TIP: Coin<Nls> = Coin::new(1);
const ICA_TRANSFER_TIMEOUT_TIP: Coin<Nls> = ICA_TRANSFER_ACK_TIP;

//TODO take them as input from the client
const ICA_SWAP_ACK_TIP: Coin<Nls> = Coin::new(1);
const ICA_SWAP_TIMEOUT_TIP: Coin<Nls> = ICA_SWAP_ACK_TIP;

#[derive(Serialize, Deserialize)]
pub(crate) struct Account {
    /// The contract at Nolus that owns the account
    owner: Addr,
    ica_account: HostAccount,
    dex: ConnectionParams,
}

impl Account {
    pub fn register_request(dex: &ConnectionParams) -> LocalBatch {
        ica::register_account(&dex.connection_id)
    }

    pub fn from_register_response(
        response: &str,
        owner: Addr,
        dex: ConnectionParams,
    ) -> ContractResult<Self> {
        let ica_account = ica::parse_register_response(response)?;
        Ok(Self {
            owner,
            ica_account,
            dex,
        })
    }

    pub fn transfer_to(&self, now: Timestamp) -> TransferOutTrx {
        TransferOutTrx::new(
            &self.dex.transfer_channel.local_endpoint,
            &self.owner,
            &self.ica_account,
            now,
        )
    }

    pub fn swap<'a>(&'a self, oracle: &'a OracleRef, querier: &'a QuerierWrapper) -> SwapTrx {
        SwapTrx::new(&self.dex.connection_id, &self.ica_account, oracle, querier)
    }
}

impl From<Account> for HostAccount {
    fn from(account: Account) -> Self {
        account.ica_account
    }
}

pub struct TransferOutTrx<'a> {
    sender: Sender<'a>,
}

impl<'a> TransferOutTrx<'a> {
    fn new(channel: &'a str, sender: &Addr, receiver: &HostAccount, now: Timestamp) -> Self {
        let sender = Sender::new(
            channel,
            sender.clone(),
            receiver.clone(),
            now + IBC_TIMEOUT,
            ICA_TRANSFER_ACK_TIP,
            ICA_TRANSFER_TIMEOUT_TIP,
        );
        TransferOutTrx { sender }
    }

    pub fn send<G>(&mut self, amount: &CoinDTO<G>) -> ContractResult<()>
    where
        G: Group,
    {
        self.sender.send(amount).map_err(Into::into)
    }
}

impl From<TransferOutTrx<'_>> for LocalBatch {
    fn from(value: TransferOutTrx) -> Self {
        value.into()
    }
}

pub struct SwapTrx<'a> {
    conn: &'a str,
    ica_account: &'a HostAccount,
    batch: RemoteBatch,
    oracle: &'a OracleRef,
    querier: &'a QuerierWrapper<'a>,
}

impl<'a> SwapTrx<'a> {
    fn new(
        conn: &'a str,
        ica_account: &'a HostAccount,
        oracle: &'a OracleRef,
        querier: &'a QuerierWrapper,
    ) -> Self {
        let batch = RemoteBatch::default();
        Self {
            conn,
            ica_account,
            batch,
            oracle,
            querier,
        }
    }

    pub fn swap_exact_in<G>(
        &mut self,
        amount: &CoinDTO<G>,
        currency_out: Symbol<'_>,
    ) -> ContractResult<()>
    where
        G: Group,
    {
        let swap_path =
            self.oracle
                .swap_path(amount.ticker().into(), currency_out.into(), self.querier)?;
        trx::exact_amount_in(
            &mut self.batch,
            self.ica_account.clone(),
            amount,
            &swap_path,
        )?;
        Ok(())
    }
}

impl From<SwapTrx<'_>> for LocalBatch {
    fn from(value: SwapTrx<'_>) -> Self {
        ica::submit_transaction(
            value.conn,
            value.batch,
            "memo",
            IBC_TIMEOUT,
            ICA_SWAP_ACK_TIP,
            ICA_SWAP_TIMEOUT_TIP,
        )
    }
}
