use sdk::cosmwasm_std::Addr;
use timealarms::{
    contract::{execute, instantiate, reply},
    msg::InstantiateMsg,
};

use crate::common::test_case::app::App;

use super::{mock_query, CwContractWrapper, ADMIN};

pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate(app: &mut App) -> Addr {
        let endpoints: CwContractWrapper<_, _, _, _, _, _, _, _, _, _, _> =
            CwContractWrapper::new(execute, instantiate, mock_query).with_reply(reply);

        let code_id = app.store_code(Box::new(endpoints));

        let msg = InstantiateMsg {};

        app.instantiate(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[],
            "timealarms",
            None,
        )
        .unwrap()
        .unwrap_response()
    }
}
