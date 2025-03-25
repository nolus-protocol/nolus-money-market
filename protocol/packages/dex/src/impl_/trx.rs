use std::marker::PhantomData;

use currency::{Group, MemberOf, platform::Nls};
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
};
use oracle::stub::SwapPath;
use platform::{
    bank_ibc::{local::Sender as LocalSender, remote::Sender as RemoteSender},
    batch::Batch as LocalBatch,
    ica::{self, HostAccount},
    trx::Transaction,
};
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{Account, Connectable, error::Result, swap::ExactAmountIn};

pub(super) const IBC_TIMEOUT: Duration = Duration::from_days(1); //enough for the relayers to process

//TODO take them as input from the client
const ICA_TRANSFER_ACK_TIP: Coin<Nls> = Coin::new(1);
const ICA_TRANSFER_TIMEOUT_TIP: Coin<Nls> = ICA_TRANSFER_ACK_TIP;

//TODO take them as input from the client
const ICA_SWAP_ACK_TIP: Coin<Nls> = Coin::new(1);
const ICA_SWAP_TIMEOUT_TIP: Coin<Nls> = ICA_SWAP_ACK_TIP;

pub(super) struct TransferOutTrx<'ica> {
    sender: LocalSender<'ica>,
}

impl<'ica> TransferOutTrx<'ica> {
    pub(super) fn new(ica: &'ica Account, now: Timestamp) -> Self {
        Self {
            sender: LocalSender::new(
                &ica.dex().transfer_channel.local_endpoint,
                ica.owner(),
                ica.host(),
                now + IBC_TIMEOUT,
                ICA_TRANSFER_ACK_TIP,
                ICA_TRANSFER_TIMEOUT_TIP,
                format!(
                    "Transfer out: {sender} -> {receiver}",
                    sender = ica.owner(),
                    receiver = ica.host()
                ),
            ),
        }
    }

    pub fn send<G>(&mut self, amount: &CoinDTO<G>) -> Result<()>
    where
        G: Group,
    {
        self.sender.send(amount).map_err(Into::into)
    }
}

impl<'ica> From<TransferOutTrx<'ica>> for LocalBatch {
    fn from(value: TransferOutTrx<'ica>) -> Self {
        value.sender.into()
    }
}

pub(super) struct SwapTrx<'ica, 'swap_path, 'querier, SwapGroup, SwapPathImpl> {
    conn: &'ica str,
    ica_account: &'ica HostAccount,
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
        let trx = Transaction::default();
        Self {
            conn: &ica.dex().connection_id,
            ica_account: ica.host(),
            trx,
            swap_path,
            querier,
            _group: PhantomData::<SwapGroup>,
        }
    }

    pub fn swap_exact_in<GIn, SwapGIn, SwapGOut, SwapClient>(
        &mut self,
        amount_in: &CoinDTO<GIn>,
        min_amount_out: &CoinDTO<SwapGOut>,
    ) -> Result<()>
    where
        GIn: Group + MemberOf<SwapGIn>,
        SwapGIn: Group + MemberOf<SwapGroup>,
        SwapGOut: Group + MemberOf<SwapGroup>,
        SwapClient: ExactAmountIn,
    {
        self.swap_path
            .swap_path(
                amount_in.currency().into_super_group::<SwapGIn>(),
                min_amount_out.currency(),
                self.querier,
            )
            .map_err(Into::into)
            .and_then(|ref swap_path| {
                SwapClient::build_request(
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

impl<'ica> TransferInTrx<'ica> {
    pub(super) fn new(ica: &'ica Account, now: Timestamp) -> Self {
        let sender = RemoteSender::new(
            &ica.dex().transfer_channel.remote_endpoint,
            ica.host(),
            ica.owner(),
            now + IBC_TIMEOUT,
        );
        TransferInTrx {
            conn: &ica.dex().connection_id,
            sender,
        }
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
