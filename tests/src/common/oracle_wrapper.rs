use currency::{lpn::Usdc, native::Nls};
use finance::{
    coin::Coin,
    currency::Currency,
    percent::Percent,
    price::{dto::PriceDTO, total_of},
};
use oracle::{
    contract::{execute, instantiate, query, reply},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::supported_pairs::TreeStore,
    ContractError,
};
use sdk::{
    cosmwasm_std::{to_binary, Addr, Binary, Deps, Empty, Env},
    cw_multi_test::Executor,
};
use trees::tr;

use crate::common::{ContractWrapper, MockApp};

use super::{Native, ADMIN};

pub struct MarketOracleWrapper {
    contract_wrapper: Box<OracleContractWrapper>,
}

impl MarketOracleWrapper {
    pub fn with_contract_wrapper(contract: OracleContractWrapper) -> Self {
        Self {
            contract_wrapper: Box::new(contract),
        }
    }
    #[track_caller]
    pub fn instantiate<BaseC>(self, app: &mut MockApp, timealarms_addr: &str) -> Addr
    where
        BaseC: Currency,
    {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {
            base_asset: BaseC::TICKER.into(),
            price_feed_period_secs: 60,
            expected_feeders: Percent::from_percent(1),
            swap_tree: TreeStore(tr((0, Usdc::TICKER.into())) / tr((1, Native::TICKER.to_string()))),
            timealarms_addr: timealarms_addr.to_string(),
        };

        app.instantiate_contract(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &Vec::default(),
            "oracle",
            None,
        )
        .unwrap()
    }
}

impl Default for MarketOracleWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(execute, instantiate, query).with_reply(reply);

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}

pub fn mock_oracle_query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let price = total_of(Coin::<Nls>::new(123456789)).is(Coin::<Usdc>::new(100000000));
    let res = match msg {
        QueryMsg::Prices { currencies: _ } => to_binary(&oracle::msg::PricesResponse {
            prices: vec![price.into()],
        }),
        QueryMsg::Price { currency: _ } => to_binary(&PriceDTO::from(price)),
        _ => Ok(query(deps, env, msg)?),
    }?;

    Ok(res)
}

type OracleContractWrapper = ContractWrapper<
    ExecuteMsg,
    ContractError,
    InstantiateMsg,
    ContractError,
    QueryMsg,
    ContractError,
    Empty,
    anyhow::Error,
    ContractError,
>;
