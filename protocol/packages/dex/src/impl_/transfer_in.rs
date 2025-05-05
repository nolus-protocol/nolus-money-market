use currency::CurrencyDef;
use finance::{coin::Coin, duration::Duration};
use platform::{bank, batch::Batch};
use sdk::cosmwasm_std::{Addr, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::error::Result;

const POLLING_INTERVAL: Duration = Duration::from_secs(5);

pub(super) fn check_received<C>(
    expected_payment: &Coin<C>,
    account: &Addr,
    querier: QuerierWrapper<'_>,
) -> Result<bool>
where
    C: CurrencyDef,
{
    bank::balance(account, querier)
        .map_err(Into::into)
        .map(|ref balance| balance >= expected_payment)
}

pub(super) fn setup_alarm(time_alarms: &TimeAlarmsRef, now: Timestamp) -> Result<Batch> {
    time_alarms
        .setup_alarm(now + POLLING_INTERVAL)
        .map_err(Into::into)
}
