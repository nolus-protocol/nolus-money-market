use cosmwasm_std::{to_binary, Addr, Binary, Deps, Env};
use cw_multi_test::Executor;

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
    ContractError,
};

use crate::common::{ContractWrapper, MockApp};

use super::{ADMIN, NATIVE_DENOM};

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
    pub fn instantiate(
        self,
        app: &mut MockApp,
        base_currency: &str,
        timealarms_addr: &str,
    ) -> Addr {
        let code_id = app.store_code(self.contract_wrapper);
        let msg = InstantiateMsg {
            base_asset: base_currency.to_string(),
            price_feed_period_secs: 60,
            expected_feeders: Percent::from_percent(1),
            currency_paths: vec![vec![NATIVE_DENOM.to_string(), Usdc::SYMBOL.to_string()]],
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
    cosmwasm_std::Empty,
    anyhow::Error,
    ContractError,
>;
