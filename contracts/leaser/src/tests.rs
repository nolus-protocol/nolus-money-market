use crate::contract::{execute, instantiate, query};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    coins, from_binary, to_binary, Addr, Coin, CosmosMsg, SubMsg, Uint256, WasmMsg,
};

use lease::msg::InstantiateMsg as LeaseInstantiateMsg;

use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        lease_code_id: 1,
        lpp_ust_addr: Addr::unchecked("test"),
        lease_interest_rate_margin: 3,
        lease_max_liability: 80,
        lease_healthy_liability: 70,
        lease_initial_liability: 65,
        repayment_period_nano_sec: Uint256::from(123_u64),
        grace_period_nano_sec: Uint256::from(123_u64),
        lease_minimal_downpayment: Some(Coin::new(10, "UST")),
    };
    let info = mock_info("creator", &coins(1000, "unolus"));

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&res).unwrap();
    let config = config_response.config;
    assert_eq!("creator", config.owner);
    assert_eq!(1, config.lease_code_id);
    assert_eq!(Addr::unchecked("test"), config.lpp_ust_addr);
}

#[test]
fn testexecute() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        lease_code_id: 1,
        lpp_ust_addr: Addr::unchecked("test"),
        lease_interest_rate_margin: 3,
        lease_max_liability: 80,
        lease_healthy_liability: 70,
        lease_initial_liability: 65,
        repayment_period_nano_sec: Uint256::from(123_u64),
        grace_period_nano_sec: Uint256::from(123_u64),
        lease_minimal_downpayment: Some(Coin::new(10, "UST")),
    };
    let info = mock_info("creator", &coins(1000, "unolus"));
    let _ = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // try open lease with not enought UST
    let msg = ExecuteMsg::Borrow {};
    let info = mock_info("addr0000", coins(40, "ETH").as_ref());
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(res.to_string(), "Insufficient funds for down payment");

    // try open lease with enought UST
    let msg = ExecuteMsg::Borrow {};
    let info = mock_info("addr0000", coins(40, "UST").as_ref());
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                funds: coins(40, "UST"),
                msg: to_binary(&LeaseInstantiateMsg {
                    owner: info.sender.to_string(),
                })
                .unwrap(),
                admin: None,
                code_id: 1,
                label: "lease".to_string()
            }),
            1
        )]
    );
}
