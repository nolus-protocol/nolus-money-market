use currency::{AnyVisitor, AnyVisitorResult, Currency, GroupVisit, Tickers};
use platform::{
    batch::{Emit, Emitter},
    response,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Storage,
        SubMsgResult,
    },
};
use versioning::{package_version, version, SemVer, Version, VersionSegment};

use crate::{
    api::{
        BaseCurrencyGroup, Config, ExecuteMsg, InstantiateMsg, MigrateMsg, PriceCurrencies,
        QueryMsg, SudoMsg,
    },
    contract::alarms::MarketAlarms,
    error::ContractError,
    result::ContractResult,
    state::supported_pairs::SupportedPairs,
};

use self::{
    config::query_config, exec::ExecWithOracleBase, oracle::feeder::Feeders,
    query::QueryWithOracleBase, sudo::SudoWithOracleBase,
};

mod alarms;
mod config;
pub mod exec;
mod oracle;
pub mod query;
mod sudo;

const CONTRACT_STORAGE_VERSION: VersionSegment = 0;
const PACKAGE_VERSION: SemVer = package_version!();
const CONTRACT_VERSION: Version = version!(CONTRACT_STORAGE_VERSION, PACKAGE_VERSION);

struct InstantiateWithCurrency<'a> {
    deps: DepsMut<'a>,
    msg: InstantiateMsg,
}

impl<'a> InstantiateWithCurrency<'a> {
    pub fn cmd(
        deps: DepsMut<'a>,
        msg: InstantiateMsg,
    ) -> ContractResult<<Self as AnyVisitor>::Output> {
        let context = Self { deps, msg };
        Tickers.visit_any::<BaseCurrencyGroup, _>(&context.msg.config.base_asset.clone(), context)
    }
}

impl<'a> AnyVisitor for InstantiateWithCurrency<'a> {
    type Output = ();
    type Error = ContractError;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency,
    {
        self.msg
            .config
            .store(self.deps.storage)
            .map_err(ContractError::StoreConfig)?;

        SupportedPairs::<C>::new(self.msg.swap_tree.into_tree())?
            .validate_tickers()?
            .save(self.deps.storage)?;

        Ok(())
    }
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    versioning::initialize(deps.storage, CONTRACT_VERSION)
        .map_err(ContractError::InitializeVersioning)?;

    InstantiateWithCurrency::cmd(deps, msg)?;

    Ok(response::empty_response())
}

#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    MigrateMsg {}: MigrateMsg,
) -> ContractResult<CwResponse> {
    versioning::update_software(
        deps.storage,
        CONTRACT_VERSION,
        ContractError::UpdateSoftware,
    )
    .and_then(response::response)
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::ContractVersion {} => {
            to_json_binary(&package_version!()).map_err(ContractError::ConvertToBinary)
        }
        QueryMsg::Config {} => {
            to_json_binary(&query_config(deps.storage)?).map_err(ContractError::ConvertToBinary)
        }
        QueryMsg::Feeders {} => Feeders::get(deps.storage)
            .map_err(ContractError::LoadFeeders)
            .and_then(|ref feeders| {
                to_json_binary(feeders).map_err(ContractError::ConvertToBinary)
            }),
        QueryMsg::IsFeeder { address } => Feeders::is_feeder(deps.storage, &address)
            .map_err(ContractError::LoadFeeders)
            .and_then(|ref f| to_json_binary(&f).map_err(ContractError::ConvertToBinary)),
        _ => QueryWithOracleBase::cmd(deps, env, msg),
    }
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    ExecWithOracleBase::cmd(deps, env, msg, info.sender)
}

#[entry_point]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    match msg {
        SudoMsg::UpdateConfig(price_config) => Config::update(deps.storage, price_config),
        SudoMsg::RegisterFeeder { feeder_address } => Feeders::try_register(deps, feeder_address),
        SudoMsg::RemoveFeeder { feeder_address } => Feeders::try_remove(deps, feeder_address),
        _ => SudoWithOracleBase::cmd(deps, msg),
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
            StableC::TICKER.to_string(),
            60,
            Percent::from_percent(50),
            swap_tree!({ base: StableC::TICKER }, (1, PaymentC5::TICKER)),
        );
        let (deps, _info) = setup_test(msg);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: Config = cosmwasm_std::from_json(res).unwrap();
        assert_eq!(
            Config {
                base_asset: StableC::TICKER.into(),
                price_config: PriceConfig::new(
                    Percent::from_percent(50),
                    Duration::from_secs(60),
                    1,
                    Percent::from_percent(88),
                )
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
                to: to.into()
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
