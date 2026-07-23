use currency::Group;
use cw_time::IntoTimestamp;
use finance::{coin::CoinDTO, instant::Instant};
use platform::{bank_ibc::remote::Sender as RemoteSender, batch::Batch as LocalBatch, remote};

use crate::{Account, Connectable, IBC_TIMEOUT};

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
