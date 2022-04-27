#[cfg(test)]
mod tests {
    use cosmwasm_std::{coins, Addr, Coin, Empty, Uint256};
    use cw_multi_test::{next_block, App, AppBuilder, Contract, ContractWrapper, Executor};

    const USER: &str = "USER";
    const ADMIN: &str = "ADMIN";

    pub fn contract_leaser_mock() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply(crate::contract::reply);
        Box::new(contract)
    }

    pub fn contract_lease_mock() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            lease::contract::execute,
            lease::contract::instantiate,
            lease::contract::query,
        );
        Box::new(contract)
    }

    fn mock_app(init_funds: &[Coin]) -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked(ADMIN), init_funds.to_vec())
                .unwrap();
        })
    }

    pub fn leaser_instantiate_msg(lease_code_id: u64) -> crate::msg::InstantiateMsg {
        crate::msg::InstantiateMsg {
            lease_code_id,
            lpp_ust_addr: Addr::unchecked("test"),
            lease_interest_rate_margin: 3,
            lease_max_liability: 80,
            lease_healthy_liability: 70,
            lease_initial_liability: 65,
            repayment_period_nano_sec: Uint256::from(123_u64),
            grace_period_nano_sec: Uint256::from(123_u64),
            lease_minimal_downpayment: Some(Coin::new(10, "UST")),
        }
    }

    // uploads code and returns address of group contract
    #[track_caller]
    fn instantiate_leaser(app: &mut App, lease_code_id: u64) -> Addr {
        let leaser_id = app.store_code(contract_leaser_mock());
        let msg = leaser_instantiate_msg(lease_code_id);
        app.instantiate_contract(leaser_id, Addr::unchecked(ADMIN), &msg, &[], "leaser", None)
            .unwrap()
    }

    // uploads code and returns address of group contract
    #[track_caller]
    fn instantiate_lease(app: &mut App) -> (Addr, u64) {
        let lease_id = app.store_code(contract_lease_mock());
        let msg = lease::msg::InstantiateMsg {
            owner: ADMIN.to_string(),
        };
        (
            app.instantiate_contract(lease_id, Addr::unchecked(ADMIN), &msg, &[], "lease", None)
                .unwrap(),
            lease_id,
        )
    }

    fn setup_test_case(app: &mut App, init_funds: Vec<Coin>, user_addr: Addr) -> (Addr, u64) {
        // 1. Instantiate Lease contract (and OWNER as admin)
        let (_lease_addr, lease_code_id) = instantiate_lease(app);
        app.update_block(next_block);

        // 2. Instantiate Leaser contract
        let leaser_addr = instantiate_leaser(app, lease_code_id);
        app.update_block(next_block);

        // Bonus: set some funds on the user for future proposals
        if !init_funds.is_empty() {
            app.send_tokens(Addr::unchecked(ADMIN), user_addr, &init_funds)
                .unwrap();
        }
        (leaser_addr, lease_code_id)
    }

    #[test]
    fn open_lease() {
        let mut app = mock_app(&coins(10000, "UST"));
        let user_addr = Addr::unchecked(USER);

        let (leaser_addr, lease_code_id) =
            setup_test_case(&mut app, coins(500, "UST"), user_addr.clone());

        assert_eq!(
            coins(500, "UST"),
            app.wrap().query_all_balances(user_addr.clone()).unwrap()
        );

        let res = app
            .execute_contract(
                user_addr.clone(),
                leaser_addr.clone(),
                &crate::msg::ExecuteMsg::Borrow {},
                &coins(40, "UST"),
            )
            .unwrap();

        // ensure the attributes were relayed from the sub-message
        assert_eq!(4, res.events.len(), "{:?}", res.events);
        // reflect only returns standard wasm-execute event
        let leaser_exec = &res.events[0];
        assert_eq!(leaser_exec.ty.as_str(), "execute");
        assert_eq!(leaser_exec.attributes, [("_contract_addr", &leaser_addr)]);

        let lease_exec = &res.events[1];
        assert_eq!(lease_exec.ty.as_str(), "instantiate");
        assert_eq!(
            lease_exec.attributes,
            [
                ("_contract_addr", "Contract #2"),
                ("code_id", &lease_code_id.to_string())
            ]
        );

        let lease_reply = &res.events[2];
        assert_eq!(lease_reply.ty.as_str(), "reply");
        assert_eq!(
            lease_reply.attributes,
            [
                ("_contract_addr", leaser_addr.as_str()),
                ("mode", "handle_success")
            ]
        );

        let lease_reply = &res.events[3];
        assert_eq!(lease_reply.ty.as_str(), "wasm");
        assert_eq!(
            lease_reply.attributes,
            [
                ("_contract_addr", leaser_addr.as_str()),
                ("lease_address", "Contract #2")
            ]
        );

        let lease_address = &res.events[3].attributes.get(1).unwrap().value;

        assert_eq!(
            coins(460, "UST"),
            app.wrap().query_all_balances(user_addr).unwrap()
        );
        assert_eq!(
            coins(40, "UST"),
            app.wrap().query_all_balances(lease_address).unwrap()
        );
    }
}
