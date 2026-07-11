use profit::{CadenceHours, contract, msg::InstantiateMsg};
use sdk::{cosmwasm_std::Addr, testing};

use super::{ADMIN, CwContractWrapper, test_case::app::App};

pub(crate) const SETTLEMENT: &str = "profit_settlement";

pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate(
        app: &mut App,
        cadence_hours: CadenceHours,
        settlement: Addr,
        timealarms: Addr,
    ) -> Addr {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints =
            CwContractWrapper::new(contract::execute, contract::instantiate, contract::query);

        let code_id = app.store_code(Box::new(endpoints));

        let msg = InstantiateMsg {
            cadence_hours,
            settlement,
            timealarms,
        };

        app.instantiate(code_id, testing::user(ADMIN), &msg, &[], "profit", None)
            .unwrap()
            .unwrap_response()
    }
}
