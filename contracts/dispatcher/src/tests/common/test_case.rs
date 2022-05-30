use anyhow::Error;
use cosmwasm_std::{coins, Addr, Coin, Empty, StdError, Uint64};
use cw_multi_test::{next_block, App, ContractWrapper, Executor};

use super::{
    mock_app, mock_dispatcher::MockDispatcher, mock_lease::contract_lease_mock, mock_lpp::MockLpp,
    mock_oracle::MockMarketOracle, mock_treasury::MockTreasury, ADMIN,
};

const STABLECOIN: &str = "UST";

pub struct TestCase {
    pub app: App,
    pub dispatcher_addr: Option<Addr>,
    pub treasury_addr: Option<Addr>,
    pub lpp_addr: Option<Addr>,
    pub market_oracle: Option<Addr>,
    pub time_oracle: Option<Addr>,
    lease_code_id: Option<u64>,
    denom: String,
}

impl TestCase {
    pub fn new(denom: &str) -> Self {
        Self {
            app: mock_app(&coins(10000, denom)),
            dispatcher_addr: None,
            treasury_addr: None,
            lpp_addr: None,
            market_oracle: None,
            time_oracle: None,
            lease_code_id: None,
            denom: denom.to_string(),
        }
    }
    pub fn init(&mut self, user_addr: &Addr, init_funds: Vec<Coin>) -> &mut Self {
        self.lease_code_id = Some(self.app.store_code(contract_lease_mock()));
        // Bonus: set some funds on the user for future proposals
        if !init_funds.is_empty() {
            self.app
                .send_tokens(Addr::unchecked(ADMIN), user_addr.clone(), &init_funds)
                .unwrap();
        }

        self
    }
    pub fn init_lpp(
        &mut self,
        custom_wrapper: Option<
            ContractWrapper<
                lpp::msg::ExecuteMsg,
                lpp::msg::InstantiateMsg,
                lpp::msg::QueryMsg,
                lpp::error::ContractError,
                lpp::error::ContractError,
                lpp::error::ContractError,
                Empty,
                Empty,
                Empty,
                Error,
                Error,
                Empty,
                Error,
            >,
        >,
    ) -> &mut Self {
        let mocked_lpp = match custom_wrapper {
            Some(wrapper) => MockLpp::with_contract_wrapper(wrapper),
            None => MockLpp::default(),
        };
        self.lpp_addr = Some(
            mocked_lpp
                .instantiate(
                    &mut self.app,
                    Uint64::new(self.lease_code_id.unwrap()),
                    &self.denom,
                )
                .0,
        );
        self.app.update_block(next_block);
        self
    }

    pub fn init_treasury(&mut self) -> &mut Self {
        self.treasury_addr = Some(MockTreasury::default().instantiate(&mut self.app, &self.denom));
        self.app.update_block(next_block);

        self
    }

    pub fn init_market_oracle(
        &mut self,
        custom_wrapper: Option<
            ContractWrapper<
                oracle::msg::ExecuteMsg,
                oracle::msg::InstantiateMsg,
                oracle::msg::QueryMsg,
                oracle::ContractError,
                oracle::ContractError,
                StdError,
                Empty,
                Empty,
                Empty,
                Error,
                Error,
                Empty,
                Error,
            >,
        >,
    ) -> &mut Self {
        let mocked_oracle = match custom_wrapper {
            Some(wrapper) => MockMarketOracle::with_contract_wrapper(wrapper),
            None => MockMarketOracle::default(),
        };

        self.market_oracle = Some(mocked_oracle.instantiate(&mut self.app, &self.denom));
        self.app.update_block(next_block);

        self
    }

    pub fn init_time_oracle(&mut self) -> &mut Self {
        self.time_oracle = Some(Addr::unchecked("time"));
        self
    }
    pub fn init_dispatcher(&mut self) -> &mut Self {
        // Instantiate Dispatcher contract
        let dispatcher_addr = MockDispatcher::default().instantiate(
            &mut self.app,
            self.lpp_addr.as_ref().unwrap(),
            self.time_oracle.as_ref().unwrap(),
            &self.treasury_addr.as_ref().unwrap().clone(),
            self.market_oracle.as_ref().unwrap(),
            &self.denom,
        );
        self.app.update_block(next_block);

        self.app
            .execute_contract(
                Addr::unchecked(ADMIN),
                self.treasury_addr.to_owned().unwrap(),
                &treasury::msg::ExecuteMsg::ConfigureRewardTransfer {
                    rewards_dispatcher: dispatcher_addr.clone(),
                },
                &[],
            )
            .unwrap();

        self.dispatcher_addr = Some(dispatcher_addr);
        self
    }
}
