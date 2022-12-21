use std::marker::PhantomData;

use finance::currency::{Currency, Symbol};
use lease::api::dex::{ConnectionParams, Ics20Channel};
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin, Empty, Uint64},
    cw_multi_test::{next_block, Executor},
};

use crate::common::{lease_wrapper::LeaseWrapperAddresses, ContractWrapper, MockApp};

use super::{
    cwcoin,
    dispatcher_wrapper::DispatcherWrapper,
    lease_wrapper::{LeaseWrapper, LeaseWrapperConfig},
    leaser_wrapper::LeaserWrapper,
    lpp_wrapper::LppWrapper,
    mock_app,
    oracle_wrapper::MarketOracleWrapper,
    profit_wrapper::ProfitWrapper,
    timealarms_wrapper::TimeAlarmsWrapper,
    treasury_wrapper::TreasuryWrapper,
    ADMIN,
};

type OptionalContractWrapper = Option<
    ContractWrapper<
        lpp::msg::ExecuteMsg,
        lpp::error::ContractError,
        lpp::msg::InstantiateMsg,
        lpp::error::ContractError,
        lpp::msg::QueryMsg,
        lpp::error::ContractError,
    >,
>;

type OptionalContractWrapperStd = Option<
    ContractWrapper<
        oracle::msg::ExecuteMsg,
        oracle::ContractError,
        oracle::msg::InstantiateMsg,
        oracle::ContractError,
        oracle::msg::QueryMsg,
        oracle::ContractError,
        Empty,
        anyhow::Error,
        oracle::ContractError,
    >,
>;

pub struct TestCase<Lpn> {
    pub app: MockApp,
    pub dispatcher_addr: Option<Addr>,
    pub treasury_addr: Option<Addr>,
    pub profit_addr: Option<Addr>,
    pub leaser_addr: Option<Addr>,
    pub lpp_addr: Option<Addr>,
    pub oracle: Option<Addr>,
    pub timealarms: Option<Addr>,
    pub lease_code_id: Option<u64>,
    _lpn: PhantomData<Lpn>,
}

impl<Lpn> TestCase<Lpn>
where
    Lpn: Currency,
{
    pub fn new() -> Self {
        Self::with_reserve(&[cwcoin::<Lpn, _>(10_000)])
    }

    pub fn with_reserve(reserve: &[CwCoin]) -> Self {
        Self {
            app: mock_app(reserve),
            dispatcher_addr: None,
            treasury_addr: None,
            profit_addr: None,
            leaser_addr: None,
            lpp_addr: None,
            oracle: None,
            timealarms: None,
            lease_code_id: None,
            _lpn: PhantomData,
        }
    }

    pub fn send_funds(&mut self, user_addr: &Addr, funds: Vec<CwCoin>) -> &mut Self {
        self.app
            .send_tokens(Addr::unchecked(ADMIN), user_addr.clone(), &funds)
            .unwrap();

        self
    }

    pub fn init(&mut self, user: &Addr, mut init_funds: Vec<CwCoin>) -> &mut Self {
        self.init_lease();
        // Bonus: set some funds on the user for future proposals
        let admin = Addr::unchecked(ADMIN);

        if !init_funds.is_empty() && user != &admin {
            let coin_sort_fn = |coin: &CwCoin| (coin.denom.clone(), coin.amount.u128());

            init_funds.sort_by_key(coin_sort_fn);

            self.app
                .send_tokens(admin, user.clone(), &init_funds)
                .unwrap();

            assert_eq!(
                {
                    let mut funds = self.app.wrap().query_all_balances(user).unwrap();

                    funds.sort_by_key(coin_sort_fn);

                    funds
                },
                init_funds,
                "Initial funds are not the same!"
            );
        }

        self
    }

    pub fn open_lease<D>(&mut self, lease_currency: Symbol) -> Addr
    where
        D: Currency,
    {
        LeaseWrapper::default().instantiate::<D>(
            &mut self.app,
            self.lease_code_id,
            LeaseWrapperAddresses {
                lpp: self
                    .lpp_addr
                    .clone()
                    .expect("LPP contract not instantiated!"),
                time_alarms: self
                    .oracle
                    .clone()
                    .expect("Time Alarms contract not instantiated!"),
                oracle: self
                    .oracle
                    .clone()
                    .expect("Market Price Oracle contract not instantiated!"),
                profit: self
                    .profit_addr
                    .clone()
                    .expect("Profit contract not instantiated!"),
            },
            lease_currency,
            1000.into(),
            LeaseWrapperConfig::default(),
        )
    }

    pub fn init_lease(&mut self) -> &mut Self {
        self.lease_code_id = Some(LeaseWrapper::default().store(&mut self.app));
        self
    }

    pub fn init_lpp(&mut self, custom_wrapper: OptionalContractWrapper) -> &mut Self {
        self.init_lpp_with_funds(custom_wrapper, vec![CwCoin::new(400, Lpn::BANK_SYMBOL)])
    }

    pub fn init_lpp_with_funds(
        &mut self,
        custom_wrapper: OptionalContractWrapper,
        init_balance: Vec<CwCoin>,
    ) -> &mut Self
    where
        Lpn: Currency,
    {
        let mocked_lpp = match custom_wrapper {
            Some(wrapper) => LppWrapper::with_contract_wrapper(wrapper),
            None => LppWrapper::default(),
        };
        self.lpp_addr = Some(
            mocked_lpp
                .instantiate::<Lpn>(
                    &mut self.app,
                    Uint64::new(self.lease_code_id.unwrap()),
                    init_balance,
                )
                .0,
        );
        self.app.update_block(next_block);
        self
    }

    pub fn init_leaser(&mut self) -> &mut Self {
        self.leaser_addr = Some(
            LeaserWrapper::default().instantiate(
                &mut self.app,
                self.lease_code_id.unwrap(),
                self.lpp_addr.as_ref().unwrap(),
                self.timealarms
                    .clone()
                    .expect("Time Alarms not initialized!"),
                self.oracle
                    .clone()
                    .expect("Market Price Oracle not initialized!"),
                self.profit_addr.clone().expect("Profit not initialized!"),
            ),
        );
        self.app
            .execute_contract(
                Addr::unchecked(ADMIN),
                self.leaser_addr.clone().unwrap(),
                &leaser::msg::ExecuteMsg::SetupDex(ConnectionParams {
                    connection_id: "connection-0".into(),
                    transfer_channel: Ics20Channel {
                        local_endpoint: "channel-0".into(),
                        remote_endpoint: "channel-422".into(),
                    },
                }),
                &[cwcoin::<Lpn, _>(3)],
            )
            .unwrap();

        self.app.update_block(next_block);

        self
    }

    pub fn init_treasury(&mut self) -> &mut Self {
        self.treasury_addr = Some(TreasuryWrapper::default().instantiate::<Lpn>(&mut self.app));
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

        self.oracle = Some(mocked_oracle.instantiate::<Lpn>(&mut self.app));
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
