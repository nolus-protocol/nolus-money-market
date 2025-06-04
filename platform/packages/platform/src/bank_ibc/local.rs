use currency::{BankSymbols, Group};
use finance::coin::CoinDTO;
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin, Timestamp},
    neutron_sdk::{
        bindings::msg::{IbcFee, NeutronMsg},
        sudo::msg::RequestPacketTimeoutHeight,
    },
};

use crate::{batch::Batch, coin_legacy, ica::HostAccount, result::Result};

pub struct Sender<'conn> {
    channel: &'conn str,
    sender: &'conn Addr,
    receiver: &'conn HostAccount,
    timeout: Timestamp,
    amounts: Vec<CwCoin>,
    memo: String,
}

impl<'conn> Sender<'conn> {
    pub fn new(
        channel: &'conn str,
        sender: &'conn Addr,
        receiver: &'conn HostAccount,
        timeout: Timestamp,
        memo: String,
    ) -> Self {
        Self {
            channel,
            sender,
            receiver,
            timeout,
            amounts: vec![],
            memo,
        }
    }

    pub fn send<G>(&mut self, amount: &CoinDTO<G>) -> Result<()>
    where
        G: Group,
    {
        coin_legacy::to_cosmwasm_on_network::<BankSymbols<G>>(amount).map(|coin| {
            self.amounts.push(coin);
        })
    }

    fn into_ibc_msgs(self) -> impl Iterator<Item = NeutronMsg> {
        let Self {
            channel,
            sender,
            receiver,
            timeout,
            amounts,
            memo,
        } = self;

        amounts.into_iter().map(move |amount: CwCoin| {
            new_msg(
                channel,
                sender.clone(),
                receiver.clone(),
                amount,
                timeout,
                memo.clone(),
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
    memo: String,
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
            ack_fee: vec![],
            timeout_fee: vec![],
        },
        memo,
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
    use currency::test::{SubGroupTestC10, SuperGroup, SuperGroupTestC1};
    use finance::coin::Coin;
    use sdk::cosmwasm_std::{Addr, Timestamp};

    use crate::{
        bank_ibc::local::{Sender, new_msg},
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
        let mut funds_sender = Sender::new(channel, &sender, &receiver, timeout, "MEMO".into());

        let coin1: Coin<SubGroupTestC10> = 234214.into();
        let coin2: Coin<SuperGroupTestC1> = 234214.into();
        funds_sender.send::<SuperGroup>(&coin1.into()).unwrap();
        funds_sender.send::<SuperGroup>(&coin2.into()).unwrap();

        assert_eq!(Batch::from(funds_sender), {
            let mut batch = Batch::default();
            batch.schedule_execute_no_reply(new_msg(
                channel,
                sender.clone(),
                receiver.clone(),
                coin_legacy::to_cosmwasm_on_nolus(coin1),
                timeout,
                "MEMO".into(),
            ));
            batch.schedule_execute_no_reply(new_msg(
                channel,
                sender,
                receiver,
                coin_legacy::to_cosmwasm_on_nolus(coin2),
                timeout,
                "MEMO".into(),
            ));
            batch
        });
    }
}
