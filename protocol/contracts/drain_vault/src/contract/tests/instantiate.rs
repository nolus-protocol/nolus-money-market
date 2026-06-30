use sdk::{
    cosmwasm_std::{Deps, from_json, testing},
    testing as sdk_testing,
};

use crate::{
    api::{ConfigResponse, QueryMsg},
    contract::query,
};

use super::{OWNER, deps, instantiate_default};

fn query_config(deps: Deps<'_>) -> ConfigResponse {
    let raw =
        query(deps, testing::mock_env(), QueryMsg::Config()).expect("the config query succeeds");
    from_json(raw).expect("the config response deserializes")
}

#[test]
fn sweep_target_only_owner_configured() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());

    let config = query_config(deps.as_ref());
    assert_eq!(
        sdk_testing::user(OWNER),
        config.owner,
        "the stored owner is the configured profit address"
    );
}
