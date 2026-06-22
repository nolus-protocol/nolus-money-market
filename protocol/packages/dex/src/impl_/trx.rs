use std::marker::PhantomData;

use currency::{Group, MemberOf};
use cw_time::IntoTimestamp;
use finance::instant::Instant;
use finance::{coin::CoinDTO, duration::Duration};
use oracle::stub::SwapPath;
use platform::{
    bank_ibc::{local::Sender as LocalSender, remote::Sender as RemoteSender},
    batch::Batch as LocalBatch,
    ica::{self, HostAccount},
    trx::Transaction,
};
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{Account, Connectable, error::Result, swap::ExactAmountIn};

pub(super) const IBC_TIMEOUT: Duration = Duration::from_days(1); //enough for the relayers to process

/// The ICA host is present on every leg of the legacy ICA path. Remote-lease
/// leases carry no host (`Account::funding`) but never reach these trx
/// builders; the whole ICA path is removed by #649.
const ICA_HOST_REQUIRED: &str = "ICA host present on the legacy ICA path";

pub(super) struct TransferOutTrx<'ica> {
    sender: LocalSender<'ica>,
}

impl<'ica> TransferOutTrx<'ica> {
    pub(super) fn new(ica: &'ica Account, now: Instant) -> Self {
        let host = ica.host().expect(ICA_HOST_REQUIRED);
        Self {
            sender: LocalSender::new(
                &ica.dex().transfer_channel.local_endpoint,
                ica.owner(),
                host,
                (now + IBC_TIMEOUT).into_timestamp(),
                format!(
                    "Transfer out: {sender} -> {receiver}",
                    sender = ica.owner(),
                    receiver = host
                ),
            ),
        }
    }

    pub fn send<G>(&mut self, amount: &CoinDTO<G>)
    where
        G: Group,
    {
        self.sender.send(amount)
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
        Self {
            conn: &ica.dex().connection_id,
            ica_account: ica.host().expect(ICA_HOST_REQUIRED),
            trx: Transaction::default(),
            swap_path,
            querier,
            _group: PhantomData::<SwapGroup>,
        }
    }

    pub fn swap_exact_in<SwapGIn, SwapGOut, SwapClient>(
        &mut self,
        amount_in: &CoinDTO<SwapGIn>,
        min_amount_out: &CoinDTO<SwapGOut>,
    ) -> Result<()>
    where
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
        ica::submit_transaction(value.conn, value.trx, "memo", IBC_TIMEOUT)
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
            ica.host().expect(ICA_HOST_REQUIRED),
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
        ica::submit_transaction(value.conn, value.sender.into(), "memo", IBC_TIMEOUT)
    }
}
