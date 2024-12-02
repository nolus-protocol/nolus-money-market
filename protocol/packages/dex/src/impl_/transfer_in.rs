use std::marker::PhantomData;

use currency::{CurrencyDef, Group, MemberOf};
use finance::{
    coin::{Coin, CoinDTO, WithCoin, WithCoinResult},
    duration::Duration,
};
use platform::{bank, batch::Batch};
use sdk::cosmwasm_std::{Addr, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{error::Result, Error};

const POLLING_INTERVAL: Duration = Duration::from_secs(5);

pub(super) fn check_received<G>(
    payment: &CoinDTO<G>,
    account: &Addr,
    querier: QuerierWrapper<'_>,
) -> Result<bool>
where
    G: Group,
{
    struct CheckBalance<'a, G> {
        account: &'a Addr,
        querier: QuerierWrapper<'a>,
        currency_g: PhantomData<G>,
    }
    impl<G> WithCoin<G> for CheckBalance<'_, G>
    where
        G: Group,
    {
        type Output = bool;
        type Error = Error;

        fn on<C>(self, expected_payment: Coin<C>) -> WithCoinResult<G, Self>
        where
            C: CurrencyDef,
            C::Group: MemberOf<G>,
        {
            let received = bank::balance(self.account, self.querier)? >= expected_payment;
            Ok(received)
        }
    }

    payment.with_coin(CheckBalance::<G> {
        account,
        querier,
        currency_g: PhantomData,
    })
}

pub(super) fn setup_alarm(time_alarms: &TimeAlarmsRef, now: Timestamp) -> Result<Batch> {
    time_alarms
        .setup_alarm(now + POLLING_INTERVAL)
        .map_err(Into::into)
}
