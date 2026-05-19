use sdk::cosmwasm_std::testing;
use versioning::{ProtocolPackageRelease, package_name, package_version};

use crate::{
    api::{ChannelStateResponse, QueryMsg},
    contract::query,
};

use super::{
    CONTRACT_STORAGE_VERSION, LOCAL_CHANNEL_ID, deps, instantiate_default, query_channel,
    store_open_channel,
};

#[test]
fn query_protocol_package_release_returns_current() {
    let deps = deps();
    let raw = query(
        deps.as_ref(),
        testing::mock_env(),
        QueryMsg::ProtocolPackageRelease {},
    )
    .unwrap();
    let parsed: ProtocolPackageRelease = sdk::cosmwasm_std::from_json(raw).unwrap();
    let expected = ProtocolPackageRelease::current(
        package_name!(),
        package_version!(),
        CONTRACT_STORAGE_VERSION,
    );
    assert_eq!(
        sdk::cosmwasm_std::to_json_binary(&expected).unwrap(),
        sdk::cosmwasm_std::to_json_binary(&parsed).unwrap(),
    );
}

#[test]
fn query_channel_returns_open_state_when_channel_is_open() {
    let mut deps = deps();
    instantiate_default(deps.as_mut());
    store_open_channel(deps.as_mut());

    let info = query_channel(deps.as_ref())
        .channel
        .expect("an open channel is recorded");
    assert!(matches!(info.state, ChannelStateResponse::Open));
    assert_eq!(LOCAL_CHANNEL_ID, info.local_channel_id);
}
