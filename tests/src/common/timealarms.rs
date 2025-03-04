use sdk::{cosmwasm_std::Addr, testing};
use timealarms::{
    contract::{execute, instantiate, reply},
    msg::InstantiateMsg,
};

use super::{ADMIN, CwContractWrapper, dummy_query, test_case::app::App};

pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate(app: &mut App) -> Addr {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints = CwContractWrapper::new(execute, instantiate, dummy_query).with_reply(reply);

        let code_id = app.store_code(Box::new(endpoints));

        let msg = InstantiateMsg {};

        app.instantiate(code_id, testing::user(ADMIN), &msg, &[], "timealarms", None)
            .unwrap()
            .unwrap_response()
    }
}
