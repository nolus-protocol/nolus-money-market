use finance::percent::Percent;
use rewards_dispatcher::{
    msg::{Dex, InstantiateMsg},
    state::reward_scale::{Bar, RewardScale, TotalValueLocked},
};
use sdk::cosmwasm_std::Addr;

use super::{test_case::app::App, CwContractWrapper, ADMIN};

pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate(
        app: &mut App,
        lpp: Addr,
        oracle: Addr,
        timealarms: Addr,
        treasury: Addr,
    ) -> Addr {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints = CwContractWrapper::new(
            rewards_dispatcher::contract::execute,
            rewards_dispatcher::contract::instantiate,
            rewards_dispatcher::contract::query,
        )
        .with_sudo(rewards_dispatcher::contract::sudo);

        let code_id = app.store_code(Box::new(endpoints));

        let msg = InstantiateMsg {
            cadence_hours: 10,
            dex: Dex { lpp, oracle },
            timealarms,
            treasury,
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

        app.instantiate(
            code_id,
            Addr::unchecked(ADMIN),
            &msg,
            &[],
            "dispatcher",
            None,
        )
        .unwrap()
        .unwrap_response()
    }
}
