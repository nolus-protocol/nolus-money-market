use currencies::{Lpn, Lpns, PaymentGroup};
use sdk::{
    cosmwasm_std::{
        self,
        testing::{self as cw_testing},
        Binary, Deps, Env,
    },
    testing::manage_state,
};

use oracle::{
    api::{MigrateMsg, PricesResponse, QueryMsg},
    contract,
    result::ContractResult,
};

#[test]
fn migrate_oracle_v_1() {
    let mut deps = cw_testing::mock_dependencies();
    manage_state::try_load_into_storage_from_csv(&mut deps.storage, "data/oracle_v1.txt".as_ref())
        .expect("state load succeeded");

    let env = cw_testing::mock_env();

    let price_err = query_prices(deps.as_ref(), env.clone()).unwrap_err();
    assert!(matches!(
        price_err,
        oracle::ContractError::PriceFeedsError(_)
    ));

    let migration = contract::migrate(deps.as_mut(), env.clone(), MigrateMsg {}).unwrap();
    assert!(migration.messages.is_empty());

    let price: PricesResponse<PaymentGroup, Lpn, Lpns> =
        cosmwasm_std::from_json(query_prices(deps.as_ref(), env.clone()).unwrap()).unwrap();
    assert!(price.prices.is_empty());
}

fn query_prices(deps: Deps<'_>, env: Env) -> ContractResult<Binary> {
    contract::query(deps, env, QueryMsg::Prices {})
}
