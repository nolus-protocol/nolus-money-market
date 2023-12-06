use currency::Currency;
use sdk::cosmwasm_std::Addr;
use treasury::msg::InstantiateMsg;

use super::{cwcoin, dummy_query, native_cwcoin, test_case::app::App, CwContractWrapper, ADMIN};

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
            dummy_query,
        )
        .with_sudo(treasury::contract::sudo);

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
