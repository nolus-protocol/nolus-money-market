use crate::contract::{execute, instantiate, query};
use crate::helpers::open_lease_msg;
use crate::tests::common::leaser_instantiate_msg;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    coins, from_binary, to_binary, Addr, Coin, CosmosMsg, Decimal, DepsMut, StdError, SubMsg,
    Uint128, WasmMsg,
};

use crate::msg::{ConfigResponse, ExecuteMsg, QueryMsg, QuoteResponse};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let lpp_addr = Addr::unchecked("test");
    let msg = leaser_instantiate_msg(1, lpp_addr.clone());
    let info = mock_info("creator", &coins(1000, "unolus"));

    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&res).unwrap();
    let config = config_response.config;
    assert_eq!("creator", config.owner);
    assert_eq!(1, config.lease_code_id);
    assert_eq!(lpp_addr, config.lpp_ust_addr);
}

#[test]
fn testexecute() {
    let mut deps = mock_dependencies();

    let lpp_addr = Addr::unchecked("test");
    let msg = leaser_instantiate_msg(1, lpp_addr);
    let info = mock_info("creator", &coins(1000, "unolus"));
    let _ = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&res).unwrap();
    let config = config_response.config;

    // try open lease with enought UST
    let msg = ExecuteMsg::Borrow {};
    let info = mock_info("addr0000", coins(40, "UST").as_ref());
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = open_lease_msg(info.sender, config);
    assert_eq!(
        res.messages,
        vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                funds: coins(40, "UST"),
                msg: to_binary(&msg).unwrap(),
                admin: None,
                code_id: 1,
                label: "lease".to_string()
            }),
            1
        )]
    );
}

fn setup_test_case(deps: DepsMut) {
    let lpp_addr = Addr::unchecked("test");
    let msg = leaser_instantiate_msg(1, lpp_addr);

    let info = mock_info("creator", &coins(2, "token"));
    let _res = instantiate(deps, mock_env(), info, msg).unwrap();
}

#[test]
fn quote_test() {
    let mut deps = mock_dependencies();
    setup_test_case(deps.as_mut());

    // should fail if zero downpaynment
    let err = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Quote {
            downpayment: Coin::new(0, "UST"),
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
            downpayment: Coin::new(100, "UST"),
        },
    )
    .unwrap();
    let resp: QuoteResponse = from_binary(&res).unwrap();

    assert_eq!(Uint128::new(185), resp.borrow.amount);
    assert_eq!(Uint128::new(285), resp.total.amount);
    assert_eq!("UST", resp.borrow.denom);
    assert_eq!("UST", resp.total.denom);
    assert_eq!(Decimal::one(), resp.annual_interest_rate); // hardcoded until LPP contract is merged

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Quote {
            downpayment: Coin::new(15, "UST"),
        },
    )
    .unwrap();
    let resp: QuoteResponse = from_binary(&res).unwrap();

    assert_eq!(Uint128::new(27), resp.borrow.amount);
    assert_eq!(Uint128::new(42), resp.total.amount);
}
