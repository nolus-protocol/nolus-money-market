#[cfg(test)]
mod tests {
    use crate::helpers::CwTemplateContract;
    use crate::msg::InstantiateMsg;
    use cosmwasm_std::{Addr, Coin, Empty, Uint128, Uint256};
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

    pub fn contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    const USER: &str = "USER";
    const ADMIN: &str = "ADMIN";
    const NATIVE_DENOM: &str = "denom";

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(USER),
                    vec![Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100000),
                    }],
                )
                .unwrap();
        })
    }

    fn proper_instantiate() -> (App, CwTemplateContract) {
        let mut app = mock_app();
        let cw_template_id = app.store_code(contract_template());

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
        let cw_template_contract_addr = app
            .instantiate_contract(
                cw_template_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "test",
                None,
            )
            .unwrap();

        let cw_template_contract = CwTemplateContract(cw_template_contract_addr);

        (app, cw_template_contract)
    }

    mod config {
        use super::*;

        #[test]
        fn config() {
            let (app, cw_template_contract) = proper_instantiate();

            let response = cw_template_contract.config(&app).unwrap();
            assert_eq!(Addr::unchecked("test"), response.config.lpp_ust_addr)
        }
    }

    // mod lease {
    //     use crate::msg::ExecuteMsg;

    //     use super::*;

    //     #[test]
    //     fn open_lease() {
    //         let (mut app, cw_template_contract) = proper_instantiate();

    //         // send without funds
    //         let msg = ExecuteMsg::Borrow {};
    //         let cosmos_msg = cw_template_contract.call(msg, None).unwrap();
    //         app.execute(Addr::unchecked(USER), cosmos_msg).unwrap_err();

    //         // send not enought funds
    //         let msg = ExecuteMsg::Borrow {};
    //         let cosmos_msg = cw_template_contract
    //             .call(msg, Some(vec![Coin::new(123_u128, "ETH")]))
    //             .unwrap();
    //         app.execute(Addr::unchecked(USER), cosmos_msg).unwrap();

    //         // send not enought funds - only UST are accepted
    //         let msg = ExecuteMsg::Borrow {};
    //         let cosmos_msg = cw_template_contract
    //             .call(msg, Some(vec![Coin::new(1_u128, "ETH")]))
    //             .unwrap();
    //         app.execute(Addr::unchecked(USER), cosmos_msg).unwrap();

    //         // // send with funds
    //         // let msg = ExecuteMsg::Borrow {};
    //         // let cosmos_msg = cw_template_contract
    //         //     .call(msg, Some(vec![Coin::new(123_u128, "UST")]))
    //         //     .unwrap();
    //         // app.execute(Addr::unchecked(USER), cosmos_msg).unwrap();
    //     }
    // }
}
