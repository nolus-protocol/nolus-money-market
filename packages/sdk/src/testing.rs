use cosmwasm_std::{
    testing::{mock_dependencies, MockApi, MockQuerier, MockStorage},
    Binary, ContractResult, Empty, GovMsg, IbcMsg, IbcQuery, OwnedDeps, SystemError, SystemResult,
    WasmQuery,
};
use cw_multi_test::{
    BankKeeper, BasicAppBuilder as BasicCwAppBuilder, DistributionKeeper, FailingModule,
    StakeKeeper, WasmKeeper,
};
pub use cw_multi_test::{ContractWrapper as CwContractWrapper, Executor as CwExecutor};

use crate::cosmwasm_ext::InterChainMsg;

use self::custom_msg::Module as CustomMsgModule;

pub type CwApp<Exec = InterChainMsg, Query = Empty> =
    cw_multi_test::App<BankKeeper, MockApi, MockStorage, CustomMsgModule, WasmKeeper<Exec, Query>>;

pub type CwAppBuilder<Exec = InterChainMsg, Query = Empty> = cw_multi_test::AppBuilder<
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

pub type CwContract = dyn cw_multi_test::Contract<InterChainMsg>;

pub type InterChainMsgSender = std::sync::mpsc::Sender<InterChainMsg>;
pub type InterChainMsgReceiver = std::sync::mpsc::Receiver<InterChainMsg>;

pub fn new_inter_chain_msg_queue() -> (InterChainMsgSender, InterChainMsgReceiver) {
    let (sender, receiver): (InterChainMsgSender, InterChainMsgReceiver) =
        std::sync::mpsc::channel();

    (sender, receiver)
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

pub fn new_app(message_sender: InterChainMsgSender) -> CwAppBuilder {
    BasicCwAppBuilder::<InterChainMsg, Empty>::new_custom()
        .with_custom(CustomMsgModule::new(message_sender))
        .with_wasm::<CustomMsgModule, _>(WasmKeeper::new())
}

mod custom_msg {
    use anyhow::{bail, Result as AnyResult};
    use cosmwasm_schema::schemars::JsonSchema;
    use cosmwasm_std::{Addr, Api, Binary, BlockInfo, CustomQuery, Empty, Querier, Storage};
    use cw_multi_test::{AppResponse, CosmosRouter, Module as ModuleTrait};
    use serde::de::DeserializeOwned;

    use crate::cosmwasm_ext::InterChainMsg;

    use super::InterChainMsgSender;

    pub struct Module {
        message_sender: InterChainMsgSender,
    }

    impl Module {
        pub fn new(message_sender: InterChainMsgSender) -> Self {
            Self { message_sender }
        }
    }

    impl ModuleTrait for Module {
        type ExecT = InterChainMsg;

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
            self.message_sender
                .send(msg)
                .expect("Receiver closed but message had to be sent!");

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
