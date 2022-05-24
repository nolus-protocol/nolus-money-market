use cosmwasm_std::Addr;
use cw_multi_test::ContractWrapper;
use finance::{liability::Liability, percent::Percent};
use lease::msg::{LoanForm, NewLeaseForm};

use crate::{
    msg::QueryMsg,
    state::tvl_intervals::{Intervals, Stop},
};
use cosmwasm_std::{coins, to_binary, Binary, Coin, Deps, Empty, Env, StdResult};
use cw_multi_test::{App, AppBuilder, Contract, Executor};
use serde::Serialize;

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";

pub fn dispatcher_instantiate_msg(
    lpp: Addr,
    time_oracle: Addr,
    treasury: Addr,
    market_oracle: Addr,
) -> crate::msg::InstantiateMsg {
    crate::msg::InstantiateMsg {
        cadence_hours: 10,
        lpp,
        time_oracle,
        treasury,
        market_oracle,
        tvl_to_apr: Intervals::from(vec![Stop::new(0, 5), Stop::new(1000000, 10)]).unwrap(),
    }
}

pub fn lease_instantiate_msg(denom: &str, lpp_addr: Addr) -> NewLeaseForm {
    NewLeaseForm {
        customer: USER.to_string(),
        currency: denom.to_string(),
        liability: Liability::new(
            Percent::from_percent(65),
            Percent::from_percent(5),
            Percent::from_percent(10),
            20 * 24,
        ),
        loan: LoanForm {
            annual_margin_interest: Percent::from_percent(0), // 3.1%
            lpp: lpp_addr.into_string(),
            interest_due_period_secs: 100, // 90 days TODO use a crate for daytime calculations
            grace_period_secs: 10,         // 10 days TODO use a crate for daytime calculations
        },
    }
}

pub fn treasury_instantiate_msg() -> treasury::msg::InstantiateMsg {
    treasury::msg::InstantiateMsg {}
}

pub fn contract_lease_mock() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        lease::contract::execute,
        lease::contract::instantiate,
        lease::contract::query,
    )
    .with_reply(lease::contract::reply);
    Box::new(contract)
}

pub fn contract_dispatcher_mock() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

#[derive(Serialize, Clone, Debug, PartialEq)]
struct MockResponse {
    pub ok: bool,
}

fn mock_treasury_query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    to_binary(&MockResponse { ok: true })
}

pub fn contract_treasury_mock() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        treasury::contract::execute,
        treasury::contract::instantiate,
        mock_treasury_query,
    );
    Box::new(contract)
}

pub fn mock_app(init_funds: &[Coin]) -> App {
    AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &Addr::unchecked(ADMIN), init_funds.to_vec())
            .unwrap();
    })
}

#[track_caller]
pub fn instantiate_dispatcher(
    app: &mut App,
    lpp_addr: Addr,
    time_oracle: Addr,
    treasury: Addr,
    market_oracle: Addr,
) -> Addr {
    let code_id = app.store_code(contract_dispatcher_mock());
    let msg = dispatcher_instantiate_msg(lpp_addr, time_oracle, treasury, market_oracle);
    app.instantiate_contract(
        code_id,
        Addr::unchecked(ADMIN),
        &msg,
        &[],
        "dispatcher",
        None,
    )
    .unwrap()
}

#[track_caller]
pub fn instantiate_lease(app: &mut App, lease_id: u64, lpp_addr: Addr, denom: &str) -> Addr {
    let msg = lease_instantiate_msg(denom, lpp_addr);

    app.instantiate_contract(
        lease_id,
        Addr::unchecked(ADMIN),
        &msg,
        &coins(400, denom),
        "lease",
        None,
    )
    .unwrap()
}

#[track_caller]
pub fn instantiate_treasury(app: &mut App, denom: &str) -> Addr {
    let code_id = app.store_code(contract_treasury_mock());
    let msg = treasury_instantiate_msg();

    app.instantiate_contract(
        code_id,
        Addr::unchecked(ADMIN),
        &msg,
        &coins(1000, denom),
        "treasury",
        None,
    )
    .unwrap()
}
