use std::marker::PhantomData;

use currency::{Currency, Symbol};
use finance::percent::Percent;
use lease::api::{ConnectionParams, Ics20Channel};
use platform::ica::OpenAckVersion;
use profit::msg::{ConfigResponse as ProfitConfigResponse, QueryMsg as ProfitQueryMsg};
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin, Uint64},
    cw_multi_test::{next_block, Executor},
    neutron_sdk::{bindings::msg::NeutronMsg, sudo::msg::SudoMsg as NeutronSudoMsg},
    testing::{new_custom_msg_queue, CustomMessageSender, WrappedCustomMessageReceiver},
};

use crate::common::{
    lease_wrapper::{LeaseInitConfig, LeaseWrapperAddresses},
    ContractWrapper, MockApp,
};

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

type OptionalLppWrapper = Option<
    ContractWrapper<
        lpp::msg::ExecuteMsg,
        lpp::error::ContractError,
        lpp::msg::InstantiateMsg,
        lpp::error::ContractError,
        lpp::msg::QueryMsg,
        lpp::error::ContractError,
        lpp::msg::SudoMsg,
        lpp::error::ContractError,
    >,
>;

type OptionalOracleWrapper = Option<
    ContractWrapper<
        oracle::msg::ExecuteMsg,
        oracle::ContractError,
        oracle::msg::InstantiateMsg,
        oracle::ContractError,
        oracle::msg::QueryMsg,
        oracle::ContractError,
        oracle::msg::SudoMsg,
        oracle::ContractError,
        oracle::ContractError,
    >,
>;

pub struct TestCase {
    pub app: MockApp,
    pub message_receiver: WrappedCustomMessageReceiver,
    dispatcher_addr: Option<Addr>,
    treasury_addr: Option<Addr>,
    profit_addr: Option<Addr>,
    leaser_addr: Option<Addr>,
    lpp_addr: Option<Addr>,
    oracle_addr: Option<Addr>,
    time_alarms_addr: Option<Addr>,
    lease_code_id: u64,
}

impl TestCase {
    pub const LEASER_CONNECTION_ID: &'static str = "connection-0";

    fn with_reserve(reserve: &[CwCoin]) -> Self {
        let (custom_message_sender, custom_message_receiver): (
            CustomMessageSender,
            WrappedCustomMessageReceiver,
        ) = new_custom_msg_queue();

        let mut app: MockApp = mock_app(custom_message_sender, reserve);

        let lease_code_id: u64 = Self::store_lease(&mut app);

        Self {
            app,
            message_receiver: custom_message_receiver,
            dispatcher_addr: None,
            treasury_addr: None,
            profit_addr: None,
            leaser_addr: None,
            lpp_addr: None,
            oracle_addr: None,
            time_alarms_addr: None,
            lease_code_id,
        }
    }

    pub fn send_funds_from_admin(&mut self, user_addr: Addr, funds: &[CwCoin]) -> &mut Self {
        self.app
            .send_tokens(Addr::unchecked(ADMIN), user_addr, funds)
            .unwrap();

        self
    }

    pub fn dispatcher(&self) -> &Addr {
        self.dispatcher_addr.as_ref().unwrap()
    }

    pub fn treasury(&self) -> &Addr {
        self.treasury_addr.as_ref().unwrap()
    }

    pub fn profit(&self) -> &Addr {
        self.profit_addr.as_ref().unwrap()
    }

    pub fn leaser(&self) -> &Addr {
        self.leaser_addr.as_ref().unwrap()
    }

    pub fn lpp(&self) -> &Addr {
        self.lpp_addr.as_ref().unwrap()
    }

    pub fn oracle(&self) -> &Addr {
        self.oracle_addr.as_ref().unwrap()
    }

    pub fn time_alarms(&self) -> &Addr {
        self.time_alarms_addr.as_ref().unwrap()
    }

    pub fn lease_code_id(&self) -> u64 {
        self.lease_code_id
    }

    pub fn open_lease<D>(&mut self, lease_currency: Symbol<'_>) -> Addr
    where
        D: Currency,
    {
        let lease_code_id = self.lease_code_id();
        let lpp = self.lpp().clone();
        let time_alarms = self.time_alarms().clone();
        let oracle = self.oracle().clone();
        let profit = self.profit().clone();

        let lease: Addr = LeaseWrapper::default().instantiate::<D>(
            &mut self.app,
            Some(lease_code_id),
            LeaseWrapperAddresses {
                lpp,
                time_alarms,
                oracle,
                profit,
            },
            LeaseInitConfig::new(lease_currency, 1000.into(), None),
            LeaseWrapperConfig::default(),
        );

        self.message_receiver.assert_empty();

        lease
    }

    pub fn store_new_lease_code(&mut self) -> &mut Self {
        self.lease_code_id = Self::store_lease(&mut self.app);

        self
    }

    fn init_lpp<Lpn>(
        &mut self,
        custom_wrapper: OptionalLppWrapper,
        init_balance: &[CwCoin],
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> &mut Self
    where
        Lpn: Currency,
    {
        assert_eq!(self.lpp_addr, None);

        let mocked_lpp = match custom_wrapper {
            Some(wrapper) => LppWrapper::with_contract_wrapper(wrapper),
            None => LppWrapper::default(),
        };

        let lease_code_id = self.lease_code_id();

        self.lpp_addr = Some(
            mocked_lpp
                .instantiate::<Lpn>(
                    &mut self.app,
                    Uint64::new(lease_code_id),
                    init_balance,
                    base_interest_rate,
                    utilization_optimal,
                    addon_optimal_interest_rate,
                )
                .0,
        );

        self.app.update_block(next_block);

        self.message_receiver.assert_empty();

        self
    }

    fn init_leaser(&mut self) -> &mut Self {
        assert_eq!(self.leaser_addr, None);

        let lease_code_id = self.lease_code_id();
        let lpp = self.lpp().clone();
        let time_alarms = self.time_alarms().clone();
        let oracle = self.oracle().clone();
        let profit = self.profit().clone();

        let leaser = LeaserWrapper::default().instantiate(
            &mut self.app,
            lease_code_id,
            lpp,
            time_alarms,
            oracle,
            profit,
        );

        self.leaser_addr = Some(leaser.clone());

        self.message_receiver.assert_empty();

        self.app
            .wasm_sudo(
                leaser,
                &leaser::msg::SudoMsg::SetupDex(ConnectionParams {
                    connection_id: "connection-0".into(),
                    transfer_channel: Ics20Channel {
                        local_endpoint: "channel-0".into(),
                        remote_endpoint: "channel-422".into(),
                    },
                }),
            )
            .unwrap();

        self.app.update_block(next_block);

        self.message_receiver.assert_empty();

        self
    }

    fn init_treasury<Lpn>(&mut self, wrapper: TreasuryWrapper) -> &mut Self
    where
        Lpn: Currency,
    {
        assert_eq!(self.treasury_addr, None);

        self.treasury_addr = Some(wrapper.instantiate::<Lpn>(&mut self.app));

        self.app.update_block(next_block);

        self.message_receiver.assert_empty();

        self
    }

    fn init_profit(&mut self, cadence_hours: u16) -> &mut Self {
        assert_eq!(self.profit_addr, None);

        const CONNECTION_ID: &str = "dex-connection";

        let treasury = self.treasury().clone();
        let oracle = self.oracle().clone();
        let time_alarms = self.time_alarms().clone();

        let profit = ProfitWrapper::default().instantiate(
            &mut self.app,
            cadence_hours,
            treasury,
            oracle,
            time_alarms,
        );

        self.profit_addr = Some(profit.clone());

        self.app.update_block(next_block);

        self.app
            .wasm_sudo(
                profit.clone(),
                &NeutronSudoMsg::OpenAck {
                    port_id: CONNECTION_ID.into(),
                    channel_id: "channel-1".into(),
                    counterparty_channel_id: "channel-1".into(),
                    counterparty_version: String::new(),
                },
            )
            .unwrap();

        let NeutronMsg::RegisterInterchainAccount { connection_id, .. } = self.message_receiver.try_recv().unwrap() else {
            unreachable!()
        };
        assert_eq!(&connection_id, CONNECTION_ID);

        self.app
            .wasm_sudo(
                profit.clone(),
                &NeutronSudoMsg::OpenAck {
                    port_id: "ica-port".into(),
                    channel_id: "channel-1".into(),
                    counterparty_channel_id: "channel-1".into(),
                    counterparty_version: serde_json_wasm::to_string(&OpenAckVersion {
                        version: "1".into(),
                        controller_connection_id: CONNECTION_ID.into(),
                        host_connection_id: "DEADCODE".into(),
                        address: "ica1".into(),
                        encoding: "DEADCODE".into(),
                        tx_type: "DEADCODE".into(),
                    })
                    .unwrap(),
                },
            )
            .unwrap();

        self.message_receiver.assert_empty();

        let ProfitConfigResponse {
            cadence_hours: reported_cadence_hours,
        } = self
            .app
            .wrap()
            .query_wasm_smart(profit, &ProfitQueryMsg::Config {})
            .unwrap();

        assert_eq!(reported_cadence_hours, cadence_hours);

        self
    }

    fn init_time_alarms(&mut self) -> &mut Self {
        assert_eq!(self.time_alarms_addr, None);

        self.time_alarms_addr = Some(TimeAlarmsWrapper::default().instantiate(&mut self.app));

        self.app.update_block(next_block);

        self.message_receiver.assert_empty();

        self
    }

    fn init_oracle<Lpn>(&mut self, custom_wrapper: OptionalOracleWrapper) -> &mut Self
    where
        Lpn: Currency,
    {
        assert_eq!(self.oracle_addr, None);

        self.oracle_addr = Some(
            custom_wrapper
                .map_or_else(Default::default, MarketOracleWrapper::with_contract_wrapper)
                .instantiate::<Lpn>(&mut self.app),
        );

        self.app.update_block(next_block);

        self.message_receiver.assert_empty();

        self
    }

    fn init_dispatcher(&mut self) -> &mut Self {
        assert_eq!(self.dispatcher_addr, None);

        let lpp = self.lpp().clone();
        let oracle = self.oracle().clone();
        let time_alarms = self.time_alarms().clone();
        let treasury = self.treasury().clone();

        // Instantiate Dispatcher contract
        let dispatcher_addr = DispatcherWrapper::default().instantiate(
            &mut self.app,
            lpp,
            oracle,
            time_alarms,
            treasury.clone(),
        );

        self.dispatcher_addr = Some(dispatcher_addr.clone());

        self.app.update_block(next_block);

        self.message_receiver.assert_empty();

        self.app
            .wasm_sudo(
                treasury,
                &treasury::msg::SudoMsg::ConfigureRewardTransfer {
                    rewards_dispatcher: dispatcher_addr,
                },
            )
            .unwrap();

        self.app.update_block(next_block);

        self.message_receiver.assert_empty();

        self
    }

    fn store_lease(app: &mut MockApp) -> u64 {
        LeaseWrapper::default().store(app)
    }
}

pub struct Builder<Lpn> {
    test_case: TestCase,
    _lpn: PhantomData<Lpn>,
}

impl<Lpn> Builder<Lpn>
where
    Lpn: Currency,
{
    pub fn new() -> Self {
        Self::with_reserve(&[cwcoin::<Lpn, _>(10_000)])
    }

    pub fn with_reserve(reserve: &[CwCoin]) -> Self {
        Self {
            test_case: TestCase::with_reserve(reserve),
            _lpn: PhantomData,
        }
    }

    pub fn init_lpp(
        self,
        custom_wrapper: OptionalLppWrapper,
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> Self {
        self.init_lpp_with_funds(
            custom_wrapper,
            &[CwCoin::new(400, Lpn::BANK_SYMBOL)],
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        )
    }

    pub fn init_lpp_with_funds(
        mut self,
        custom_wrapper: OptionalLppWrapper,
        init_balance: &[CwCoin],
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> Self {
        self.test_case.init_lpp::<Lpn>(
            custom_wrapper,
            init_balance,
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        );

        self
    }

    pub fn init_leaser(mut self) -> Self {
        self.test_case.init_leaser();

        self
    }

    pub fn init_treasury(mut self) -> Self {
        self.test_case
            .init_treasury::<Lpn>(TreasuryWrapper::new_with_no_dispatcher());

        self
    }

    pub fn init_treasury_with_dispatcher(mut self, rewards_dispatcher: Addr) -> Self {
        self.test_case
            .init_treasury::<Lpn>(TreasuryWrapper::new(rewards_dispatcher));

        self
    }

    pub fn init_profit(mut self, cadence_hours: u16) -> Self {
        self.test_case.init_profit(cadence_hours);

        self
    }

    pub fn init_time_alarms(mut self) -> Self {
        self.test_case.init_time_alarms();

        self
    }

    pub fn init_oracle(mut self, custom_wrapper: OptionalOracleWrapper) -> Self {
        self.test_case.init_oracle::<Lpn>(custom_wrapper);

        self
    }

    pub fn init_dispatcher(mut self) -> Self {
        self.test_case.init_dispatcher();

        self
    }

    pub fn into_generic(self) -> TestCase {
        self.test_case
    }
}
