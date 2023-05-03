use serde::{Deserialize, Serialize};

use dex::{Account, ConnectionParams, DexConnectable, IcaConnectee, Ics20Channel};
use oracle::stub::OracleRef;
use timealarms::stub::TimeAlarmsRef;

use crate::{error::ContractError, msg::ConfigResponse, result::ContractResult};

use super::{idle::Idle, Config, ConfigManagement, IcaConnector, ProfitMessageHandler, State};

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct OpenIca {
    config: Config,
    dex: ConnectionParams,
    oracle: OracleRef,
    time_alarms: TimeAlarmsRef,
}

impl OpenIca {
    pub fn new(
        config: Config,
        connection_id: String,
        oracle: OracleRef,
        time_alarms: TimeAlarmsRef,
        transfer_channel: Ics20Channel,
    ) -> Self {
        Self {
            config,
            dex: ConnectionParams {
                connection_id,
                transfer_channel,
            },
            oracle,
            time_alarms,
        }
    }
}

impl IcaConnectee for OpenIca {
    type State = State;
    type NextState = Idle;

    fn connected(self, account: Account) -> Self::NextState {
        Idle::new(self.config, account, self.oracle, self.time_alarms)
    }
}

impl DexConnectable for OpenIca {
    fn dex(&self) -> &ConnectionParams {
        &self.dex
    }
}

impl ConfigManagement for IcaConnector {
    fn try_update_config(self, _: u16) -> ContractResult<Self> {
        Err(ContractError::UnsupportedOperation(String::from(
            "Configuration changes are not allowed during ICA opening process.",
        )))
    }

    fn try_query_config(&self) -> ContractResult<ConfigResponse> {
        Err(ContractError::UnsupportedOperation(String::from(
            "Querying configuration is not allowed during ICA opening process.",
        )))
    }
}

impl ProfitMessageHandler for IcaConnector {}
