#[cfg(test)]
mod tests {
    use cosmwasm_std::{coins, Addr, Coin, Uint64};
    use cw_multi_test::{next_block, App, Executor};

    use crate::{
        msg::QueryMsg,
        tests::common::{
            mock_app, mock_dispatcher::instantiate_dispatcher, mock_lease::contract_lease_mock,
            mock_lpp::instantiate_lpp, mock_oracle::instantiate_oracle,
            mock_treasury::instantiate_treasury, ADMIN, USER,
        },
    };

    pub fn setup_test_case(
        app: &mut App,
        init_funds: Vec<Coin>,
        user_addr: Addr,
        denom: &str,
    ) -> (Addr, Addr, Addr) {
        let lease_id = app.store_code(contract_lease_mock());

        // 1. Instantiate LPP contract
        let (lpp_addr, _lpp_id) = instantiate_lpp(app, Uint64::new(lease_id), denom);
        app.update_block(next_block);

        // 2. Instantiate Treasury contract (and OWNER as admin)
        let treasury_addr = instantiate_treasury(app, denom);
        app.update_block(next_block);

        // 3. Instantiate Oracle contract (and OWNER as admin)
        let market_oracle = instantiate_oracle(app, denom);
        app.update_block(next_block);

        // 3. Instantiate Dispatcher contract
        let dispatcher_addr = instantiate_dispatcher(
            app,
            lpp_addr.clone(),
            Addr::unchecked("time"),
            treasury_addr.clone(),
            market_oracle,
        );
        app.update_block(next_block);

        // Bonus: set some funds on the user for future proposals
        if !init_funds.is_empty() {
            app.send_tokens(Addr::unchecked(ADMIN), user_addr, &init_funds)
                .unwrap();
        }
        (dispatcher_addr, treasury_addr, lpp_addr)
    }

    fn set_rewards_dispatcher(app: &mut App, treasury: Addr, dispatcher: Addr) {
        app.execute_contract(
            Addr::unchecked(ADMIN),
            treasury,
            &treasury::msg::ExecuteMsg::ConfigureRewardTransfer {
                rewards_dispatcher: dispatcher,
            },
            &[],
        )
        .unwrap();
    }

    #[test]
    fn on_alarm() {
        let denom = "UST";
        let mut app = mock_app(&coins(10000, denom));
        let time_oracle_addr = Addr::unchecked("time");

        let (dispatcher_addr, treasury_addr, lpp_addr) =
            setup_test_case(&mut app, coins(500, denom), time_oracle_addr.clone(), denom);

        set_rewards_dispatcher(&mut app, treasury_addr.clone(), dispatcher_addr.clone());

        let res = app
            .execute_contract(
                time_oracle_addr,
                dispatcher_addr.clone(),
                &crate::msg::ExecuteMsg::Alarm {
                    time: app.block_info().time,
                },
                &coins(40, denom),
            )
            .unwrap();

        // ensure the attributes were relayed from the sub-message
        assert_eq!(5, res.events.len(), "{:?}", res.events);
        // reflect only returns standard wasm-execute event
        let dispatcher_exec = &res.events[0];
        assert_eq!(dispatcher_exec.ty.as_str(), "execute");
        assert_eq!(
            dispatcher_exec.attributes,
            [("_contract_addr", &dispatcher_addr)]
        );
        let treasury_exec = &res.events[1];
        assert_eq!(treasury_exec.ty.as_str(), "execute");
        assert_eq!(
            treasury_exec.attributes,
            [("_contract_addr", &treasury_addr)]
        );
        let treasury_wasm = &res.events[2];
        assert_eq!(treasury_wasm.ty.as_str(), "wasm");
        assert_eq!(
            treasury_wasm.attributes,
            [
                ("_contract_addr", treasury_addr.to_string()),
                ("method", "try_send_rewards".to_string())
            ]
        );
        let treasury_wasm = &res.events[3];
        assert_eq!(treasury_wasm.ty.as_str(), "transfer");
        assert_eq!(
            treasury_wasm.attributes,
            [
                ("recipient", lpp_addr.to_string()),
                ("sender", treasury_addr.to_string()),
                ("amount", "32UST".to_string())
            ]
        );
        let treasury_exec = &res.events[4];
        assert_eq!(treasury_exec.ty.as_str(), "execute");
        assert_eq!(treasury_exec.attributes, [("_contract_addr", &lpp_addr)]);
    }

    #[test]
    fn test_config() {
        let denom = "UST";
        let mut app = mock_app(&coins(2000000, denom));
        let user_addr = Addr::unchecked(USER);
        let (dispatcher_addr, _, _) =
            setup_test_case(&mut app, coins(500, denom), user_addr, denom);

        let resp: crate::msg::ConfigResponse = app
            .wrap()
            .query_wasm_smart(dispatcher_addr, &QueryMsg::Config {})
            .unwrap();

        assert_eq!(10, resp.cadence_hours);
    }
}
