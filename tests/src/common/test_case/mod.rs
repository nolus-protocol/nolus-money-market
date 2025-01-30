use currencies::{
    LeaseGroup as AlarmCurrencies, LeaseGroup as AssetCurrencies, Lpn as BaseCurrency,
    Lpns as BaseCurrencies, Lpns, PaymentGroup as PriceCurrencies,
};
use currency::{CurrencyDTO, CurrencyDef};
use finance::percent::bound::BoundToHundredPercent;
use platform::contract::Code;
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin},
    cw_multi_test::{AppResponse, Executor as _},
    testing::{self, InterChainMsgReceiver, InterChainMsgSender},
};
use versioning::ProtocolMigrationMessage;

use super::{
    lease::{
        InitConfig, Instantiator as LeaseInstantiator, InstantiatorAddresses,
        InstantiatorConfig as LeaseInstantiatorConfig,
    },
    mock_app, CwContractWrapper, ADMIN,
};

use self::{address_book::AddressBook, app::App};

pub mod address_book;
pub mod app;
pub mod builder;
pub mod response;

type OptionalLppEndpoints = Option<
    CwContractWrapper<
        lpp::msg::ExecuteMsg<Lpns>,
        lpp::contract::ContractError,
        lpp::msg::InstantiateMsg,
        lpp::contract::ContractError,
        lpp::msg::QueryMsg<Lpns>,
        lpp::contract::ContractError,
        lpp::msg::SudoMsg,
        lpp::contract::ContractError,
    >,
>;

type OptionalOracleWrapper = Option<
    CwContractWrapper<
        oracle::api::ExecuteMsg<BaseCurrency, BaseCurrencies, AlarmCurrencies, PriceCurrencies>,
        oracle::ContractError,
        oracle::api::InstantiateMsg<PriceCurrencies>,
        oracle::ContractError,
        oracle::api::QueryMsg<PriceCurrencies>,
        oracle::ContractError,
        oracle::api::SudoMsg<PriceCurrencies>,
        oracle::ContractError,
        oracle::ContractError,
        ProtocolMigrationMessage<oracle::api::MigrateMsg>,
        oracle::ContractError,
    >,
>;

#[must_use]
pub(crate) struct TestCase<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
    TimeAlarms,
> {
    pub app: App,
    pub address_book:
        AddressBook<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>,
}

impl TestCase<(), (), (), (), (), (), (), ()> {
    pub const DEX_CONNECTION_ID: &'static str = "connection-0";

    pub const LEASER_IBC_CHANNEL: &'static str = "channel-0";

    pub const LEASE_ICA_ID: &'static str = "0";

    pub const PROFIT_IBC_CHANNEL: &'static str = "channel-1";
    pub const PROFIT_ICA_ID: &'static str = "0";

    pub const DEFAULT_LPP_MIN_UTILIZATION: BoundToHundredPercent = BoundToHundredPercent::ZERO;

    pub fn ica_addr(local: &Addr, id: &str) -> Addr {
        testing::user(&format!("{local}-ica{id}"))
    }

    fn with_reserve(reserve: &[CwCoin]) -> Self {
        let (custom_message_sender, custom_message_receiver): (
            InterChainMsgSender,
            InterChainMsgReceiver,
        ) = testing::new_inter_chain_msg_queue();

        let mut app: App = App::new(
            mock_app(custom_message_sender, reserve),
            custom_message_receiver,
        );

        let lease_code: Code = LeaseInstantiator::store(&mut app);

        Self {
            app,
            address_book: AddressBook::new(lease_code),
        }
    }
}

impl<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
    TestCase<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>
{
    pub fn send_funds_from_admin(&mut self, user_addr: Addr, funds: &[CwCoin]) -> &mut Self {
        let _: AppResponse = self
            .app
            .with_mock_app(|app| app.send_tokens(testing::user(ADMIN), user_addr, funds))
            .unwrap()
            .unwrap_response();

        self
    }
}

impl<ProtocolsRegistry, Treasury>
    TestCase<ProtocolsRegistry, Treasury, Addr, Addr, Addr, Addr, Addr, Addr>
{
    pub fn open_lease<D>(&mut self, lease_currency: CurrencyDTO<AssetCurrencies>) -> Addr
    where
        D: CurrencyDef,
    {
        LeaseInstantiator::instantiate::<D>(
            &mut self.app,
            self.address_book.lease_code(),
            InstantiatorAddresses {
                lpp: self.address_book.lpp().clone(),
                time_alarms: self.address_book.time_alarms().clone(),
                oracle: self.address_book.oracle().clone(),
                profit: self.address_book.profit().clone(),
                reserve: self.address_book.reserve().clone(),
                finalizer: self.address_book.leaser().clone(),
            },
            InitConfig::new(lease_currency, 1000.into(), None),
            LeaseInstantiatorConfig::default(),
            TestCase::DEX_CONNECTION_ID,
            TestCase::LEASE_ICA_ID,
        )
    }
}
