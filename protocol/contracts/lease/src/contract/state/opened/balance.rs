use platform::bank;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    error::ContractResult,
    finance::{LpnCoinDTO, LpnCurrencies, LpnCurrency},
};

pub(super) fn lpn_balance(
    account: &Addr,
    querier: QuerierWrapper<'_>,
) -> ContractResult<LpnCoinDTO> {
    bank::balance::<LpnCurrency, LpnCurrencies>(account, querier)
        .map(Into::into)
        .map_err(Into::into)
}
