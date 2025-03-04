use finance::percent::Percent;
use sdk::{cosmwasm_std::Addr, testing};
use treasury::{
    msg::InstantiateMsg,
    state::reward_scale::{Bar, RewardScale, TotalValueLocked},
};

use super::{ADMIN, CwContractWrapper, test_case::app::App};

#[derive(Default)]
pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate(app: &mut App, protocols_registry: Addr, timealarms: Addr) -> Addr {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints = CwContractWrapper::new(
            treasury::contract::execute,
            treasury::contract::instantiate,
            treasury::contract::query,
        )
        .with_sudo(treasury::contract::sudo);

        let code_id = app.store_code(Box::new(endpoints));

        let msg = InstantiateMsg {
            cadence_hours: 10,
            protocols_registry,
            timealarms,
            tvl_to_apr: RewardScale::try_from(vec![
                Bar {
                    tvl: Default::default(),
                    apr: Percent::from_permille(10),
                },
                Bar {
                    tvl: TotalValueLocked::new(1000),
                    apr: Percent::from_permille(10),
                },
            ])
            .unwrap(),
        };

        app.instantiate(code_id, testing::user(ADMIN), &msg, &[], "dispatcher", None)
            .unwrap()
            .unwrap_response()
    }
}
