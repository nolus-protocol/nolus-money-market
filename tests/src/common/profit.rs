use profit::{
    contract::{execute, instantiate, query, sudo},
    msg::InstantiateMsg,
    typedefs::CadenceHours,
};
use sdk::cosmwasm_std::Addr;

use super::{
    test_case::{app::App, wasm::Wasm as WasmTrait},
    CwContractWrapper, ADMIN,
};

pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate<Wasm>(
        app: &mut App<Wasm>,
        cadence_hours: CadenceHours,
        treasury: Addr,
        oracle: Addr,
        timealarms: Addr,
    ) -> Addr
    where
        Wasm: WasmTrait,
    {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints = CwContractWrapper::new(execute, instantiate, query).with_sudo(sudo);

        let code_id = app.store_code(Box::new(endpoints));

        let msg = InstantiateMsg {
            cadence_hours,
            treasury,
            oracle,
            timealarms,
        };

        app.instantiate(code_id, Addr::unchecked(ADMIN), &msg, &[], "profit", None)
            .unwrap()
            .unwrap_response()
    }
}
