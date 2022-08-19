use anyhow::Error;
use cosmwasm_std::{coins, Addr, Coin, Empty, StdError, Uint64};
use cw_multi_test::{next_block, App, ContractWrapper, Executor};
use finance::coin::Amount;

use super::{
    dispatcher_wrapper::DispatcherWrapper, lease_wrapper::{LeaseWrapper, LeaseWrapperConfig},
    leaser_wrapper::LeaserWrapper, lpp_wrapper::LppWrapper, mock_app,
    oracle_wrapper::MarketOracleWrapper, profit_wrapper::ProfitWrapper,
    timealarms_wrapper::TimeAlarmsWrapper, treasury_wrapper::TreasuryWrapper, ADMIN,
};

type OptionalContractWrapper = Option<
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
>;

type OptionalContractWrapperStd = Option<
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
>;

pub struct TestCase {
    pub app: App,
    pub dispatcher_addr: Option<Addr>,
    pub treasury_addr: Option<Addr>,
    pub profit_addr: Option<Addr>,
    pub leaser_addr: Option<Addr>,
    pub lpp_addr: Option<Addr>,
    pub oracle: Option<Addr>,
    pub timealarms: Option<Addr>,
    pub lease_code_id: Option<u64>,
    denom: String,
}

impl TestCase {
    pub fn new(denom: &str) -> Self {
        Self::with_reserve(denom, 10_000)
    }

    pub fn with_reserve(denom: &str, reserve: Amount) -> Self {
        Self {
            app: mock_app(&coins(reserve, denom)),
            dispatcher_addr: None,
            treasury_addr: None,
            profit_addr: None,
            leaser_addr: None,
            lpp_addr: None,
            oracle: None,
            timealarms: None,
            lease_code_id: None,
            denom: denom.to_string(),
        }
    }

    pub fn send_funds(&mut self, user_addr: &Addr, funds: Vec<Coin>) -> &mut Self {
        self.app
            .send_tokens(Addr::unchecked(ADMIN), user_addr.clone(), &funds)
            .unwrap();

        self
    }

    pub fn init(&mut self, user: &Addr, init_funds: Vec<Coin>) -> &mut Self {
        self.lease_code_id = Some(LeaseWrapper::default().store(&mut self.app));
        // Bonus: set some funds on the user for future proposals
        let admin = Addr::unchecked(ADMIN);
        if !init_funds.is_empty() && user != &admin {
            self.app
                .send_tokens(admin, user.clone(), &init_funds)
                .unwrap();

            assert_eq!(
                init_funds,
                self.app.wrap().query_all_balances(user).unwrap()
            );
        }

        self
    }

    pub fn get_lease_instance(&mut self) -> Addr {
        LeaseWrapper::default().instantiate(
            &mut self.app,
            self.lease_code_id,
            self.lpp_addr.as_ref().unwrap(),
            &self.denom,
            LeaseWrapperConfig::default(),
        )
    }

    pub fn init_lease(&mut self) -> &mut Self {
        self.lease_code_id = Some(LeaseWrapper::default().store(&mut self.app));
        self
    }

    pub fn init_lpp(&mut self, custom_wrapper: OptionalContractWrapper) -> &mut Self {
        self.init_lpp_with_funds(custom_wrapper, 400)
    }

    pub fn init_lpp_with_funds(&mut self, custom_wrapper: OptionalContractWrapper, amount: Amount) -> &mut Self {
        let mocked_lpp = match custom_wrapper {
            Some(wrapper) => LppWrapper::with_contract_wrapper(wrapper),
            None => LppWrapper::default(),
        };
        self.lpp_addr = Some(
            mocked_lpp
                .instantiate(
                    &mut self.app,
                    Uint64::new(self.lease_code_id.unwrap()),
                    &self.denom,
                    amount,
                )
                .0,
        );
        self.app.update_block(next_block);
        self
    }

    pub fn init_leaser(&mut self) -> &mut Self {
        self.leaser_addr = Some(LeaserWrapper::default().instantiate(
            &mut self.app,
            self.lease_code_id.unwrap(),
            self.lpp_addr.as_ref().unwrap(),
        ));
        self.app.update_block(next_block);

        self
    }

    pub fn init_treasury(&mut self) -> &mut Self {
        self.treasury_addr =
            Some(TreasuryWrapper::default().instantiate(&mut self.app, &self.denom));
        self.app.update_block(next_block);

        self
    }

    pub fn init_profit(&mut self, cadence_hours: u16) -> &mut Self {
        self.profit_addr = Some(ProfitWrapper::default().instantiate(
            &mut self.app,
            cadence_hours,
            self.treasury_addr.as_ref().unwrap(),
            self.timealarms.as_ref().unwrap(),
        ));
        self.app.update_block(next_block);

        self
    }

    pub fn init_timealarms(&mut self) -> &mut Self {
        self.timealarms = Some(TimeAlarmsWrapper::default().instantiate(&mut self.app));
        self.app.update_block(next_block);

        self
    }
    pub fn init_oracle(&mut self, custom_wrapper: OptionalContractWrapperStd) -> &mut Self {
        let mocked_oracle = match custom_wrapper {
            Some(wrapper) => MarketOracleWrapper::with_contract_wrapper(wrapper),
            None => MarketOracleWrapper::default(),
        };

        self.oracle = Some(mocked_oracle.instantiate(&mut self.app, &self.denom, "timealarms"));
        self.app.update_block(next_block);

        self
    }

    pub fn init_dispatcher(&mut self) -> &mut Self {
        // Instantiate Dispatcher contract
        let dispatcher_addr = DispatcherWrapper::default().instantiate(
            &mut self.app,
            self.lpp_addr.as_ref().unwrap(),
            self.oracle.as_ref().unwrap(),
            self.timealarms.as_ref().unwrap(),
            &self.treasury_addr.as_ref().unwrap().clone(),
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

        self.app.update_block(next_block);

        self.dispatcher_addr = Some(dispatcher_addr);
        self
    }
}
