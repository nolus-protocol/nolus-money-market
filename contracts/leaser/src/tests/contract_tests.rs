use crate::contract::{execute, instantiate, query};
use crate::leaser::Leaser;
use crate::ContractError;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    coins, from_binary, to_binary, Addr, CosmosMsg, DepsMut, MessageInfo, StdError, SubMsg, Uint64,
    WasmMsg,
};
use finance::coin::{self, Coin};
use finance::currency::{Currency, Usdc};
use finance::liability::Liability;
use finance::percent::Percent;

use crate::msg::{ConfigResponse, ExecuteMsg, QueryMsg, QuoteResponse, Repayment};

const CREATOR: &str = "creator";
const LPP_ADDR: &str = "test";
type TheCurrency = Usdc;
const DENOM: &str = TheCurrency::SYMBOL;
const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(30);

fn leaser_instantiate_msg(lease_code_id: u64, lpp_addr: Addr) -> crate::msg::InstantiateMsg {
    crate::msg::InstantiateMsg {
        lease_code_id: Uint64::new(lease_code_id),
        lpp_ust_addr: lpp_addr,
        lease_interest_rate_margin: MARGIN_INTEREST_RATE,
        liability: Liability::new(
            Percent::from_percent(65),
            Percent::from_percent(70),
            Percent::from_percent(80),
            1,
        ),
        repayment: Repayment::new(90 * 24 * 60 * 60, 10 * 24 * 60 * 60),
    }
}

fn setup_test_case(deps: DepsMut) -> MessageInfo {
    let lpp_addr = Addr::unchecked(LPP_ADDR);
    let msg = leaser_instantiate_msg(1, lpp_addr);

    let info = mock_info(CREATOR, &coins(2, DENOM));
    let _res = instantiate(deps, mock_env(), info.clone(), msg).unwrap();
    info
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let lpp_addr = Addr::unchecked(LPP_ADDR);
    let msg = leaser_instantiate_msg(1, lpp_addr.clone());
    let info = mock_info(CREATOR, &coins(1000, DENOM));

    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&res).unwrap();
    let config = config_response.config;
    assert_eq!(CREATOR, config.owner);
    assert_eq!(1, config.lease_code_id);
    assert_eq!(lpp_addr, config.lpp_ust_addr);
}

#[test]
fn test_update_config() {
    let mut deps = mock_dependencies();
    let expected_liability = Liability::new(
        Percent::from_percent(55),
        Percent::from_percent(60),
        Percent::from_percent(65),
        12,
    );
    let expected_repaiment = Repayment::new(10, 10);
    let info = setup_test_case(deps.as_mut());
    let msg = ExecuteMsg::Config {
        lease_interest_rate_margin: Percent::from_percent(5),
        liability: expected_liability,
        repayment: expected_repaiment.clone(),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&res).unwrap();

    assert_eq!(expected_liability, config_response.config.liability);
    assert_eq!(expected_repaiment, config_response.config.repayment);
}

#[test]
fn test_update_config_unauthorized() {
    let mut deps = mock_dependencies();
    let expected_liability = Liability::new(
        Percent::from_percent(55),
        Percent::from_percent(60),
        Percent::from_percent(65),
        12,
    );
    let expected_repaiment = Repayment::new(10, 10);
    setup_test_case(deps.as_mut());
    let msg = ExecuteMsg::Config {
        lease_interest_rate_margin: Percent::from_percent(5),
        liability: expected_liability,
        repayment: expected_repaiment,
    };

    let info = mock_info("addr0000", coins(40, DENOM).as_ref());
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err);
}

#[test]
fn test_open_lease() {
    let mut deps = mock_dependencies();
    setup_test_case(deps.as_mut());

    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&res).unwrap();
    let config = config_response.config;

    // try open lease with enought UST
    let msg = ExecuteMsg::OpenLease {
        currency: DENOM.to_string(),
    };
    let info = mock_info("addr0000", coins(40, DENOM).as_ref());
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = Leaser::open_lease_msg(info.sender, config, DENOM.to_string());
    assert_eq!(
        res.messages,
        vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                funds: coins(40, DENOM),
                msg: to_binary(&msg).unwrap(),
                admin: None,
                code_id: 1,
                label: "lease".to_string()
            }),
            1
        )]
    );
}

#[test]
fn test_quote() {
    let mut deps = mock_dependencies();
    setup_test_case(deps.as_mut());

    // should fail if zero downpaynment
    let err = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Quote {
            downpayment: coin::funds::<TheCurrency>(0),
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("cannot open lease with zero downpayment",).into()
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Quote {
            downpayment: coin::funds::<TheCurrency>(100),
        },
    )
    .unwrap();
    let resp: QuoteResponse<TheCurrency, TheCurrency> = from_binary(&res).unwrap();

    assert_eq!(Coin::<TheCurrency>::new(185), resp.borrow);
    assert_eq!(Coin::<TheCurrency>::new(285), resp.total);
    // TODO: move to finance::coin::Coin
    // assert_eq!(finance::coin::Coin::<Usdc>::new(185), resp.borrow);
    // assert_eq!(finance::coin::Coin::<Usdc>::new(285), resp.total);
    /*
        103% =
        100% lpp annual_interest_rate (when calling the test version of get_annual_interest_rate() in lpp_querier.rs)
        +
        3% margin_interest_rate of the leaser
    */
    assert_eq!(
        Percent::HUNDRED.checked_add(MARGIN_INTEREST_RATE).unwrap(),
        resp.annual_interest_rate
    ); // hardcoded until LPP contract is merged

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Quote {
            downpayment: coin::funds::<TheCurrency>(15),
        },
    )
    .unwrap();
    let resp: QuoteResponse<TheCurrency, TheCurrency> = from_binary(&res).unwrap();

    assert_eq!(Coin::<TheCurrency>::new(27), resp.borrow);
    assert_eq!(Coin::<TheCurrency>::new(42), resp.total);

    // TODO: move to finance::coin
    // assert_eq!(finance::coin::Coin::<Usdc>::new(27), resp.borrow);
    // assert_eq!(finance::coin::Coin::<Usdc>::new(42), resp.total);
}
