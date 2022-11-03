use crate::{
    error::{Error, Result},
    ica::Batch as IcaBatch,
};
use finance::coin::CoinDTO;
use sdk::cosmwasm_std::{Addr, Coin as CwCoin, IbcMsg, IbcTimeout, Timestamp};

use crate::batch::Batch;

pub struct Sender<'c> {
    channel: &'c str,
    receiver: &'c Addr,
    timeout: Timestamp,
    amounts: Vec<CwCoin>,
}

impl<'c> Sender<'c> {
    pub fn new(channel: &'c str, receiver: &'c Addr, timeout: Timestamp) -> Self {
        Self {
            channel,
            receiver,
            timeout,
            amounts: vec![],
        }
    }

    pub fn send(&mut self, amount: CoinDTO) {
        self.amounts
            .push(CwCoin::new(amount.amount(), amount.ticker()));
    }
}

fn new_msg(channel: &str, receiver: &Addr, amount: CwCoin, timeout: Timestamp) -> IbcMsg {
    IbcMsg::Transfer {
        channel_id: channel.into(),
        to_address: receiver.into(),
        amount,
        timeout: IbcTimeout::with_timestamp(timeout),
    }
}

impl<'c> From<Sender<'c>> for Batch {
    fn from(sender: Sender) -> Self {
        let mut batch = Self::default();

        sender
            .amounts
            .into_iter()
            .map(|amount| new_msg(sender.channel, sender.receiver, amount, sender.timeout))
            .for_each(|msg| batch.schedule_execute_no_reply(msg));
        batch
    }
}

impl<'c> TryFrom<Sender<'c>> for IcaBatch {
    type Error = Error;
    fn try_from(sender: Sender<'c>) -> Result<Self> {
        const IBC_TRANSFER_TYPE: &str = "/ibc.applications.transfer.v2.FungibleTokenPacketData";

        let mut batch = Self::default();

        sender
            .amounts
            .into_iter()
            .map(|amount| new_msg(sender.channel, sender.receiver, amount, sender.timeout))
            .try_for_each(|msg| batch.add_message(IBC_TRANSFER_TYPE, msg))?;
        Ok(batch)
    }
}
