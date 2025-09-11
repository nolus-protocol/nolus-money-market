use std::marker::PhantomData;

use currency::{CurrencyDTO, CurrencyDef, FilterMapT, Group, MemberOf};
use finance::coin::WithCoin;
use sdk::cosmwasm_std::Uint128;

use crate::{bank::Aggregate, coin_legacy, result::Result};

use super::BankView;

pub(super) struct NonZeroBalances<'addr, 'view, GBalances, CmdBalances> {
    _g: PhantomData<GBalances>,
    view: &'view BankView<'addr>,
    cmd: CmdBalances,
}

impl<'addr, 'view, GBalances, CmdBalances> NonZeroBalances<'addr, 'view, GBalances, CmdBalances> {
    pub fn new(view: &'view BankView<'addr>, cmd: CmdBalances) -> Self {
        Self {
            _g: PhantomData,
            view,
            cmd,
        }
    }
}

impl<'addr, 'view, GBalances, CmdBalances> Clone
    for NonZeroBalances<'addr, 'view, GBalances, CmdBalances>
where
    CmdBalances: Clone,
{
    fn clone(&self) -> Self {
        Self {
            _g: self._g,
            view: self.view,
            cmd: self.cmd.clone(),
        }
    }
}

impl<'addr, 'view, GBalances, CmdBalances> FilterMapT<GBalances>
    for NonZeroBalances<'addr, 'view, GBalances, CmdBalances>
where
    GBalances: Group,
    CmdBalances: WithCoin<GBalances> + Clone,
    CmdBalances::Outcome: Aggregate,
{
    type Outcome = Result<<CmdBalances as WithCoin<GBalances>>::Outcome>;

    fn on<C>(&self, def: &CurrencyDTO<C::Group>) -> Option<Self::Outcome>
    where
        C: CurrencyDef + currency::PairsGroup<CommonGroup = <GBalances as Group>::TopG>,
        C::Group: MemberOf<GBalances> + currency::MemberOf<<GBalances as Group>::TopG>,
    {
        self.view.cw_balance(def).map_or_else(
            |err| Some(Err(err)),
            |ref cw_balance| {
                (cw_balance.amount != Uint128::zero()).then(|| {
                    coin_legacy::from_cosmwasm_any::<GBalances, _>(cw_balance, self.cmd.clone())
                })
            },
        )
    }
}
