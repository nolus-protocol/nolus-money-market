use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

use dex::{ConnectionParams, Handler, Response as DexResponse};
use platform::{
    message::Response as PlatformResponse, state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Deps, Env};

use crate::{msg::ConfigResponse, result::ContractResult};

use super::{
    open_ica::OpenIca, Config, ConfigAndResponse, ConfigManagement, IcaConnector, SetupDexHandler,
    State, StateAndResponse,
};

#[derive(Serialize, Deserialize)]
pub(super) struct OpenTransferChannel {
    config: Config,
}

impl OpenTransferChannel {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl Handler for OpenTransferChannel {
    type Response = State;
    type SwapResult = ContractResult<DexResponse<State>>;
}

impl ConfigManagement for OpenTransferChannel {
    fn with_config<F>(self, f: F) -> ContractResult<StateAndResponse<Self>>
    where
        F: FnOnce(Config) -> ContractResult<ConfigAndResponse>,
    {
        f(self.config).map(|ConfigAndResponse { config, response }| StateAndResponse {
            state: Self { config },
            response,
        })
    }

    fn try_query_config(&self) -> ContractResult<ConfigResponse> {
        Ok(ConfigResponse {
            cadence_hours: self.config.cadence_hours(),
        })
    }
}

impl SetupDexHandler for OpenTransferChannel {
    type State = IcaConnector;

    fn setup_dex(
        self,
        _: Deps<'_>,
        _: Env,
        connection: ConnectionParams,
    ) -> ContractResult<StateMachineResponse<Self::State>> {
        let ica_connector: IcaConnector = IcaConnector::new(OpenIca::new(self.config, connection));

        Ok(StateMachineResponse {
            response: PlatformResponse::messages_only(ica_connector.enter()),
            next_state: ica_connector,
        })
    }
}

impl Display for OpenTransferChannel {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("OpenTransferChannel"))
    }
}
