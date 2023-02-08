use sdk::cosmwasm_std::{Addr, QuerierWrapper, Timestamp};
use finance::{
    coin::{Coin, CoinDTO, WithCoin, WithCoinResult},
    currency::{Currency, Group},
    duration::Duration,
};
use platform::{bank, batch::Batch};
use timealarms::stub::{TimeAlarms, TimeAlarmsRef, WithTimeAlarms};

use crate::error::{ContractError, ContractResult};

const POLLING_INTERVAL: Duration = Duration::from_secs(5);

pub(super) fn check_received<G>(
    payment: &CoinDTO<G>,
    account: &Addr,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<bool>
where
    G: Group,
{
    struct CheckBalance<'a> {
        account: &'a Addr,
        querier: &'a QuerierWrapper<'a>,
    }
    impl<'a> WithCoin for CheckBalance<'a> {
        type Output = bool;
        type Error = ContractError;

        fn on<C>(&self, expected_payment: Coin<C>) -> WithCoinResult<Self>
        where
            C: Currency,
        {
            let received = bank::balance(self.account, self.querier)? >= expected_payment;
            Ok(received)
        }
    }

    payment.with_coin(CheckBalance { account, querier })
}

pub(super) fn setup_alarm(time_alarms: TimeAlarmsRef, now: Timestamp) -> ContractResult<Batch> {
    struct SetupAlarm(Timestamp);
    impl WithTimeAlarms for SetupAlarm {
        type Output = Batch;
        type Error = ContractError;

        fn exec<TA>(self, mut time_alarms: TA) -> Result<Self::Output, Self::Error>
        where
            TA: TimeAlarms,
        {
            time_alarms.add_alarm(self.0 + POLLING_INTERVAL)?;
            Ok(time_alarms.into().batch)
        }
    }

    time_alarms.execute(SetupAlarm(now))
}
