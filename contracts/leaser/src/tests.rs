use crate::contract::{execute, instantiate, query};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    coins, from_binary, to_binary, Addr, Coin, CosmosMsg, Decimal, StdError, SubMsg, Uint128,
    Uint256, WasmMsg,
};

use lease::msg::InstantiateMsg as LeaseInstantiateMsg;

use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, QuoteResponse};

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

    // try open lease with nothing
    let msg = ExecuteMsg::Borrow {};
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res.to_string(), "Insufficient funds for down payment");

    // try open lease with not enought UST
    let msg = ExecuteMsg::Borrow {};
    let mut amount = coins(40, "ETH");
    amount.append(&mut coins(2, "UST"));
    let info = mock_info("addr0000", coins(40, "ETH").as_ref());
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res.to_string(), "Insufficient funds for down payment");

    // try open lease with no UST
    let msg = ExecuteMsg::Borrow {};
    let info = mock_info("addr0000", coins(40, "ETH").as_ref());
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res.to_string(), "Insufficient funds for down payment");

    // try open lease with enought UST
    let msg = ExecuteMsg::Borrow {};
    let info = mock_info("addr0000", coins(40, "UST").as_ref());
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
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

#[test]
fn quote_test() {
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
    let info = mock_info("creator", &coins(2, "token"));
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // should fail if zero downpaynment
    let err = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Quote {
            downpayment: Uint128::zero(),
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("cannot open lease with zero downpayment",)
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Quote {
            downpayment: Uint128::new(100),
        },
    )
    .unwrap();
    let resp: QuoteResponse = from_binary(&res).unwrap();

    assert_eq!(Uint128::new(185), resp.borrow_ust);
    assert_eq!(Uint128::new(285), resp.total_ust);
    assert_eq!(Decimal::one(), resp.annual_interest_rate); // hardcoded until LPP contract is merged
}
