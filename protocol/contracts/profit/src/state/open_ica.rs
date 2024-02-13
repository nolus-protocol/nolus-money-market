use serde::{Deserialize, Serialize};

use std::fmt::{Display, Formatter, Result as FmtResult};

use dex::{Account, ConnectionParams, DexConnectable, IcaConnectee};

use crate::{error::ContractError, msg::ConfigResponse, result::ContractResult};

use super::{idle::Idle, Config, ConfigManagement, IcaConnector, State};

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct OpenIca {
    config: Config,
    dex: ConnectionParams,
}

impl OpenIca {
    const QUERY_ERROR: &'static str =
        "Configuration querying is not supported while opening interchain account!";

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

impl DexConnectable for OpenIca {
    fn dex(&self) -> &ConnectionParams {
        &self.dex
    }
}

impl Display for OpenIca {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("Idle"))
    }
}

impl ConfigManagement for IcaConnector {
    fn try_query_config(&self) -> ContractResult<ConfigResponse> {
        Err(ContractError::unsupported_operation(OpenIca::QUERY_ERROR))
    }
}
