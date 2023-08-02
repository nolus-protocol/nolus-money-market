use currency::{Currency, Symbol};
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin},
    cw_multi_test::{AppResponse, Executor as _},
    testing::{new_inter_chain_msg_queue, InterChainMsgReceiver, InterChainMsgSender},
};

use super::{
    lease::{
        InitConfig, Instantiator as LeaseInstantiator, InstantiatorAddresses,
        InstantiatorConfig as LeaseInstantiatorConfig,
    },
    mock_app, CwContractWrapper, ADMIN,
};

use self::{
    address_book::AddressBook,
    app::App,
    wasm::{DefaultWasm, Wasm as WasmTrait},
};

pub mod address_book;
pub mod app;
pub mod builder;
pub mod response;
pub mod wasm;

type OptionalLppEndpoints = Option<
    CwContractWrapper<
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
    CwContractWrapper<
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

#[must_use]
pub(crate) struct TestCase<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
where
    Wasm: WasmTrait,
{
    pub app: App<Wasm>,
    pub address_book: AddressBook<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>,
}

impl TestCase<DefaultWasm, (), (), (), (), (), (), ()> {
    pub const LEASER_CONNECTION_ID: &'static str = "connection-0";
    pub const LEASER_IBC_CHANNEL: &'static str = "channel-0";

    pub const PROFIT_ICA_CHANNEL: &'static str = "channel-0";
    pub const PROFIT_ICA_ADDR: &'static str = "ica1";
}

impl<Wasm> TestCase<Wasm, (), (), (), (), (), (), ()>
where
    Wasm: WasmTrait,
{
    fn with_reserve<WasmF>(reserve: &[CwCoin], wasm_f: WasmF) -> Self
    where
        WasmF: FnOnce() -> (Wasm, Wasm::CounterPart),
    {
        let (custom_message_sender, custom_message_receiver): (
            InterChainMsgSender,
            InterChainMsgReceiver,
        ) = new_inter_chain_msg_queue();

        let (wasm, wasm_counter_part): (Wasm, Wasm::CounterPart) = wasm_f();

        let mut app: App<Wasm> = App::new(
            mock_app(custom_message_sender, reserve, wasm),
            wasm_counter_part,
            custom_message_receiver,
        );

        let lease_code_id: u64 = LeaseInstantiator::store(&mut app);

        Self {
            app,
            address_book: AddressBook::new(lease_code_id),
        }
    }
}

impl<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
    TestCase<Wasm, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, TimeAlarms>
where
    Wasm: WasmTrait,
{
    pub fn send_funds_from_admin(&mut self, user_addr: Addr, funds: &[CwCoin]) -> &mut Self {
        let _: AppResponse = self
            .app
            .with_mock_app(|app| app.send_tokens(Addr::unchecked(ADMIN), user_addr, funds))
            .unwrap()
            .unwrap_response();

        self
    }
}

impl<Wasm, Dispatcher, Treasury, Leaser>
    TestCase<Wasm, Dispatcher, Treasury, Addr, Leaser, Addr, Addr, Addr>
where
    Wasm: WasmTrait,
{
    pub fn open_lease<D>(&mut self, lease_currency: Symbol<'_>) -> Addr
    where
        D: Currency,
    {
        LeaseInstantiator::instantiate::<Wasm, D>(
            &mut self.app,
            self.address_book.lease_code_id(),
            InstantiatorAddresses {
                lpp: self.address_book.lpp().clone(),
                time_alarms: self.address_book.time_alarms().clone(),
                oracle: self.address_book.oracle().clone(),
                profit: self.address_book.profit().clone(),
            },
            InitConfig::new(lease_currency, 1000.into(), None),
            LeaseInstantiatorConfig::default(),
            TestCase::LEASER_CONNECTION_ID,
        )
    }
}
