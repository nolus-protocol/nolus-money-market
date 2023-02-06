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
    bank_ibc::{local::Sender as LocalSender, remote::Sender as RemoteSender},
    batch::Batch as LocalBatch,
    ica::{self, HostAccount},
    trx::Transaction,
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
    dex_account: HostAccount,
    dex: ConnectionParams,
}

impl Account {
    pub(crate) fn dex_account(&self) -> &HostAccount {
        &self.dex_account
    }

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
            dex_account: ica_account,
            dex,
        })
    }

    pub fn transfer_to(&self, now: Timestamp) -> TransferOutTrx {
        TransferOutTrx::new(
            &self.dex.transfer_channel.local_endpoint,
            &self.owner,
            &self.dex_account,
            now,
        )
    }

    pub fn swap<'a>(&'a self, oracle: &'a OracleRef, querier: &'a QuerierWrapper) -> SwapTrx {
        SwapTrx::new(&self.dex.connection_id, &self.dex_account, oracle, querier)
    }

    pub fn transfer_from(&self, now: Timestamp) -> TransferInTrx {
        TransferInTrx::new(
            &self.dex.connection_id,
            &self.dex.transfer_channel.remote_endpoint,
            &self.dex_account,
            &self.owner,
            now,
        )
    }
}

impl From<Account> for HostAccount {
    fn from(account: Account) -> Self {
        account.dex_account
    }
}

pub struct TransferOutTrx<'a> {
    sender: LocalSender<'a>,
}

impl<'a> TransferOutTrx<'a> {
    fn new(channel: &'a str, sender: &Addr, receiver: &HostAccount, now: Timestamp) -> Self {
        let sender = LocalSender::new(
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
        value.sender.into()
    }
}

pub struct SwapTrx<'a> {
    conn: &'a str,
    ica_account: &'a HostAccount,
    trx: Transaction,
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
        let trx = Transaction::default();
        Self {
            conn,
            ica_account,
            trx,
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
        trx::exact_amount_in(&mut self.trx, self.ica_account.clone(), amount, &swap_path)?;
        Ok(())
    }
}

impl From<SwapTrx<'_>> for LocalBatch {
    fn from(value: SwapTrx<'_>) -> Self {
        ica::submit_transaction(
            value.conn,
            value.trx,
            "memo",
            IBC_TIMEOUT,
            ICA_SWAP_ACK_TIP,
            ICA_SWAP_TIMEOUT_TIP,
        )
    }
}

pub struct TransferInTrx<'a> {
    conn: &'a str,
    sender: RemoteSender<'a>,
}

impl<'a> TransferInTrx<'a> {
    fn new(
        conn: &'a str,
        channel: &'a str,
        sender: &HostAccount,
        receiver: &Addr,
        now: Timestamp,
    ) -> Self {
        let sender =
            RemoteSender::new(channel, sender.clone(), receiver.clone(), now + IBC_TIMEOUT);
        TransferInTrx { conn, sender }
    }

    pub fn send<G>(&mut self, amount: &CoinDTO<G>) -> ContractResult<()>
    where
        G: Group,
    {
        self.sender.send(amount).map_err(Into::into)
    }
}

impl From<TransferInTrx<'_>> for LocalBatch {
    fn from(value: TransferInTrx) -> Self {
        ica::submit_transaction(
            value.conn,
            value.sender.into(),
            "memo",
            IBC_TIMEOUT,
            ICA_SWAP_ACK_TIP,
            ICA_SWAP_TIMEOUT_TIP,
        )
    }
}
