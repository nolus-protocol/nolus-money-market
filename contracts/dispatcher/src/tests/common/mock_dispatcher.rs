use cosmwasm_std::Addr;
use cw_multi_test::ContractWrapper;

use crate::state::tvl_intervals::{Intervals, Stop};
use cosmwasm_std::Empty;
use cw_multi_test::{App, Contract, Executor};

use super::ADMIN;

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

pub fn contract_dispatcher_mock() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
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
