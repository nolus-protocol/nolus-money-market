use cosmwasm_std::Addr;
use cw_multi_test::ContractWrapper;
use finance::{liability::Liability, percent::Percent};
use lease::msg::{LoanForm, NewLeaseForm};

use cosmwasm_std::{coins, Empty};
use cw_multi_test::{App, Contract, Executor};

use super::{ADMIN, USER};

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

pub fn contract_lease_mock() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        lease::contract::execute,
        lease::contract::instantiate,
        lease::contract::query,
    )
    .with_reply(lease::contract::reply);
    Box::new(contract)
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
