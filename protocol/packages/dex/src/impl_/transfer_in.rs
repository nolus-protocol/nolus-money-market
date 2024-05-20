use currency::{Currency, Group};
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
    struct CheckBalance<'a> {
        account: &'a Addr,
        querier: QuerierWrapper<'a>,
    }
    impl<'a> WithCoin for CheckBalance<'a> {
        type Output = bool;
        type Error = Error;

        fn on<C>(self, expected_payment: Coin<C>) -> WithCoinResult<Self>
        where
            C: Currency,
        {
            let received = bank::balance(self.account, self.querier)? >= expected_payment;
            Ok(received)
        }
    }

    payment.with_coin(CheckBalance { account, querier })
}

pub(super) fn setup_alarm(time_alarms: &TimeAlarmsRef, now: Timestamp) -> Result<Batch> {
    time_alarms
        .setup_alarm(now + POLLING_INTERVAL)
        .map_err(Into::into)
}
