use serde::{Deserialize, Serialize};

use dex::{ContinueResult, Handler, Ics20Channel, Response as DexResponse};
use oracle::stub::OracleRef;
use platform::message::Response as PlatformResponse;
use sdk::cosmwasm_std::{Deps, Env};
use timealarms::stub::TimeAlarmsRef;

use crate::{msg::ConfigResponse, result::ContractResult};

use super::{
    open_ica::OpenIca, Config, ConfigManagement, IcaConnector, ProfitMessageHandler, State,
};

#[derive(Serialize, Deserialize)]
pub(super) struct OpenTransferChannel {
    config: Config,
    connection_id: String,
    oracle: OracleRef,
    time_alarms: TimeAlarmsRef,
}

impl OpenTransferChannel {
    pub fn new(
        config: Config,
        connection_id: String,
        oracle: OracleRef,
        time_alarms: TimeAlarmsRef,
    ) -> Self {
        Self {
            config,
            connection_id,
            oracle,
            time_alarms,
        }
    }
}

impl Handler for OpenTransferChannel {
    type Response = State;
    type SwapResult = DexResponse<State>;
}

impl ConfigManagement for OpenTransferChannel {
    fn try_update_config(self, cadence_hours: u16) -> ContractResult<Self> {
        Ok(Self {
            config: self.config.update(cadence_hours),
            ..self
        })
    }

    fn try_query_config(&self) -> ContractResult<ConfigResponse> {
        Ok(ConfigResponse {
            cadence_hours: self.config.cadence_hours(),
        })
    }
}

impl ProfitMessageHandler for OpenTransferChannel {
    fn confirm_open(
        self,
        _: Deps<'_>,
        _: Env,
        transfer_channel: Ics20Channel,
        _: String,
    ) -> ContinueResult<Self> {
        let ica_connector: IcaConnector = IcaConnector::new(OpenIca::new(
            self.config,
            self.connection_id,
            self.oracle,
            self.time_alarms,
            transfer_channel,
        ));

        Ok(DexResponse::<Self> {
            response: PlatformResponse::messages_only(ica_connector.enter()),
            next_state: ica_connector.into(),
        })
    }
}
