use sdk::{cosmwasm_std::Addr, testing};
use timealarms::{contract, msg::InstantiateMsg};

use super::{ADMIN, CwContractWrapper, test_case::app::App};

pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate(app: &mut App) -> Addr {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints =
            CwContractWrapper::new(contract::execute, contract::instantiate, super::dummy_query)
                .with_reply(contract::reply);

        let code_id = app.store_code(Box::new(endpoints));

        let msg = InstantiateMsg {};

        app.instantiate(code_id, testing::user(ADMIN), &msg, &[], "timealarms", None)
            .unwrap()
            .unwrap_response()
    }
}
