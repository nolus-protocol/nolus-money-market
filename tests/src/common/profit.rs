use dex::{ConnectionParams, Ics20Channel};
use profit::{
    contract::{execute, instantiate, query, reply, sudo},
    msg::InstantiateMsg,
    typedefs::CadenceHours,
};
use sdk::{cosmwasm_std::Addr, testing};

use crate::common::test_case::response::RemoteChain;

use super::{
    ADMIN, CwContractWrapper,
    test_case::{TestCase, app::App},
};

pub(crate) struct Instantiator;

impl Instantiator {
    #[track_caller]
    pub fn instantiate(
        app: &mut App,
        cadence_hours: CadenceHours,
        treasury: Addr,
        oracle: Addr,
        timealarms: Addr,
    ) -> Addr {
        // TODO [Rust 1.70] Convert to static item with OnceCell
        let endpoints = CwContractWrapper::new(execute, instantiate, query)
            .with_reply(reply)
            .with_sudo(sudo);

        let code_id = app.store_code(Box::new(endpoints));

        let msg = InstantiateMsg {
            cadence_hours,
            treasury,
            oracle,
            timealarms,
            dex: ConnectionParams {
                connection_id: TestCase::DEX_CONNECTION_ID.into(),
                transfer_channel: Ics20Channel {
                    local_endpoint: TestCase::PROFIT_IBC_CHANNEL.into(),
                    remote_endpoint: "channel-262".into(),
                },
            },
        };

        app.instantiate(code_id, testing::user(ADMIN), &msg, &[], "profit", None)
            .map(|mut response| {
                response.expect_register_ica(TestCase::DEX_CONNECTION_ID, TestCase::PROFIT_ICA_ID);
                response.unwrap_response()
            })
            .unwrap()
    }
}
