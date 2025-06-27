use std::fmt::{Display, Formatter, Result as FmtResult};

use finance::duration::Duration;
use serde::{Deserialize, Serialize};

use dex::{Account, Connectable, ConnectionParams, Contract, IcaConnectee};
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::msg::ConfigResponse;

use super::{Config, ConfigManagement, IcaConnector, State, idle::Idle};

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct OpenIca {
    config: Config,
    dex: ConnectionParams,
}

impl OpenIca {
    pub fn new(config: Config, connection: ConnectionParams) -> Self {
        Self {
            config,
            dex: connection,
        }
    }
}

impl IcaConnectee for OpenIca {
    type State = State;
    type NextState = Idle;

    fn connected(self, account: Account) -> Self::NextState {
        Idle::new(self.config, account)
    }
}

impl Connectable for OpenIca {
    fn dex(&self) -> &ConnectionParams {
        &self.dex
    }
}

impl Contract for OpenIca {
    type StateResponse = ConfigResponse;

    fn state(
        self,
        _now: Timestamp,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        ConfigResponse {
            cadence_hours: self.config.cadence_hours(),
        }
    }
}

impl Display for OpenIca {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("Idle"))
    }
}

impl ConfigManagement for IcaConnector {
    fn load_config(&self) -> ContractResult<&Config> {
        Ok(&self.config)
    }
}
