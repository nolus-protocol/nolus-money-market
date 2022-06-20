use crate::lpp::NTokenPrice;
use crate::{contract::NOLUS_DENOM, error::ContractError};
use cosmwasm_std::{
    coin, Addr, Coin, Decimal, DepsMut, Fraction, StdResult, Storage, Uint128,
};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

type Balance = Uint128;

#[derive(Debug)]
pub struct Deposit {
    addr: Addr,
    data: DepositData,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
struct DepositData {
    pub deposited_nlpn: Balance,

    // Rewards
    pub reward_per_token: Decimal,
    pub pending_rewards_nls: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
struct DepositsGlobals {
    pub balance_nlpn: Balance,

    // Rewards
    pub reward_per_token: Decimal,
}

impl Deposit {
    const DEPOSITS: Map<'static, Addr, DepositData> = Map::new("deposits");
    const GLOBALS: Item<'static, DepositsGlobals> = Item::new("deposits_globals");

    pub fn load(storage: &dyn Storage, addr: Addr) -> StdResult<Self> {
        let data = Self::DEPOSITS
            .may_load(storage, addr.clone())?
            .unwrap_or_default();

        Ok(Self { addr, data })
    }

    pub fn deposit(
        &mut self,
        storage: &mut dyn Storage,
        amount_lpn: Uint128,
        price: NTokenPrice,
    ) -> StdResult<()> {

        if amount_lpn.is_zero() {
            return Ok(())
        }

        let mut globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        self.update_rewards(&globals);

        let inv_price = price.get().inv().expect("price should not be zero");
        let deposited_nlpn = inv_price * amount_lpn;
        self.data.deposited_nlpn += deposited_nlpn;

        Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;

        globals.balance_nlpn += deposited_nlpn;

        Self::GLOBALS.save(storage, &globals)
    }

    /// return optional reward payment msg in case of deleting account
    pub fn withdraw(
        &mut self,
        storage: &mut dyn Storage,
        amount_nlpn: Uint128,
    ) -> Result<Option<Coin>, ContractError> {
        if self.data.deposited_nlpn < amount_nlpn {
            return Err(ContractError::InsufficientBalance);
        }

        let mut globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        self.update_rewards(&globals);

        self.data.deposited_nlpn -= amount_nlpn;
        globals.balance_nlpn -= amount_nlpn;

        let maybe_reward = if self.data.deposited_nlpn.is_zero() {
            self.update_rewards(&globals);
            let reward: Uint128 = self.data.pending_rewards_nls * Uint128::new(1);
            Self::DEPOSITS.remove(storage, self.addr.clone());
            Some(coin(reward.u128(), NOLUS_DENOM))
        } else {
            Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;
            None
        };

        Self::GLOBALS.save(storage, &globals)?;

        Ok(maybe_reward)
    }

    pub fn distribute_rewards(deps: DepsMut, rewards_nls: Coin) -> StdResult<()> {

        let mut globals = Self::GLOBALS.may_load(deps.storage)?.unwrap_or_default();

        // TODO: should we throw error in this case?
        if !globals.balance_nlpn.is_zero() {
            globals.reward_per_token += Decimal::from_ratio(
                rewards_nls.amount,
                globals.balance_nlpn,
            );

            Self::GLOBALS.save(deps.storage, &globals)
        } else {
            Ok(())
        }
    }

    fn update_rewards(&mut self, globals: &DepositsGlobals) {
        self.data.pending_rewards_nls = self.calculate_reward(globals);
        self.data.reward_per_token = globals.reward_per_token;
    }

    fn calculate_reward(&self, globals: &DepositsGlobals) -> Decimal {
        let deposit = &self.data;
        deposit.pending_rewards_nls
            + (globals.reward_per_token - deposit.reward_per_token)
                * Decimal::from_ratio(deposit.deposited_nlpn.u128(), 1u128)
    }

    /// query accounted rewards
    pub fn query_rewards(&self, storage: &dyn Storage) -> StdResult<Coin> {
        let globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();

        let reward = self.calculate_reward(&globals) * Uint128::new(1);
        Ok(coin(reward.u128(), NOLUS_DENOM))
    }

    /// pay accounted rewards to the deposit owner or optioanl recipient
    pub fn claim_rewards(
        &mut self,
        storage: &mut dyn Storage,
    ) -> StdResult<Coin> {

        let globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        self.update_rewards(&globals);
        let reward = self.data.pending_rewards_nls * Uint128::new(1);
        self.data.pending_rewards_nls -= Decimal::from_ratio(reward.u128(), 1u128);

        Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;

        Ok(coin(reward.u128(), NOLUS_DENOM))
    }

/*
    /// create `BankMsg` to send nolus tokens to `addr`
    pub fn pay_nls(addr: Addr, amount: Uint128) -> BankMsg {
        BankMsg::Send {
            to_address: addr.to_string(),
            amount: vec![coin(amount.u128(), NOLUS_DENOM)],
        }
    }
*/

    /// lpp derivative tokens balance
    pub fn balance_nlpn(storage: &dyn Storage) -> StdResult<Balance> {
        Ok(Self::GLOBALS
            .may_load(storage)?
            .unwrap_or_default()
            .balance_nlpn)
    }

    /// deposit derivative tokens balance
    pub fn query_balance_nlpn(storage: &dyn Storage, addr: Addr) -> StdResult<Option<Balance>> {
        let maybe_balance = Self::DEPOSITS
            .may_load(storage, addr)?
            .map(|data| data.deposited_nlpn);
        Ok(maybe_balance)
    }

    // pub fn balance_nls(deps: &Deps, env: &Env) -> StdResult<Uint128> {
    //     let querier = deps.querier;
    //     querier
    //         .query_balance(&env.contract.address, NOLUS_DENOM)
    //         .map(|coin| coin.amount)
    // }
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::testing;
    use crate::lpp::NTokenPrice;

    #[test]
    fn test_deposit_and_withdraw() {
        let mut deps = testing::mock_dependencies();
        let addr1 = Addr::unchecked("depositor1");
        let addr2 = Addr::unchecked("depositor2");
        let ndenom = "nusdc".to_string();
        let price = NTokenPrice::mock(Decimal::one(), &ndenom);

        let mut deposit1 = Deposit::load(deps.as_ref().storage, addr1.clone())
            .expect("should load");
        deposit1.deposit(deps.as_mut().storage, 1000u128.into(), price)
            .expect("should deposit");

        Deposit::distribute_rewards(deps.as_mut(), coin(1000, "unolus"))
            .expect("should distribute rewards");

        let price = NTokenPrice::mock(Decimal::percent(200), &ndenom);
        let mut deposit2 = Deposit::load(deps.as_ref().storage, addr2.clone())
            .expect("should load");
        deposit2.deposit(deps.as_mut().storage, 1000u128.into(), price)
            .expect("should deposit");

        let balance_nlpn = Deposit::balance_nlpn(deps.as_ref().storage)
            .expect("should query balance_nlpn");
        assert_eq!(balance_nlpn, 1500u128.into());

        let balance2 = Deposit::query_balance_nlpn(deps.as_ref().storage, addr2)
            .expect("should query deposit balance_nlpn")
            .expect("should be some balance");
        assert_eq!(balance2, 500u128.into());

        let reward = deposit1.query_rewards(deps.as_ref().storage)
            .expect("should query rewards");

        assert_eq!(reward.amount, 1000u128.into());

        let reward = deposit2.query_rewards(deps.as_ref().storage)
            .expect("should query rewards");

        assert_eq!(reward.amount, 0u128.into());


        Deposit::distribute_rewards(deps.as_mut(), coin(1500, "unolus"))
            .expect("should distribute rewards");

        let reward = deposit1.query_rewards(deps.as_ref().storage)
            .expect("should query rewards");

        assert_eq!(reward.amount, 2000u128.into());

        let reward = deposit2.query_rewards(deps.as_ref().storage)
            .expect("should query rewards");

        assert_eq!(reward.amount, 500u128.into());

        let some_rewards = deposit1.withdraw(deps.as_mut().storage, 500u128.into())
            .expect("should withdraw");
        assert!(some_rewards.is_none());

        let amount = deposit1.claim_rewards(deps.as_mut().storage)
            .expect("should claim rewards");
        assert_eq!(amount, coin(2000, NOLUS_DENOM));

        let amount = deposit2.claim_rewards(deps.as_mut().storage)
            .expect("should claim rewards");
        assert_eq!(amount, coin(500, NOLUS_DENOM));


        Deposit::distribute_rewards(deps.as_mut(), coin(1000, "unolus"))
            .expect("should distribute rewards");

        let reward = deposit1.query_rewards(deps.as_ref().storage)
            .expect("should query rewards");

        assert_eq!(reward.amount, 500u128.into());

        let reward = deposit2.query_rewards(deps.as_ref().storage)
            .expect("should query rewards");

        assert_eq!(reward.amount, 500u128.into());

        // withdraw all, return rewards, close deposit
        let rewards = deposit1.withdraw(deps.as_mut().storage, 500u128.into())
            .expect("should withdraw")
            .expect("should be some rewards");
        assert_eq!(rewards, coin(500, NOLUS_DENOM));
        let response = Deposit::query_balance_nlpn(deps.as_mut().storage, addr1)
            .expect("should query");
        assert!(response.is_none());



    }
}
