use currencies::{Lpn, Lpns, PaymentGroup};
use sdk::{
    cosmwasm_std::{
        self, Binary, Deps, Env, Storage, Timestamp,
        testing::{self as cw_testing},
    },
    testing::manage_state,
};

use oracle::{
    api::{MigrateMsg, PricesResponse, QueryMsg},
    contract,
    result::ContractResult,
};

/// This integration test is intended to be run only against the `osmosis-osmosis-usdc_axelar` protocol.
///
/// Pass `--features="oracle/contract sdk/testing"` to `cargo test` to enable test execution.
#[test]
fn migrate_oracle() {
    let mut deps = cw_testing::mock_dependencies();
    import_state(&mut deps.storage);

    let env = cw_testing::mock_env();

    let price: PricesResponse<PaymentGroup, Lpn, Lpns> =
        cosmwasm_std::from_json(query_prices_int(deps.as_ref(), env.clone()).unwrap()).unwrap();
    assert!(price.prices.is_empty());

    let migration_err = contract::migrate(deps.as_mut(), env.clone(), MigrateMsg {}).unwrap_err();
    assert!(matches!(
        migration_err,
        oracle::ContractError::UpdateSoftware(_)
    ));
}

/// Simulates a case after migration to a break-changed serialization.
///
/// In particular, u128 was serialized as a Serde number by Money Market with CW 1.x.
/// Only the JSON (de-)serializer, though, was crafted to (de-)serialize (from)into JSON String.
/// The Postcard (de-)serializer was doing (de-)serialize (from)into Postcard number.
///
/// Migrating to CW 2.x, and motivated to not break the API, we shifted to serializing u128 as a Serde string.
/// That broke the existing Postcard serialized data.
#[test]
fn query_prices() {
    let mut deps = cw_testing::mock_dependencies();

    manage_state::try_load_into_storage_from_csv(&mut deps.storage, "data/oracle_v2.txt".as_ref())
        .expect("state load succeeded");

    let env = set_export_date_time(cw_testing::mock_env());

    let currencies: Vec<oracle::api::Currency> = cosmwasm_std::from_json(
        contract::query(deps.as_ref(), env.clone(), QueryMsg::Currencies {}).unwrap(),
    )
    .unwrap();
    assert_eq!(3, currencies.len());

    let price: PricesResponse<PaymentGroup, Lpn, Lpns> =
        cosmwasm_std::from_json(query_prices_int(deps.as_ref(), env.clone()).unwrap()).unwrap();
    assert_eq!(2, price.prices.len());
}

fn import_state(store: &mut dyn Storage) {
    manage_state::try_load_into_storage_from_csv(store, "data/oracle_v2.txt".as_ref())
        .expect("state load succeeded");
}

fn set_export_date_time(mut env: Env) -> Env {
    env.block.time = Timestamp::from_seconds(1729074092); // obtained with `date +%s` right after
    env
}

fn query_prices_int(deps: Deps<'_>, env: Env) -> ContractResult<Binary> {
    contract::query(deps, env, QueryMsg::Prices {})
}
