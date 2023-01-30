use cosmwasm_std::{
    testing::{mock_dependencies, MockApi, MockQuerier, MockStorage},
    Binary, ContractResult, Empty, GovMsg, IbcMsg, IbcQuery, OwnedDeps, SystemError, SystemResult,
    WasmQuery,
};
use cw_multi_test::{
    BankKeeper, BasicAppBuilder, DistributionKeeper, FailingModule, StakeKeeper, WasmKeeper,
};
pub use cw_multi_test::{ContractWrapper, Executor};

use crate::cosmwasm_ext::CustomMsg;

use self::custom_msg::Module as CustomMsgModule;

pub type App<Exec = CustomMsg, Query = Empty> =
    cw_multi_test::App<BankKeeper, MockApi, MockStorage, CustomMsgModule, WasmKeeper<Exec, Query>>;

pub type AppBuilder<Exec = CustomMsg, Query = Empty> = cw_multi_test::AppBuilder<
    BankKeeper,
    MockApi,
    MockStorage,
    CustomMsgModule,
    WasmKeeper<Exec, Query>,
    StakeKeeper,
    DistributionKeeper,
    FailingModule<IbcMsg, IbcQuery, Empty>,
    FailingModule<GovMsg, Empty, Empty>,
>;

pub type Contract = dyn cw_multi_test::Contract<CustomMsg>;

pub type CustomMessageSender = std::sync::mpsc::Sender<CustomMsg>;
pub type CustomMessageReceiver = std::sync::mpsc::Receiver<CustomMsg>;

pub fn new_custom_msg_queue() -> (CustomMessageSender, CustomMessageReceiver) {
    std::sync::mpsc::channel()
}

pub fn mock_deps_with_contracts<const N: usize>(
    contracts: [&'static str; N],
) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    customized_mock_deps_with_contracts(mock_dependencies(), contracts)
}

pub fn customized_mock_deps_with_contracts<const N: usize>(
    mut deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    contracts: [&'static str; N],
) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    deps.querier.update_wasm(move |query| match query {
        WasmQuery::ContractInfo { contract_addr }
            if contracts.contains(&contract_addr.as_str()) =>
        {
            SystemResult::Ok(ContractResult::Ok(Binary(Vec::from(
                br#"{"code_id":0,"creator":"","admin":null,"pinned":false,"ibc_port":null}"#
                    as &[u8],
            ))))
        }
        WasmQuery::Smart { contract_addr, .. }
        | WasmQuery::Raw { contract_addr, .. }
        | WasmQuery::ContractInfo { contract_addr, .. } => {
            SystemResult::Err(SystemError::NoSuchContract {
                addr: contract_addr.clone(),
            })
        }
        _ => unimplemented!(),
    });

    deps
}

pub fn new_app(custom_message_sender: Option<CustomMessageSender>) -> AppBuilder {
    BasicAppBuilder::<CustomMsg, Empty>::new_custom()
        .with_custom(CustomMsgModule::new(custom_message_sender))
        .with_wasm::<CustomMsgModule, _>(WasmKeeper::new())
}

mod custom_msg {
    use anyhow::{bail, Result as AnyResult};
    use cosmwasm_schema::schemars::JsonSchema;
    use cosmwasm_std::{Addr, Api, Binary, BlockInfo, CustomQuery, Empty, Querier, Storage};
    use cw_multi_test::{AppResponse, CosmosRouter, Module as ModuleTrait};
    use serde::de::DeserializeOwned;

    use crate::cosmwasm_ext::CustomMsg;

    use super::CustomMessageSender;

    pub struct Module {
        message_sender: Option<CustomMessageSender>,
    }

    impl Module {
        pub fn new(message_sender: Option<CustomMessageSender>) -> Self {
            Self { message_sender }
        }
    }

    impl ModuleTrait for Module {
        type ExecT = CustomMsg;

        type QueryT = Empty;

        type SudoT = Empty;

        fn execute<ExecC, QueryC>(
            &self,
            _api: &dyn Api,
            _storage: &mut dyn Storage,
            _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
            _block: &BlockInfo,
            _sender: Addr,
            msg: Self::ExecT,
        ) -> AnyResult<AppResponse>
        where
            ExecC: std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
            QueryC: CustomQuery + DeserializeOwned + 'static,
        {
            if let Some(sender) = self.message_sender.as_ref() {
                sender
                    .send(msg)
                    .expect("Receiver closed but message had to be sent!");
            }

            Ok(AppResponse::default())
        }

        fn sudo<ExecC, QueryC>(
            &self,
            _api: &dyn Api,
            _storage: &mut dyn Storage,
            _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
            _block: &BlockInfo,
            msg: Self::SudoT,
        ) -> AnyResult<AppResponse>
        where
            ExecC: std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
            QueryC: CustomQuery + DeserializeOwned + 'static,
        {
            bail!("Unexpected sudo msg {:?}", msg)
        }

        fn query(
            &self,
            _api: &dyn Api,
            _storage: &dyn Storage,
            _querier: &dyn Querier,
            _block: &BlockInfo,
            request: Self::QueryT,
        ) -> AnyResult<Binary> {
            bail!("Unexpected custom query {:?}", request)
        }
    }
}
