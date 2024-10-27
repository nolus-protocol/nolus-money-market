use currencies::PaymentGroup;

use currency::{CurrencyDef, Group, MemberOf};
use platform::{contract, response};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Addr, DepsMut, Env},
};

use crate::{
    api::{DispatchAlarmsResponse, ExecuteMsg},
    contract::alarms::MarketAlarms,
    error::ContractError,
    result::ContractResult,
};

use super::{oracle::feeder::Feeders, Oracle};

pub fn do_executute<BaseCurrency, BaseCurrencies, AlarmCurrencies>(
    deps: DepsMut<'_>,
    env: Env,
    msg: ExecuteMsg<BaseCurrency, BaseCurrencies, AlarmCurrencies, PaymentGroup>,
    sender: Addr,
) -> ContractResult<CwResponse>
where
    BaseCurrency: CurrencyDef,
    BaseCurrency::Group:
        MemberOf<BaseCurrencies> + MemberOf<PaymentGroup> + MemberOf<AlarmCurrencies::TopG>,
    BaseCurrencies: Group + MemberOf<PaymentGroup>,
    AlarmCurrencies: Group,
{
    match msg {
        ExecuteMsg::FeedPrices { prices } => Feeders::is_feeder(deps.storage, &sender)
            .map_err(ContractError::LoadFeeders)
            .and_then(|found| {
                if found {
                    Ok(())
                } else {
                    Err(ContractError::UnknownFeeder {})
                }
            })
            .and_then(|()| Oracle::load(deps.storage))
            .and_then(|mut oracle| oracle.try_feed_prices(env.block.time, sender, prices))
            .map(|()| Default::default()),
        ExecuteMsg::DispatchAlarms { max_count } => Oracle::load(deps.storage)?
            .try_notify_alarms(env.block.time, max_count)
            .and_then(|(total, resp)| {
                response::response_with_messages(DispatchAlarmsResponse(total), resp)
            }),
        ExecuteMsg::AddPriceAlarm { alarm } => {
            contract::validate_addr(deps.querier, &sender)?;

            MarketAlarms::new(deps.storage)
                .try_add_price_alarm(sender, alarm)
                .map(|()| Default::default())
        }
    }
}
