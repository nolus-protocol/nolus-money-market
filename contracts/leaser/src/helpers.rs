use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Empty, Querier, QuerierWrapper, StdResult, WasmMsg, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{ConfigResponse, ExecuteMsg, QueryMsg};

/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CwTemplateContract(pub Addr);

impl CwTemplateContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }

    /// Get config
    pub fn config<Q>(&self, querier: &Q) -> StdResult<ConfigResponse>
    where
        Q: Querier,
    {
        let msg = QueryMsg::Config {};
        let query = WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg: to_binary(&msg)?,
        }
        .into();
        let res: ConfigResponse = QuerierWrapper::<Empty>::new(querier).query(&query)?;
        Ok(res)
    }
}
