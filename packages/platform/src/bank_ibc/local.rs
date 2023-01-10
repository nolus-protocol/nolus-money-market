use finance::{coin::CoinDTO, currency::Group};
use sdk::cosmwasm_std::{Coin as CwCoin, IbcMsg, IbcTimeout, Timestamp};

use crate::{
    batch::Batch,
    coin_legacy::{self},
    denom::local::BankMapper,
    error::Result,
    ica::HostAccount,
};

pub struct Sender<'c> {
    channel: &'c str,
    receiver: HostAccount,
    timeout: Timestamp,
    amounts: Vec<CwCoin>,
}

impl<'c> Sender<'c> {
    pub fn new(channel: &'c str, receiver: HostAccount, timeout: Timestamp) -> Self {
        Self {
            channel,
            receiver,
            timeout,
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

    fn into_ibc_msgs(self) -> impl Iterator<Item = IbcMsg> + 'c {
        let channel = self.channel;
        let receiver = self.receiver;
        let timeout = self.timeout;
        self.amounts
            .into_iter()
            .map(move |amount| new_msg(channel, receiver.clone(), amount, timeout))
    }
}

fn new_msg(channel: &str, receiver: HostAccount, amount: CwCoin, timeout: Timestamp) -> IbcMsg {
    IbcMsg::Transfer {
        channel_id: channel.into(),
        to_address: receiver.into(),
        amount,
        timeout: IbcTimeout::with_timestamp(timeout),
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
        test::currency::{Dai, TestExtraCurrencies, Usdc},
    };
    use sdk::cosmwasm_std::Timestamp;

    use crate::{
        bank_ibc::local::{new_msg, Sender},
        batch::Batch,
        coin_legacy::{self},
        ica::HostAccount,
    };

    #[test]
    fn send() {
        let channel = "channel-0";
        let receiver = HostAccount::try_from(String::from("receiver")).unwrap();
        let timeout = Timestamp::from_seconds(100);
        let mut sender = Sender::new(channel, receiver.clone(), timeout);

        let coin1: Coin<Dai> = 234214.into();
        let coin2: Coin<Usdc> = 234214.into();
        sender.send::<TestExtraCurrencies>(&coin1.into()).unwrap();
        sender.send::<TestExtraCurrencies>(&coin2.into()).unwrap();

        assert_eq!(Batch::from(sender), {
            let mut batch = Batch::default();
            batch.schedule_execute_no_reply(new_msg(
                channel,
                receiver.clone(),
                coin_legacy::to_cosmwasm_impl(coin1),
                timeout,
            ));
            batch.schedule_execute_no_reply(new_msg(
                channel,
                receiver,
                coin_legacy::to_cosmwasm_impl(coin2),
                timeout,
            ));
            batch
        });
    }
}
