use platform::contract::{Code, CodeId};
use reserve::{
    api::InstantiateMsg,
    contract,
};
use sdk::{cosmwasm_std::Addr, testing};

use super::{
    ADMIN, CwContractWrapper, leaser::Instantiator as LeaserInstantiator, test_case::app::App,
};

pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate(app: &mut App, lease_code: Code) -> Addr {
        let endpoints = CwContractWrapper::new(contract::execute, contract::instantiate, contract::query);

        let code_id = app.store_code(Box::new(endpoints));
        let protocol_admin = LeaserInstantiator::expected_addr().into(); //the Leaser address

        let msg = InstantiateMsg {
            protocol_admin,
            lease_code: CodeId::from(lease_code).into(),
        };

        app.instantiate(code_id, testing::user(ADMIN), &msg, &[], "reserve", None)
            .map(|response| response.unwrap_response())
            .unwrap()
    }
}
