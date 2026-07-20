use currency::Group;
use cw_time::IntoTimestamp;
use finance::{coin::CoinDTO, instant::Instant};
use platform::{
    bank_ibc::remote::Sender as RemoteSender,
    batch::Batch as LocalBatch,
    remote::{self, Account as RemoteAccount},
    trx::Transaction,
};

use crate::{Account, Connectable, IBC_TIMEOUT, error::Result, transport::Transport};

pub(super) struct SwapTrx<'ica> {
    conn: &'ica str,
    ica_account: &'ica RemoteAccount,
    trx: Transaction,
}

impl<'ica> SwapTrx<'ica> {
    pub(super) fn new(ica: &'ica Account) -> Self {
        Self {
            conn: &ica.dex().connection_id,
            ica_account: ica.remote(),
            trx: Transaction::default(),
        }
    }

    pub fn swap_exact_in<SwapGIn, SwapGOut, TransportImpl>(
        &mut self,
        amount_in: &CoinDTO<SwapGIn>,
        min_amount_out: &CoinDTO<SwapGOut>,
    ) -> Result<()>
    where
        SwapGIn: Group,
        SwapGOut: Group,
        TransportImpl: Transport,
    {
        TransportImpl::build_request(
            &mut self.trx,
            self.ica_account.clone(),
            amount_in,
            min_amount_out,
        )
        .map_err(Into::into)
    }
}

impl From<SwapTrx<'_>> for LocalBatch {
    fn from(value: SwapTrx<'_>) -> Self {
        remote::submit_transaction(value.conn, value.trx, "memo", IBC_TIMEOUT)
    }
}

pub(super) struct TransferInTrx<'a> {
    conn: &'a str,
    sender: RemoteSender<'a>,
}

impl<'ica> TransferInTrx<'ica> {
    pub(super) fn new(ica: &'ica Account, now: Instant) -> Self {
        let sender = RemoteSender::new(
            &ica.dex().transfer_channel.remote_endpoint,
            ica.remote(),
            ica.owner(),
            (now + IBC_TIMEOUT).into_timestamp(),
        );
        TransferInTrx {
            conn: &ica.dex().connection_id,
            sender,
        }
    }

    pub fn send<G>(&mut self, amount: &CoinDTO<G>)
    where
        G: Group,
    {
        self.sender.send(amount)
    }
}

impl<'r> From<TransferInTrx<'r>> for LocalBatch {
    fn from(value: TransferInTrx<'r>) -> Self {
        remote::submit_transaction(value.conn, value.sender.into(), "memo", IBC_TIMEOUT)
    }
}
