use cosmwasm_std::{Uint128, Fraction, Addr, Storage, StdResult};
use cw_storage_plus::{Map, Item};
use crate::error::ContractError;
use crate::lpp::NTokenPrice;

type Balance = Uint128;

pub struct Deposit {
    addr: Addr,
    acc_balance: Balance,
}

impl Deposit {
    const DEPOSITS: Map<'static, Addr, Balance> = Map::new("deposits");
    const BALANCE: Item<'static, Balance> = Item::new("deposits_balance");

    pub fn load(storage: &dyn Storage, addr: Addr) -> StdResult<Self> {

        let acc_balance = Self::DEPOSITS.may_load(storage, addr.clone())?
            .unwrap_or_default();

        Ok(Self {
            addr,
            acc_balance,
        })
    }

    pub fn deposit(&mut self, storage: &mut dyn Storage, amount_lnp: Uint128, price: NTokenPrice) -> StdResult<()> {
        let inv_price = price.get().inv()
            .expect("price should not be zero");
        let deposited_nlnp = inv_price*amount_lnp;
        self.acc_balance += deposited_nlnp;

        Self::DEPOSITS.save(storage, self.addr.clone(), &self.acc_balance)?;

        Self::BALANCE.update(storage, |balance| -> StdResult<Balance> { Ok(balance + deposited_nlnp) })?;

        Ok(())
    }

    pub fn withdraw(&mut self, storage: &mut dyn Storage, amount_nlnp: Uint128) -> Result<(), ContractError> {

        if self.acc_balance < amount_nlnp {
            return Err(ContractError::InsufficientBalance);
        }

        Self::BALANCE.update(storage, |balance| -> StdResult<Balance> { Ok(balance - amount_nlnp) })?;

        self.acc_balance -= amount_nlnp;

        if self.acc_balance.is_zero() {
            Self::DEPOSITS.remove(storage, self.addr.clone())
        } else {
            Self::DEPOSITS.save(storage, self.addr.clone(), &self.acc_balance)?
        }

        Ok(())
    }

    pub fn balance(storage: &dyn Storage) -> StdResult<Balance> {
        Ok(Self::BALANCE.may_load(storage)?
            .unwrap_or_default())
    }

    pub fn query_balance(storage: &dyn Storage, addr: Addr) -> StdResult<Option<Balance>> {
        Self::DEPOSITS.may_load(storage, addr)
    }
}
