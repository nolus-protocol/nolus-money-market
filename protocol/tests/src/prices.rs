use std::time::{SystemTime, UNIX_EPOCH};

use sdk::{
    cosmwasm_std::{
        self,
        testing::{self as cw_testing},
        Env, Timestamp,
    },
    testing::manage_state,
};

use oracle::{api::QueryMsg, contract};

/// Simulates a case after migration to a break-changed serialization.
///
/// In particular, u128 was serialized as a Serde number by Money Market with CW 1.x.
/// Only the JSON (de-)serializer, though, was crafted to (de-)serialize (from)into JSON String.
/// The Postcard (de-)serializer was doing (de-)serialize (from)into Postcard number.
///
/// Migrating to CW 2.x, and motivated to not break the API, we shifted to serializing u128 as a Serde string.
/// That broke the existing Postcard serialized data.
#[test]
fn query_prices_from_production_state() {
    let mut deps = cw_testing::mock_dependencies();

    manage_state::try_load_into_storage_from_csv(
        &mut deps.storage,
        "data/dev-osmosis-oracle.txt".as_ref(),
    )
    .expect("state load succeeded");

    let env = set_current_time(cw_testing::mock_env());

    let currencies: Vec<oracle::api::Currency> = cosmwasm_std::from_json(
        contract::query(deps.as_ref(), env.clone(), QueryMsg::Currencies {}).unwrap(),
    )
    .unwrap();
    assert_eq!(3, currencies.len());

    let price_err = contract::query(deps.as_ref(), env.clone(), QueryMsg::Prices {}).unwrap_err();
    assert!(matches!(
        price_err,
        oracle::ContractError::PriceFeedsError(_)
    ));
}

// TODO put the time next to the moment the data were exported
fn set_current_time(mut env: Env) -> Env {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    env.block.time = Timestamp::from_nanos(
        since_the_epoch
            .as_nanos()
            .try_into()
            .expect("The time has gone too far in the tuture!"),
    );
    env
}
