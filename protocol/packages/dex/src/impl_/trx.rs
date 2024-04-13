use currency::{Group, NlsPlatform, SymbolSlice};
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
};
use oracle::stub::SwapPath;
use oracle_platform::OracleRef;
use platform::{
    bank_ibc::{local::Sender as LocalSender, remote::Sender as RemoteSender},
    batch::Batch as LocalBatch,
    ica::{self, HostAccount},
    trx::Transaction,
};
use sdk::cosmwasm_std::{Addr, QuerierWrapper, Timestamp};

use crate::{error::Result, swap::ExactAmountIn};

pub(super) const IBC_TIMEOUT: Duration = Duration::from_hours(3);

//TODO take them as input from the client
const ICA_TRANSFER_ACK_TIP: Coin<NlsPlatform> = Coin::new(1);
const ICA_TRANSFER_TIMEOUT_TIP: Coin<NlsPlatform> = ICA_TRANSFER_ACK_TIP;

//TODO take them as input from the client
const ICA_SWAP_ACK_TIP: Coin<NlsPlatform> = Coin::new(1);
const ICA_SWAP_TIMEOUT_TIP: Coin<NlsPlatform> = ICA_SWAP_ACK_TIP;

pub(super) struct TransferOutTrx<'a> {
    sender: LocalSender<'a>,
}

impl<'a> TransferOutTrx<'a> {
    pub(super) fn new(
        channel: &'a str,
        sender: &Addr,
        receiver: &HostAccount,
        now: Timestamp,
        memo: String,
    ) -> Self {
        let sender = LocalSender::new(
            channel,
            sender.clone(),
            receiver.clone(),
            now + IBC_TIMEOUT,
            ICA_TRANSFER_ACK_TIP,
            ICA_TRANSFER_TIMEOUT_TIP,
            memo,
        );

        TransferOutTrx { sender }
    }

    pub fn send<G>(&mut self, amount: &CoinDTO<G>) -> Result<()>
    where
        G: Group,
    {
        self.sender.send(amount).map_err(Into::into)
    }
}

impl<'r> From<TransferOutTrx<'r>> for LocalBatch {
    fn from(value: TransferOutTrx<'r>) -> Self {
        value.sender.into()
    }
}

pub(super) struct SwapTrx<'a> {
    conn: &'a str,
    ica_account: &'a HostAccount,
    trx: Transaction,
    oracle: &'a OracleRef,
    querier: QuerierWrapper<'a>,
}

impl<'a> SwapTrx<'a> {
    pub(super) fn new(
        conn: &'a str,
        ica_account: &'a HostAccount,
        oracle: &'a OracleRef,
        querier: QuerierWrapper<'a>,
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

    pub fn swap_exact_in<GIn, GSwap, SwapClient>(
        &mut self,
        amount: &CoinDTO<GIn>,
        currency_out: &SymbolSlice,
    ) -> Result<()>
    where
        GIn: Group,
        GSwap: Group,
        SwapClient: ExactAmountIn,
    {
        self.oracle
            .swap_path(amount.ticker().into(), currency_out.into(), self.querier)
            .map_err(Into::into)
            .and_then(|swap_path| {
                SwapClient::build_request::<GIn, GSwap>(
                    &mut self.trx,
                    self.ica_account.clone(),
                    amount,
                    &swap_path,
                )
                .map_err(Into::into)
            })
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

pub(super) struct TransferInTrx<'a> {
    conn: &'a str,
    sender: RemoteSender<'a>,
}

impl<'a> TransferInTrx<'a> {
    pub(super) fn new(
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

    pub fn send<G>(&mut self, amount: &CoinDTO<G>) -> Result<()>
    where
        G: Group,
    {
        self.sender.send(amount).map_err(Into::into)
    }
}

impl<'r> From<TransferInTrx<'r>> for LocalBatch {
    fn from(value: TransferInTrx<'r>) -> Self {
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
