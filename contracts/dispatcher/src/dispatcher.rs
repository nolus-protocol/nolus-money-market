use cosmwasm_std::{to_binary, StdError, Uint128};
use cosmwasm_std::{Deps, QueryRequest, StdResult, WasmQuery};
use lpp::msg::{BalanceResponse, QueryMsg as LPPQueryMsg};

use crate::state::config::{Config, TvlApr};

pub(crate) fn _get_lpp_balance(deps: Deps, config: Config) -> StdResult<Uint128> {
    let query_msg: LPPQueryMsg = LPPQueryMsg::Balance {
        address: config.lpp.clone(),
    };
    let resp: BalanceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.lpp.to_string(),
        msg: to_binary(&query_msg)?,
    }))?;
    Ok(resp.balance)
}

pub(crate) fn _determine_apr(mut tvl_tp_arp: Vec<TvlApr>, lpp_balance: u128) -> StdResult<u32> {
    tvl_tp_arp.sort_by(|a, b| a.tvl.cmp(&b.tvl));

    let idx = match tvl_tp_arp.binary_search(&TvlApr::new(lpp_balance as u32, 0)) {
        Ok(i) => i,
        Err(e) => e,
    };
    let arp = match tvl_tp_arp.get(idx) {
        Some(tvl) => tvl.apr,
        None => return Err(StdError::generic_err("ARP not found")),
    };

    Ok(arp)
}
