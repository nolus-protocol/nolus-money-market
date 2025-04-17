use platform::bank;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    error::ContractResult,
    finance::{LpnCoinDTO, LpnCurrency},
};

pub(super) fn lpn_balance(
    account: &Addr,
    querier: QuerierWrapper<'_>,
) -> ContractResult<LpnCoinDTO> {
    bank::balance::<LpnCurrency>(account, querier)
        .map(Into::into)
        .map_err(Into::into)
}
