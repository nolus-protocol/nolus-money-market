use currency::{CurrencyDef, Group};
use finance::coin::{Coin, WithCoin};
use sdk::cosmwasm_std::Coin as CwCoin;

use crate::{coin_legacy, error::Error, result::Result};

/// Ensure a single coin of the specified currency is received by a contract and return it
pub fn received_one<C>(cw_amount: &[CwCoin]) -> Result<Coin<C>>
where
    C: CurrencyDef,
{
    received_one_impl(
        cw_amount,
        || Error::no_funds::<C>(),
        || Error::unexpected_funds::<C>(),
    )
    .and_then(coin_legacy::from_cosmwasm::<C>)
}

/// Run a command on the first coin of the specified group
pub fn may_received<VisitedG, V>(cw_amount: &Vec<CwCoin>, mut cmd: V) -> Option<V::Outcome>
where
    VisitedG: Group,
    V: WithCoin<VisitedG>,
{
    let mut may_res = None;

    for coin in cw_amount {
        cmd = match coin_legacy::from_cosmwasm_seek_any(coin, cmd) {
            Ok(res) => {
                may_res = Some(res);

                break;
            }
            Err(cmd) => cmd,
        };
    }

    may_res
}

fn received_one_impl<NoFundsErr, UnexpFundsErr>(
    cw_amount: &[CwCoin],
    no_funds_err: NoFundsErr,
    unexp_funds_err: UnexpFundsErr,
) -> Result<&CwCoin>
where
    NoFundsErr: FnOnce() -> Error,
    UnexpFundsErr: FnOnce() -> Error,
{
    match cw_amount.len() {
        0 => Err(no_funds_err()),
        1 => Ok(cw_amount.iter().next().expect("there is at least a coin")),
        _ => Err(unexp_funds_err()),
    }
}
