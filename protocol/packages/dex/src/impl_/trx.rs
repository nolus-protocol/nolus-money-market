use std::marker::PhantomData;

use currency::{Group, MemberOf};
use cw_time::IntoTimestamp;
use finance::{coin::CoinDTO, instant::Instant};
use oracle::stub::SwapPath;
use platform::{
    bank_ibc::remote::Sender as RemoteSender,
    batch::Batch as LocalBatch,
    remote::{self, Account as RemoteAccount},
    trx::Transaction,
};
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{Account, Connectable, IBC_TIMEOUT, error::Result, transport::Transport};

pub(super) struct SwapTrx<'ica, 'swap_path, 'querier, SwapGroup, SwapPathImpl> {
    conn: &'ica str,
    ica_account: &'ica RemoteAccount,
    trx: Transaction,
    swap_path: &'swap_path SwapPathImpl,
    querier: QuerierWrapper<'querier>,
    _group: PhantomData<SwapGroup>,
}

impl<'ica, 'swap_path, 'querier, SwapGroup, SwapPathImpl>
    SwapTrx<'ica, 'swap_path, 'querier, SwapGroup, SwapPathImpl>
where
    SwapGroup: Group,
    SwapPathImpl: SwapPath<SwapGroup>,
{
    pub(super) fn new(
        ica: &'ica Account,
        swap_path: &'swap_path SwapPathImpl,
        querier: QuerierWrapper<'querier>,
    ) -> Self {
        Self {
            conn: &ica.dex().connection_id,
            ica_account: ica.remote(),
            trx: Transaction::default(),
            swap_path,
            querier,
            _group: PhantomData::<SwapGroup>,
        }
    }

    pub fn swap_exact_in<SwapGIn, SwapGOut, TransportImpl>(
        &mut self,
        amount_in: &CoinDTO<SwapGIn>,
        min_amount_out: &CoinDTO<SwapGOut>,
    ) -> Result<()>
    where
        SwapGIn: Group + MemberOf<SwapGroup>,
        SwapGOut: Group + MemberOf<SwapGroup>,
        TransportImpl: Transport,
    {
        self.swap_path
            .swap_path(
                amount_in.currency().into_super_group::<SwapGIn>(),
                min_amount_out.currency(),
                self.querier,
            )
            .map_err(Into::into)
            .and_then(|ref swap_path| {
                TransportImpl::build_request(
                    &mut self.trx,
                    self.ica_account.clone(),
                    amount_in,
                    min_amount_out,
                    swap_path,
                )
                .map_err(Into::into)
            })
    }
}

impl<SwapGroup, SwapPathImpl> From<SwapTrx<'_, '_, '_, SwapGroup, SwapPathImpl>> for LocalBatch {
    fn from(value: SwapTrx<'_, '_, '_, SwapGroup, SwapPathImpl>) -> Self {
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
