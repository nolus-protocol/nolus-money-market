use cosmwasm_std::{coins, Addr};
use cw_multi_test::ContractWrapper;
use finance::{liability::Liability, percent::Percent};
use lease::msg::{LoanForm, NewLeaseForm};

use cw_multi_test::{App, Executor};

use super::{ADMIN, USER};

pub struct LeaseWrapper {
    contract_wrapper: Box<
        ContractWrapper<
            lease::msg::ExecuteMsg,
            lease::msg::NewLeaseForm,
            lease::msg::StatusQuery,
            lease::error::ContractError,
            lease::error::ContractError,
            lease::error::ContractError,
        >,
    >,
}

impl LeaseWrapper {
    pub fn store(self, app: &mut App) -> u64 {
        app.store_code(self.contract_wrapper)
    }

    #[track_caller]
    pub fn instantiate(
        self,
        app: &mut App,
        code_id: Option<u64>,
        lpp_addr: &Addr,
        denom: &str,
    ) -> Addr {
        let code_id = match code_id {
            Some(id) => id,
            None => app.store_code(self.contract_wrapper),
        };
        let msg = Self::lease_instantiate_msg(denom, lpp_addr.clone());

        app.instantiate_contract(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &coins(1000, "UST"),
            "lease",
            None,
        )
        .unwrap()
    }

    fn lease_instantiate_msg(denom: &str, lpp_addr: Addr) -> NewLeaseForm {
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
}

impl Default for LeaseWrapper {
    fn default() -> Self {
        let contract = ContractWrapper::new(
            lease::contract::execute,
            lease::contract::instantiate,
            lease::contract::query,
        );

        Self {
            contract_wrapper: Box::new(contract),
        }
    }
}
