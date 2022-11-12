use std::marker::PhantomData;

use crate::{
    coin_legacy::{self},
    denom::{
        dex::DexMapper, local::BankMapper, CurrencyMapper, DexChainCurrencyMapper,
        LocalChainCurrencyMapper,
    },
    error::{Error, Result},
    ica::Batch as IcaBatch,
};
use finance::{coin::CoinDTO, currency::Group};
use sdk::cosmwasm_std::{Addr, Coin as CwCoin, IbcMsg, IbcTimeout, Timestamp};

use crate::batch::Batch;

pub type LocalChainSender<'c> = Sender<'c, BankMapper>;

pub type DexChainSender<'c> = Sender<'c, DexMapper>;

pub struct Sender<'c, CM> {
    channel: &'c str,
    receiver: &'c Addr,
    timeout: Timestamp,
    currency_mapper: PhantomData<CM>,
    amounts: Vec<CwCoin>,
}

impl<'c, CM> Sender<'c, CM>
where
    CM: CurrencyMapper<'c>,
{
    pub fn new(channel: &'c str, receiver: &'c Addr, timeout: Timestamp) -> Self {
        Self {
            channel,
            receiver,
            timeout,
            currency_mapper: PhantomData::<CM>,
            amounts: vec![],
        }
    }

    pub fn send<G>(&mut self, amount: &CoinDTO<G>) -> Result<()>
    where
        G: Group,
    {
        self.amounts
            .push(coin_legacy::to_cosmwasm_on_network::<G, CM>(amount)?);
        Ok(())
    }

    fn into_ibc_msgs(self) -> impl Iterator<Item = IbcMsg> + 'c {
        let channel = self.channel;
        let receiver = self.receiver;
        let timeout = self.timeout;
        self.amounts
            .into_iter()
            .map(move |amount| new_msg(channel, receiver, amount, timeout))
    }
}

const IBC_TRANSFER_TYPE: &str = "/ibc.applications.transfer.v2.FungibleTokenPacketData";

fn new_msg(channel: &str, receiver: &Addr, amount: CwCoin, timeout: Timestamp) -> IbcMsg {
    IbcMsg::Transfer {
        channel_id: channel.into(),
        to_address: receiver.into(),
        amount,
        timeout: IbcTimeout::with_timestamp(timeout),
    }
}

impl<'c, CM> From<Sender<'c, CM>> for Batch
where
    CM: LocalChainCurrencyMapper + CurrencyMapper<'c>,
{
    fn from(sender: Sender<'c, CM>) -> Self {
        let mut batch = Self::default();

        sender
            .into_ibc_msgs()
            .for_each(|msg| batch.schedule_execute_no_reply(msg));
        batch
    }
}

impl<'c, CM> TryFrom<Sender<'c, CM>> for IcaBatch
where
    CM: DexChainCurrencyMapper + CurrencyMapper<'c>,
{
    type Error = Error;
    fn try_from(sender: Sender<'c, CM>) -> Result<Self> {
        let mut batch = Self::default();

        sender
            .into_ibc_msgs()
            .try_for_each(|msg| batch.add_message(IBC_TRANSFER_TYPE, msg))?;
        Ok(batch)
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
        bank_ibc::{new_msg, Sender, IBC_TRANSFER_TYPE},
        batch::Batch,
        coin_legacy::{self},
        denom::{dex::DexMapper, local::BankMapper},
        ica::Batch as IcaBatch,
    };

    #[test]
    fn local_send() {
        let channel = "channel-0";
        let receiver = Addr::unchecked("receiver");
        let timeout = Timestamp::from_seconds(100);
        let mut sender = Sender::<BankMapper>::new(channel, &receiver, timeout);

        let coin1: Coin<Dai> = 234214.into();
        let coin2: Coin<Usdc> = 234214.into();
        sender.send::<TestExtraCurrencies>(&coin1.into()).unwrap();
        sender.send::<TestExtraCurrencies>(&coin2.into()).unwrap();

        assert_eq!(Batch::from(sender), {
            let mut batch = Batch::default();
            batch.schedule_execute_no_reply(new_msg(
                channel,
                &receiver,
                coin_legacy::to_cosmwasm_impl(coin1),
                timeout,
            ));
            batch.schedule_execute_no_reply(new_msg(
                channel,
                &receiver,
                coin_legacy::to_cosmwasm_impl(coin2),
                timeout,
            ));
            batch
        });
    }

    #[test]
    fn remote_send() {
        let channel = "channel-1045";
        let receiver = Addr::unchecked("receiver");
        let timeout = Timestamp::from_seconds(100);
        let mut sender = Sender::<DexMapper>::new(channel, &receiver, timeout);

        let coin1: Coin<Nls> = 63.into();
        let coin2: Coin<Usdc> = 2.into();
        sender.send::<TestExtraCurrencies>(&coin1.into()).unwrap();
        sender.send::<TestExtraCurrencies>(&coin2.into()).unwrap();

        assert_eq!(IcaBatch::try_from(sender), {
            let mut batch = IcaBatch::default();
            batch
                .add_message(
                    IBC_TRANSFER_TYPE,
                    new_msg(
                        channel,
                        &receiver,
                        coin_legacy::to_cosmwasm_on_network::<TestExtraCurrencies, DexMapper>(
                            &coin1.into(),
                        )
                        .unwrap(),
                        timeout,
                    ),
                )
                .unwrap();
            batch
                .add_message(
                    IBC_TRANSFER_TYPE,
                    new_msg(
                        channel,
                        &receiver,
                        coin_legacy::to_cosmwasm_on_network::<TestExtraCurrencies, DexMapper>(
                            &coin2.into(),
                        )
                        .unwrap(),
                        timeout,
                    ),
                )
                .unwrap();
            Ok(batch)
        });
    }
}
