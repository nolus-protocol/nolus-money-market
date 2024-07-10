use finance::price::dto::PriceDTO;

use currencies::PaymentGroup as PriceCurrencies;
use currency::{Currency, Group};
use platform::{contract, response};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Addr, DepsMut, Env, Storage, Timestamp},
};

use crate::{
    api::{BaseCurrencies, BaseCurrency, Config, DispatchAlarmsResponse, ExecuteMsg},
    contract::{alarms::MarketAlarms, oracle::Oracle},
    error::ContractError,
    result::ContractResult,
};

use super::oracle::{feed::Feeds, feeder::Feeders};

pub fn do_executute(
    deps: DepsMut<'_>,
    env: Env,
    msg: ExecuteMsg,
    sender: Addr,
) -> ContractResult<CwResponse> {
    match msg {
        ExecuteMsg::FeedPrices { prices } => {
            if !Feeders::is_feeder(deps.storage, &sender).map_err(ContractError::LoadFeeders)? {
                return Err(ContractError::UnknownFeeder {});
            }

            try_feed_prices::<PriceCurrencies, BaseCurrency, PriceCurrencies>(
                deps.storage,
                env.block.time,
                sender,
                prices,
            )
            .map(|()| Default::default())
        }
        ExecuteMsg::DispatchAlarms { max_count } => {
            Oracle::<_, PriceCurrencies, BaseCurrency, BaseCurrencies>::load(deps.storage)?
                .try_notify_alarms(env.block.time, max_count)
                .and_then(|(total, resp)| {
                    response::response_with_messages(DispatchAlarmsResponse(total), resp)
                })
        }
        ExecuteMsg::AddPriceAlarm { alarm } => {
            contract::validate_addr(deps.querier, &sender)?;

            MarketAlarms::new(deps.storage)
                .try_add_price_alarm(sender, alarm)
                .map(|()| Default::default())
        }
    }
}

fn try_feed_prices<G, BaseC, QuoteG>(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    sender: Addr,
    prices: Vec<PriceDTO<G, G>>,
) -> ContractResult<()>
where
    G: Group,
    BaseC: Currency,
    QuoteG: Group,
{
    Config::load(storage)
        .map(|cfg| Feeds::<G, BaseC, QuoteG>::with(cfg.price_config))
        .and_then(|oracle| oracle.feed_prices(storage, block_time, &sender, &prices))
}
