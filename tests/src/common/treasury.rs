use currency::Currency;
use sdk::cosmwasm_std::Addr;
use treasury::{contract::sudo, msg::InstantiateMsg};

use crate::common::test_case::app::App;

use super::{cwcoin, mock_query, native_cwcoin, CwContractWrapper, ADMIN};

pub(crate) struct Instantiator {
    rewards_dispatcher: Addr,
}

impl Instantiator {
    pub fn new(rewards_dispatcher: Addr) -> Self {
        Self { rewards_dispatcher }
    }

    pub fn new_with_no_dispatcher() -> Self {
        Self::new(Addr::unchecked("DEADCODE"))
    }

    #[track_caller]
    pub fn instantiate<Lpn>(self, app: &mut App) -> Addr
    where
        Lpn: Currency,
    {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints = CwContractWrapper::new(
            treasury::contract::execute,
            treasury::contract::instantiate,
            mock_query,
        )
        .with_sudo(sudo);

        let code_id = app.store_code(Box::new(endpoints));

        let msg = InstantiateMsg {
            rewards_dispatcher: self.rewards_dispatcher,
        };

        app.instantiate(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[cwcoin::<Lpn, _>(1000), native_cwcoin(1000)],
            "treasury",
            None,
        )
        .unwrap()
        .unwrap_response()
    }
}
