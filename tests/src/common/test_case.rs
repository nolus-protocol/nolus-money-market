use std::{fmt::Debug, marker::PhantomData, sync::mpsc::TryIter as MpscIter};

use serde::Serialize;

use currency::{Currency, Symbol};
use finance::{duration::Duration, percent::Percent};
use lease::api::{ConnectionParams, Ics20Channel};
use platform::ica::OpenAckVersion;
use profit::msg::{ConfigResponse as ProfitConfigResponse, QueryMsg as ProfitQueryMsg};
use sdk::{
    cosmwasm_ext::{CosmosMsg, CustomMsg},
    cosmwasm_std::{Addr, BlockInfo, Coin as CwCoin, Empty, QuerierWrapper, Uint64},
    cw_multi_test::{next_block, AppResponse, Contract as CwContract, Executor as _},
    neutron_sdk::sudo::msg::SudoMsg as NeutronSudoMsg,
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
    AppExt, ADMIN,
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

pub(crate) struct AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
    dispatcher_addr: Dispatcher,
    treasury_addr: Treasury,
    profit_addr: Profit,
    leaser_addr: Leaser,
    lpp_addr: Lpp,
    oracle_addr: Oracle,
    time_alarms_addr: TimeAlarms,
    lease_code_id: u64,
}

impl AddressBook<(), (), (), (), (), (), ()> {
    const fn new(lease_code_id: u64) -> Self {
        Self {
            dispatcher_addr: (),
            treasury_addr: (),
            profit_addr: (),
            leaser_addr: (),
            lpp_addr: (),
            oracle_addr: (),
            time_alarms_addr: (),
            lease_code_id,
        }
    }
}

impl<Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<(), Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
{
    fn with_dispatcher(
        self,
        dispatcher_addr: Addr,
    ) -> AddressBook<Addr, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
        AddressBook {
            dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<Addr, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub const fn dispatcher(&self) -> &Addr {
        &self.dispatcher_addr
    }
}

impl<Dispatcher, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, (), Profit, Leaser, Lpp, Oracle, TimeAlarms>
{
    fn with_treasury(
        self,
        treasury_addr: Addr,
    ) -> AddressBook<Dispatcher, Addr, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
        AddressBook {
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr,
            profit_addr: self.profit_addr,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Dispatcher, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Addr, Profit, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub const fn treasury(&self) -> &Addr {
        &self.treasury_addr
    }
}

impl<Dispatcher, Treasury, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, (), Leaser, Lpp, Oracle, TimeAlarms>
{
    fn with_profit(
        self,
        profit_addr: Addr,
    ) -> AddressBook<Dispatcher, Treasury, Addr, Leaser, Lpp, Oracle, TimeAlarms> {
        AddressBook {
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Dispatcher, Treasury, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Addr, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub const fn profit(&self) -> &Addr {
        &self.profit_addr
    }
}

impl<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, (), Lpp, Oracle, TimeAlarms>
{
    fn with_leaser(
        self,
        leaser_addr: Addr,
    ) -> AddressBook<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms> {
        AddressBook {
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Dispatcher, Treasury, Profit, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, Addr, Lpp, Oracle, TimeAlarms>
{
    pub const fn leaser(&self) -> &Addr {
        &self.leaser_addr
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, (), Oracle, TimeAlarms>
{
    fn with_lpp(
        self,
        lpp_addr: Addr,
    ) -> AddressBook<Dispatcher, Treasury, Profit, Leaser, Addr, Oracle, TimeAlarms> {
        AddressBook {
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            leaser_addr: self.leaser_addr,
            lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, Addr, Oracle, TimeAlarms>
{
    pub const fn lpp(&self) -> &Addr {
        &self.lpp_addr
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Lpp, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, (), TimeAlarms>
{
    fn with_oracle(
        self,
        oracle_addr: Addr,
    ) -> AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Addr, TimeAlarms> {
        AddressBook {
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr,
            time_alarms_addr: self.time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Lpp, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Addr, TimeAlarms>
{
    pub const fn oracle(&self) -> &Addr {
        &self.oracle_addr
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, ()>
{
    fn with_time_alarms(
        self,
        time_alarms_addr: Addr,
    ) -> AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr> {
        AddressBook {
            dispatcher_addr: self.dispatcher_addr,
            treasury_addr: self.treasury_addr,
            profit_addr: self.profit_addr,
            leaser_addr: self.leaser_addr,
            lpp_addr: self.lpp_addr,
            oracle_addr: self.oracle_addr,
            time_alarms_addr,
            lease_code_id: self.lease_code_id,
        }
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>
{
    pub const fn time_alarms(&self) -> &Addr {
        &self.time_alarms_addr
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub const fn lease_code_id(&self) -> u64 {
        self.lease_code_id
    }
}

#[must_use]
pub(crate) struct WrappedApp {
    app: MockApp,
    message_receiver: WrappedCustomMessageReceiver,
}

impl WrappedApp {
    pub const fn new(app: MockApp, message_receiver: WrappedCustomMessageReceiver) -> Self {
        Self {
            app,
            message_receiver,
        }
    }

    #[must_use]
    #[inline]
    pub fn store_code(&mut self, code: Box<dyn CwContract<CustomMsg, Empty>>) -> u64 {
        self.app.store_code(code)
    }

    #[inline]
    pub fn time_shift(&mut self, duration: Duration) {
        self.app.time_shift(duration)
    }

    #[inline]
    pub fn update_block<F>(&mut self, f: F)
    where
        F: Fn(&mut BlockInfo),
    {
        self.app.update_block(f)
    }

    #[inline]
    pub fn block_info(&self) -> BlockInfo {
        self.app.block_info()
    }

    #[inline]
    pub fn send_tokens<'r>(
        &'r mut self,
        sender: Addr,
        recipient: Addr,
        amount: &[CwCoin],
    ) -> anyhow::Result<WrappedResponse<'r, AppResponse>> {
        self.app
            .send_tokens(sender, recipient, amount)
            .map(|result: AppResponse| WrappedResponse {
                receiver: &mut self.message_receiver,
                result,
            })
    }

    #[must_use]
    pub fn instantiate<'r, T, U>(
        &'r mut self,
        code_id: u64,
        sender: Addr,
        init_msg: &T,
        send_funds: &[CwCoin],
        label: U,
        admin: Option<String>,
    ) -> anyhow::Result<WrappedResponse<'r, Addr>>
    where
        T: Debug + Serialize,
        U: Into<String>,
    {
        self.with_app(|app: &mut MockApp| {
            app.instantiate_contract(code_id, sender, init_msg, send_funds, label, admin)
        })
    }

    #[must_use]
    pub fn execute<'r, T>(
        &'r mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &T,
        send_funds: &[CwCoin],
    ) -> anyhow::Result<WrappedResponse<'r, AppResponse>>
    where
        T: Debug + Serialize,
    {
        self.with_app(|app: &mut MockApp| {
            app.execute_contract(sender, contract_addr, msg, send_funds)
        })
    }

    #[must_use]
    pub fn execute_raw<T>(
        &mut self,
        sender: Addr,
        msg: T,
    ) -> anyhow::Result<WrappedResponse<'_, AppResponse>>
    where
        T: Into<CosmosMsg>,
    {
        self.with_app(|app: &mut MockApp| app.execute(sender, msg.into()))
    }

    #[must_use]
    pub fn sudo<'r, T, U>(
        &'r mut self,
        contract_addr: T,
        msg: &U,
    ) -> anyhow::Result<WrappedResponse<'r, AppResponse>>
    where
        T: Into<Addr>,
        U: Serialize,
    {
        self.with_app(|app: &mut MockApp| app.wasm_sudo(contract_addr, msg))
    }

    #[must_use]
    pub fn with_app<'r, F, R>(&'r mut self, f: F) -> anyhow::Result<WrappedResponse<'r, R>>
    where
        F: FnOnce(&'r mut MockApp) -> anyhow::Result<R>,
    {
        self.message_receiver.try_recv().unwrap_err();

        match f(&mut self.app) {
            Ok(result) => Ok(WrappedResponse {
                receiver: &mut self.message_receiver,
                result,
            }),
            Err(error) => {
                // On error no messages should be "sent out".
                while self.message_receiver.try_iter().next().is_some() {}

                Err(error)
            }
        }
    }

    pub fn query(&self) -> QuerierWrapper<'_, Empty> {
        self.app.wrap()
    }
}

#[must_use]
#[derive(Debug)]
pub struct WrappedResponse<'r, T> {
    receiver: &'r mut WrappedCustomMessageReceiver,
    result: T,
}

impl<'r> Iterator for WrappedResponse<'r, ()> {
    type Item = CustomMsg;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.try_recv().ok()
    }
}

impl<'r, T> WrappedResponse<'r, T> {
    #[inline]
    pub fn receiver(&mut self) -> &mut WrappedCustomMessageReceiver {
        self.receiver
    }

    #[inline]
    pub fn iter(&mut self) -> MpscIter<'_, CustomMsg> {
        self.receiver.try_iter()
    }

    pub fn clear_result(self) -> WrappedResponse<'r, ()> {
        WrappedResponse {
            receiver: self.receiver,
            result: (),
        }
    }

    #[must_use]
    pub fn unwrap_response(self) -> T {
        assert_eq!(self.receiver.try_recv().ok(), None);

        self.result
    }
}

impl<'r> WrappedResponse<'r, ()> {
    pub fn set_result<T>(self, result: T) -> WrappedResponse<'r, T> {
        WrappedResponse {
            receiver: self.receiver,
            result,
        }
    }
}

pub(crate) struct TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
    pub app: WrappedApp,
    pub address_book: AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
}

impl TestCase<(), (), (), (), (), (), ()> {
    pub const LEASER_CONNECTION_ID: &'static str = "connection-0";

    fn with_reserve(reserve: &[CwCoin]) -> Self {
        let (custom_message_sender, custom_message_receiver): (
            CustomMessageSender,
            WrappedCustomMessageReceiver,
        ) = new_custom_msg_queue();

        let mut app: WrappedApp = WrappedApp::new(
            mock_app(custom_message_sender, reserve),
            custom_message_receiver,
        );

        let lease_code_id: u64 = Self::store_lease_code(&mut app);

        Self {
            app,
            address_book: AddressBook::new(lease_code_id),
        }
    }
}

impl<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub fn send_funds_from_admin(&mut self, user_addr: Addr, funds: &[CwCoin]) -> &mut Self {
        let _: AppResponse = self
            .app
            .with_app(|app| app.send_tokens(Addr::unchecked(ADMIN), user_addr, funds))
            .unwrap()
            .unwrap_response();

        self
    }

    pub fn store_new_lease_code(&mut self) -> &mut Self {
        self.address_book.lease_code_id = Self::store_lease_code(&mut self.app);

        self
    }

    fn store_lease_code(app: &mut WrappedApp) -> u64 {
        LeaseWrapper::default().store(app)
    }
}

impl<Dispatcher, Treasury, Leaser> TestCase<Dispatcher, Treasury, Addr, Leaser, Addr, Addr, Addr> {
    pub fn open_lease<'r, D>(&'r mut self, lease_currency: Symbol<'_>) -> Addr
    where
        D: Currency,
    {
        LeaseWrapper::default().instantiate::<D>(
            &mut self.app,
            Some(self.address_book.lease_code_id),
            LeaseWrapperAddresses {
                lpp: self.address_book.lpp_addr.clone(),
                time_alarms: self.address_book.time_alarms_addr.clone(),
                oracle: self.address_book.oracle_addr.clone(),
                profit: self.address_book.profit_addr.clone(),
            },
            LeaseInitConfig::new(lease_currency, 1000.into(), None),
            LeaseWrapperConfig::default(),
            TestCase::LEASER_CONNECTION_ID,
        )
    }
}

pub(crate) type BlankBuilder<Lpn> = Builder<Lpn, (), (), (), (), (), (), ()>;

pub(crate) struct Builder<Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
    test_case: TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
    _lpn: PhantomData<Lpn>,
}

impl<Lpn> Builder<Lpn, (), (), (), (), (), (), ()>
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
}

impl<Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    Builder<Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
where
    Lpn: Currency,
{
    pub fn into_generic(
        self,
    ) -> TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
        self.test_case
    }
}

impl<Lpn, Profit, Leaser> Builder<Lpn, (), Addr, Profit, Leaser, Addr, Addr, Addr>
where
    Lpn: Currency,
{
    pub fn init_dispatcher(self) -> Builder<Lpn, Addr, Addr, Profit, Leaser, Addr, Addr, Addr> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        // Instantiate Dispatcher contract
        let dispatcher_addr: Addr = DispatcherWrapper::default().instantiate(
            &mut test_case.app,
            test_case.address_book.lpp_addr.clone(),
            test_case.address_book.oracle_addr.clone(),
            test_case.address_book.time_alarms_addr.clone(),
            test_case.address_book.treasury_addr.clone(),
        );

        test_case.app.update_block(next_block);

        let _: AppResponse = test_case
            .app
            .sudo(
                test_case.address_book.treasury_addr.clone(),
                &treasury::msg::SudoMsg::ConfigureRewardTransfer {
                    rewards_dispatcher: dispatcher_addr.clone(),
                },
            )
            .unwrap()
            .unwrap_response();

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_dispatcher(dispatcher_addr),
            },
            _lpn,
        }
    }
}

impl<Lpn, Dispatcher, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    Builder<Lpn, Dispatcher, (), Profit, Leaser, Lpp, Oracle, TimeAlarms>
where
    Lpn: Currency,
{
    pub fn init_treasury_without_dispatcher(
        self,
    ) -> Builder<Lpn, Dispatcher, Addr, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
        self.init_treasury(TreasuryWrapper::new_with_no_dispatcher())
    }

    pub fn init_treasury_with_dispatcher(
        self,
        rewards_dispatcher: Addr,
    ) -> Builder<Lpn, Dispatcher, Addr, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
        self.init_treasury(TreasuryWrapper::new(rewards_dispatcher))
    }

    fn init_treasury(
        self,
        treasury: TreasuryWrapper,
    ) -> Builder<Lpn, Dispatcher, Addr, Profit, Leaser, Lpp, Oracle, TimeAlarms> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let treasury_addr: Addr = treasury.instantiate::<Lpn>(&mut test_case.app);

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_treasury(treasury_addr),
            },
            _lpn,
        }
    }
}

impl<Lpn, Dispatcher, Leaser, Lpp> Builder<Lpn, Dispatcher, Addr, (), Leaser, Lpp, Addr, Addr>
where
    Lpn: Currency,
{
    pub fn init_profit(
        self,
        cadence_hours: u16,
    ) -> Builder<Lpn, Dispatcher, Addr, Addr, Leaser, Lpp, Addr, Addr> {
        const CONNECTION_ID: &str = "dex-connection";

        let Self {
            mut test_case,
            _lpn,
        } = self;

        let profit_addr: Addr = ProfitWrapper::default().instantiate(
            &mut test_case.app,
            cadence_hours,
            test_case.address_book.treasury_addr.clone(),
            test_case.address_book.oracle_addr.clone(),
            test_case.address_book.time_alarms_addr.clone(),
        );

        test_case.app.update_block(next_block);

        let mut response: WrappedResponse<'_, AppResponse> = test_case
            .app
            .sudo(
                profit_addr.clone(),
                &NeutronSudoMsg::OpenAck {
                    port_id: CONNECTION_ID.into(),
                    channel_id: "channel-1".into(),
                    counterparty_channel_id: "channel-1".into(),
                    counterparty_version: String::new(),
                },
            )
            .unwrap();

        response.receiver().assert_register_ica(CONNECTION_ID);
        let _: AppResponse = response.unwrap_response();

        test_case.app.update_block(next_block);

        let _: AppResponse = test_case
            .app
            .sudo(
                profit_addr.clone(),
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
            .unwrap()
            .unwrap_response();

        let ProfitConfigResponse {
            cadence_hours: reported_cadence_hours,
        } = test_case
            .app
            .query()
            .query_wasm_smart(profit_addr.clone(), &ProfitQueryMsg::Config {})
            .unwrap();

        assert_eq!(reported_cadence_hours, cadence_hours);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_profit(profit_addr),
            },
            _lpn,
        }
    }
}

impl<Lpn, Dispatcher, Treasury> Builder<Lpn, Dispatcher, Treasury, Addr, (), Addr, Addr, Addr>
where
    Lpn: Currency,
{
    pub fn init_leaser(self) -> Builder<Lpn, Dispatcher, Treasury, Addr, Addr, Addr, Addr, Addr> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let leaser_addr = LeaserWrapper::default().instantiate(
            &mut test_case.app,
            test_case.address_book.lease_code_id,
            test_case.address_book.lpp_addr.clone(),
            test_case.address_book.time_alarms_addr.clone(),
            test_case.address_book.oracle_addr.clone(),
            test_case.address_book.profit_addr.clone(),
        );

        let _: AppResponse = test_case
            .app
            .sudo(
                leaser_addr.clone(),
                &leaser::msg::SudoMsg::SetupDex(ConnectionParams {
                    connection_id: "connection-0".into(),
                    transfer_channel: Ics20Channel {
                        local_endpoint: "channel-0".into(),
                        remote_endpoint: "channel-422".into(),
                    },
                }),
            )
            .unwrap()
            .unwrap_response();

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_leaser(leaser_addr),
            },
            _lpn,
        }
    }
}

impl<Lpn, Dispatcher, Treasury, Profit, Leaser, Oracle, TimeAlarms>
    Builder<Lpn, Dispatcher, Treasury, Profit, Leaser, (), Oracle, TimeAlarms>
where
    Lpn: Currency,
{
    pub fn init_lpp(
        self,
        custom_wrapper: OptionalLppWrapper,
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> Builder<Lpn, Dispatcher, Treasury, Profit, Leaser, Addr, Oracle, TimeAlarms> {
        self.init_lpp_with_funds(
            custom_wrapper,
            &[CwCoin::new(400, Lpn::BANK_SYMBOL)],
            base_interest_rate,
            utilization_optimal,
            addon_optimal_interest_rate,
        )
    }

    pub fn init_lpp_with_funds(
        self,
        custom_wrapper: OptionalLppWrapper,
        init_balance: &[CwCoin],
        base_interest_rate: Percent,
        utilization_optimal: Percent,
        addon_optimal_interest_rate: Percent,
    ) -> Builder<Lpn, Dispatcher, Treasury, Profit, Leaser, Addr, Oracle, TimeAlarms> {
        let mocked_lpp = match custom_wrapper {
            Some(wrapper) => LppWrapper::with_contract_wrapper(wrapper),
            None => LppWrapper::default(),
        };

        let Self {
            mut test_case,
            _lpn,
        } = self;

        let lpp_addr = mocked_lpp
            .instantiate::<Lpn>(
                &mut test_case.app,
                Uint64::new(test_case.address_book.lease_code_id),
                init_balance,
                base_interest_rate,
                utilization_optimal,
                addon_optimal_interest_rate,
            )
            .0;

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_lpp(lpp_addr),
            },
            _lpn,
        }
    }
}

impl<Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, TimeAlarms>
    Builder<Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, (), TimeAlarms>
where
    Lpn: Currency,
{
    pub fn init_oracle(
        self,
        custom_wrapper: OptionalOracleWrapper,
    ) -> Builder<Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Addr, TimeAlarms> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let oracle_addr: Addr = custom_wrapper
            .map_or_else(Default::default, MarketOracleWrapper::with_contract_wrapper)
            .instantiate::<Lpn>(&mut test_case.app);

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_oracle(oracle_addr),
            },
            _lpn,
        }
    }
}

impl<Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>
    Builder<Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, ()>
where
    Lpn: Currency,
{
    pub fn init_time_alarms(
        self,
    ) -> Builder<Lpn, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr> {
        let Self {
            mut test_case,
            _lpn,
        } = self;

        let time_alarms_addr: Addr = TimeAlarmsWrapper::default().instantiate(&mut test_case.app);

        test_case.app.update_block(next_block);

        Builder {
            test_case: TestCase {
                app: test_case.app,
                address_book: test_case.address_book.with_time_alarms(time_alarms_addr),
            },
            _lpn,
        }
    }
}
