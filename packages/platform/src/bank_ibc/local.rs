use finance::{
    coin::{Coin, CoinDTO},
    currency::{Currency, Group},
};
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin, Timestamp},
    neutron_sdk::{
        bindings::msg::{IbcFee, NeutronMsg},
        sudo::msg::RequestPacketTimeoutHeight,
    },
};

use crate::{
    batch::Batch,
    coin_legacy::{self},
    denom::local::BankMapper,
    error::Result,
    ica::HostAccount,
};

pub struct Sender<'c> {
    channel: &'c str,
    sender: Addr,
    receiver: HostAccount,
    timeout: Timestamp,
    ack_fee: CwCoin,
    timeout_fee: CwCoin,
    amounts: Vec<CwCoin>,
}

impl<'c> Sender<'c> {
    pub fn new<C>(
        channel: &'c str,
        sender: Addr,
        receiver: HostAccount,
        timeout: Timestamp,
        ack_fee: Coin<C>,
        timeout_fee: Coin<C>,
    ) -> Self
    where
        C: Currency,
    {
        Self {
            channel,
            sender,
            receiver,
            timeout,
            ack_fee: coin_legacy::to_cosmwasm_impl(ack_fee),
            timeout_fee: coin_legacy::to_cosmwasm_impl(timeout_fee),
            amounts: vec![],
        }
    }

    pub fn send<G>(&mut self, amount: &CoinDTO<G>) -> Result<()>
    where
        G: Group,
    {
        self.amounts
            .push(coin_legacy::to_cosmwasm_on_network::<G, BankMapper>(
                amount,
            )?);
        Ok(())
    }

    fn into_ibc_msgs(self) -> impl Iterator<Item = NeutronMsg> + 'c {
        let channel = self.channel;
        let sender = self.sender;
        let receiver = self.receiver;
        let timeout = self.timeout;
        let ack_fee = self.ack_fee;
        let timeout_fee = self.timeout_fee;
        self.amounts.into_iter().map(move |amount| {
            new_msg(
                channel,
                sender.clone(),
                receiver.clone(),
                amount,
                timeout,
                ack_fee.clone(),
                timeout_fee.clone(),
            )
        })
    }
}

const ICS20_PORT_AT_NOLUS: &str = "transfer";

fn new_msg(
    channel: &str,
    sender: Addr,
    receiver: HostAccount,
    amount: CwCoin,
    timeout: Timestamp,
    ack_fee: CwCoin,
    timeout_fee: CwCoin,
) -> NeutronMsg {
    let timeout_height = RequestPacketTimeoutHeight {
        revision_height: None,
        revision_number: None,
    };
    NeutronMsg::IbcTransfer {
        source_port: ICS20_PORT_AT_NOLUS.into(),
        source_channel: channel.into(),
        token: amount,
        sender: sender.into(),
        receiver: receiver.into(),
        timeout_height,
        timeout_timestamp: timeout.nanos(),
        fee: IbcFee {
            recv_fee: vec![],
            ack_fee: vec![ack_fee],
            timeout_fee: vec![timeout_fee],
        },
    }
}

impl<'c> From<Sender<'c>> for Batch {
    fn from(sender: Sender<'c>) -> Self {
        let mut batch = Self::default();

        sender
            .into_ibc_msgs()
            .for_each(|msg| batch.schedule_execute_no_reply(msg));
        batch
    }
}

#[cfg(test)]
mod test {
    use finance::{
        coin::Coin,
        test::currency::{Dai, Nls, TestExtraCurrencies, Usdc},
    };
    use sdk::cosmwasm_std::{Addr, Timestamp};

    use crate::{
        bank_ibc::local::{new_msg, Sender},
        batch::Batch,
        coin_legacy::{self},
        ica::HostAccount,
    };

    #[test]
    fn send() {
        let channel = "channel-0";
        let sender = Addr::unchecked("sender");
        let receiver = HostAccount::try_from(String::from("receiver")).unwrap();
        let timeout = Timestamp::from_seconds(100);
        let ack_fee = Coin::<Nls>::new(100);
        let timeout_fee = Coin::<Nls>::new(50);
        let mut funds_sender = Sender::new(
            channel,
            sender.clone(),
            receiver.clone(),
            timeout,
            ack_fee,
            timeout_fee,
        );

        let coin1: Coin<Dai> = 234214.into();
        let coin2: Coin<Usdc> = 234214.into();
        funds_sender
            .send::<TestExtraCurrencies>(&coin1.into())
            .unwrap();
        funds_sender
            .send::<TestExtraCurrencies>(&coin2.into())
            .unwrap();

        assert_eq!(Batch::from(funds_sender), {
            let mut batch = Batch::default();
            batch.schedule_execute_no_reply(new_msg(
                channel,
                sender.clone(),
                receiver.clone(),
                coin_legacy::to_cosmwasm_impl(coin1),
                timeout,
                coin_legacy::to_cosmwasm_impl(ack_fee),
                coin_legacy::to_cosmwasm_impl(timeout_fee),
            ));
            batch.schedule_execute_no_reply(new_msg(
                channel,
                sender,
                receiver,
                coin_legacy::to_cosmwasm_impl(coin2),
                timeout,
                coin_legacy::to_cosmwasm_impl(ack_fee),
                coin_legacy::to_cosmwasm_impl(timeout_fee),
            ));
            batch
        });
    }
}
