use finance::price::dto::PriceDTO;

use currency::{CurrencyDef, Group, MemberOf};
use platform::{contract, response};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Addr, DepsMut, Env, Storage, Timestamp},
};

use crate::{
    api::{Config, DispatchAlarmsResponse, ExecuteMsg},
    contract::alarms::MarketAlarms,
    error::ContractError,
    result::ContractResult,
};

use super::{
    oracle::{feed::Feeds, feeder::Feeders},
    Oracle,
};

pub fn do_executute<BaseCurrency, BaseCurrencies, AlarmCurrencies, PriceCurrencies>(
    deps: DepsMut<'_>,
    env: Env,
    msg: ExecuteMsg<BaseCurrency, BaseCurrencies, AlarmCurrencies, PriceCurrencies>,
    sender: Addr,
) -> ContractResult<CwResponse>
where
    BaseCurrency: CurrencyDef,
    BaseCurrency::Group:
        MemberOf<BaseCurrencies> + MemberOf<PriceCurrencies> + MemberOf<AlarmCurrencies::TopG>,
    BaseCurrencies: Group + MemberOf<PriceCurrencies>,
    AlarmCurrencies: Group,
    PriceCurrencies: Group<TopG = PriceCurrencies>,
{
    match msg {
        ExecuteMsg::FeedPrices { prices } => {
            if !Feeders::is_feeder(deps.storage, &sender).map_err(ContractError::LoadFeeders)? {
                return Err(ContractError::UnknownFeeder {});
            }

            try_feed_prices::<PriceCurrencies, BaseCurrency, BaseCurrencies>(
                deps.storage,
                env.block.time,
                sender,
                prices,
            )
            .map(|()| Default::default())
        }
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

fn try_feed_prices<G, BaseC, BaseG>(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    sender: Addr,
    prices: Vec<PriceDTO<G>>,
) -> ContractResult<()>
where
    G: Group<TopG = G>,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<G>,
    BaseG: Group + MemberOf<G>,
{
    Config::load(storage)
        .map(|cfg| Feeds::<G, BaseC, BaseG>::with(cfg.price_config))
        .and_then(|feeds| feeds.feed_prices(storage, block_time, &sender, &prices))
}
