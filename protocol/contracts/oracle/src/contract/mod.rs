use currency::Currency;
use platform::{
    batch::{Emit, Emitter},
    response,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Storage, SubMsgResult,
    },
};
use serde::Serialize;
use versioning::{package_version, version, FullUpdateOutput, SemVer, Version, VersionSegment};

use crate::{
    api::{
        BaseCurrencies, BaseCurrency, Config, ExecuteMsg, InstantiateMsg, MigrateMsg,
        PriceCurrencies, PricesResponse, QueryMsg, SudoMsg, SwapTreeResponse,
    },
    contract::{alarms::MarketAlarms, oracle::Oracle},
    error::ContractError,
    result::ContractResult,
    state::supported_pairs::SupportedPairs,
};

use self::{config::query_config, oracle::feeder::Feeders};

mod alarms;
mod config;
pub mod exec;
mod oracle;

const FROM_CONTRACT_STORAGE_VERSION: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 1;
const PACKAGE_VERSION: SemVer = package_version!();
const CONTRACT_VERSION: Version = version!(CONTRACT_STORAGE_VERSION, PACKAGE_VERSION);

#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    versioning::initialize(deps.storage, CONTRACT_VERSION)
        .map_err(ContractError::InitializeVersioning)?;

    msg.config
        .store(deps.storage)
        .map_err(ContractError::StoreConfig)
        .and_then(|()| {
            SupportedPairs::<BaseCurrency>::new(msg.swap_tree.into_tree(), msg.stable_currency)
        })
        .and_then(|supported_pairs| supported_pairs.save(deps.storage))
        .map(|()| response::empty_response())
}

#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    MigrateMsg {}: MigrateMsg,
) -> ContractResult<CwResponse> {
    versioning::update_software_and_storage::<FROM_CONTRACT_STORAGE_VERSION, _, _, _, _>(
        deps.storage,
        CONTRACT_VERSION,
        |storage| SupportedPairs::<BaseCurrency>::migrate(storage),
        ContractError::UpdateSoftware,
    )
    .and_then(
        |FullUpdateOutput {
             release_label,
             storage_migration_output: (),
         }| response::response(release_label),
    )
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    type QueryOracle<'storage, S> =
        Oracle<'storage, S, PriceCurrencies, BaseCurrency, BaseCurrencies>;

    match msg {
        QueryMsg::ContractVersion {} => to_json_binary(&package_version!()),
        QueryMsg::Config {} => to_json_binary(&query_config(deps.storage)?),
        QueryMsg::Feeders {} => Feeders::get(deps.storage)
            .map_err(ContractError::LoadFeeders)
            .and_then(|ref feeders| to_json_binary(feeders)),
        QueryMsg::IsFeeder { address } => Feeders::is_feeder(deps.storage, &address)
            .map_err(ContractError::LoadFeeders)
            .and_then(|ref f| to_json_binary(&f)),
        QueryMsg::BaseCurrency {} => to_json_binary(BaseCurrency::TICKER),
        QueryMsg::StableCurrency {} => {
            to_json_binary(SupportedPairs::<BaseCurrency>::load(deps.storage)?.stable_currency())
        }
        QueryMsg::SupportedCurrencyPairs {} => to_json_binary(
            &SupportedPairs::<BaseCurrency>::load(deps.storage)?
                .swap_pairs_df()
                .collect::<Vec<_>>(),
        ),
        QueryMsg::Currencies {} => to_json_binary(
            &SupportedPairs::<BaseCurrency>::load(deps.storage)?
                .currencies()
                .collect::<Vec<_>>(),
        ),
        QueryMsg::Price { currency } => to_json_binary(
            &QueryOracle::load(deps.storage)?.try_query_price(env.block.time, &currency)?,
        ),
        QueryMsg::Prices {} => {
            let prices = QueryOracle::load(deps.storage)?.try_query_prices(env.block.time)?;

            to_json_binary(&PricesResponse { prices })
        }
        QueryMsg::SwapPath { from, to } => to_json_binary(
            &SupportedPairs::<BaseCurrency>::load(deps.storage)?.load_swap_path(&from, &to)?,
        ),
        QueryMsg::SwapTree {} => to_json_binary(&SwapTreeResponse {
            tree: SupportedPairs::<BaseCurrency>::load(deps.storage)?
                .query_swap_tree()
                .into_human_readable(),
        }),
        QueryMsg::AlarmsStatus {} => {
            to_json_binary(&QueryOracle::load(deps.storage)?.try_query_alarms(env.block.time)?)
        }
    }
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    exec::do_executute(deps, env, msg, info.sender)
}

#[entry_point]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    match msg {
        SudoMsg::UpdateConfig(price_config) => Config::update(deps.storage, price_config),
        SudoMsg::RegisterFeeder { feeder_address } => Feeders::try_register(deps, feeder_address),
        SudoMsg::RemoveFeeder { feeder_address } => Feeders::try_remove(deps, feeder_address),
        SudoMsg::SwapTree {
            stable_currency,
            tree,
        } => SupportedPairs::<BaseCurrency>::new(tree.into_tree(), stable_currency)
            .and_then(|supported_pairs| supported_pairs.save(deps.storage)),
    }
    .map(|()| response::empty_response())
}

// TODO: compare gas usage of this solution vs reply on error
#[entry_point]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> ContractResult<CwResponse> {
    const EVENT_TYPE: &str = "market-alarm";
    const KEY_DELIVERED: &str = "delivered";
    const KEY_DETAILS: &str = "details";

    let mut alarms: MarketAlarms<'_, &mut (dyn Storage + '_), PriceCurrencies> =
        MarketAlarms::new(deps.storage);

    let emitter: Emitter = Emitter::of_type(EVENT_TYPE);

    match msg.result {
        SubMsgResult::Ok(_) => alarms
            .last_delivered()
            .map(|()| emitter.emit(KEY_DELIVERED, "success")),
        SubMsgResult::Err(error) => alarms.last_failed().map(|()| {
            emitter
                .emit(KEY_DELIVERED, "error")
                .emit(KEY_DETAILS, error)
        }),
    }
    .map(response::response_only_messages)
}

fn to_json_binary<T>(data: &T) -> ContractResult<Binary>
where
    T: Serialize + ?Sized,
{
    cosmwasm_std::to_json_binary(data).map_err(ContractError::ConvertToBinary)
}

#[cfg(test)]
mod tests {
    use currencies::{
        test::{LeaseC1, PaymentC1, PaymentC5, StableC},
        LeaseGroup, Lpns,
    };
    use currency::Currency;
    use finance::{duration::Duration, percent::Percent, price};
    use sdk::cosmwasm_std::{self, testing::mock_env};

    use crate::{
        api::{swap::SwapTarget, Alarm, Config, ExecuteMsg, QueryMsg, SwapLeg},
        contract::query,
        swap_tree,
        tests::{dummy_instantiate_msg, setup_test},
    };

    #[test]
    fn proper_initialization() {
        use marketprice::config::Config as PriceConfig;
        let msg = dummy_instantiate_msg(
            60,
            Percent::from_percent(50),
            swap_tree!({ base: StableC::TICKER }, (1, PaymentC5::TICKER)),
            StableC::TICKER.into(),
        );
        let (deps, _info) = setup_test(msg);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: Config = cosmwasm_std::from_json(res).unwrap();
        assert_eq!(
            Config {
                price_config: PriceConfig::new(
                    Percent::from_percent(50),
                    Duration::from_secs(60),
                    1,
                    Percent::from_percent(88),
                ),
            },
            value
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::SupportedCurrencyPairs {},
        )
        .unwrap();
        let value: Vec<SwapLeg> = cosmwasm_std::from_json(res).unwrap();

        let expected = vec![SwapLeg {
            from: PaymentC5::TICKER.into(),
            to: SwapTarget {
                pool_id: 1,
                target: StableC::TICKER.into(),
            },
        }];

        assert_eq!(expected, value);
    }

    #[test]
    fn impl_swap_path() {
        use crate::api::swap::QueryMsg as QueryMsgApi;

        let from = PaymentC1::TICKER;
        let to = StableC::TICKER;
        let query_impl = QueryMsg::SwapPath {
            from: from.into(),
            to: to.into(),
        };
        let query_api = cosmwasm_std::from_json::<QueryMsgApi>(
            &cosmwasm_std::to_json_vec(&query_impl).unwrap(),
        )
        .unwrap();
        assert_eq!(
            QueryMsgApi::SwapPath {
                from: from.into(),
                to: to.into(),
            },
            query_api
        );
    }

    #[test]
    fn impl_add_price_alarm() {
        use crate::api::alarms::ExecuteMsg as ExecuteMsgApi;

        let alarm = Alarm::<LeaseGroup, Lpns>::new(
            price::total_of::<LeaseC1>(10.into()).is::<StableC>(1.into()),
            Some(price::total_of(7.into()).is(1.into())),
        );
        let query_impl = ExecuteMsg::AddPriceAlarm {
            alarm: alarm.clone(),
        };
        let query_api = cosmwasm_std::from_json::<ExecuteMsgApi>(
            &cosmwasm_std::to_json_vec(&query_impl).unwrap(),
        )
        .unwrap();
        assert_eq!(ExecuteMsgApi::AddPriceAlarm { alarm }, query_api);
    }
}
