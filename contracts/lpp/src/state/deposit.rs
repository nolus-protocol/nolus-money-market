use crate::error::ContractError;
use crate::lpp::NTokenPrice;
use crate::nlpn::NLpn;
use cosmwasm_std::{Addr, DepsMut, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use finance::coin::Coin;
use finance::currency::{Currency, Nls};
use finance::price::{self, Price};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug)]
pub struct Deposit {
    addr: Addr,
    data: DepositData,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
struct DepositData {
    pub deposited_nlpn: Coin<NLpn>,

    // Rewards
    pub reward_per_token: Option<Price<NLpn, Nls>>,
    pub pending_rewards_nls: Coin<Nls>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
struct DepositsGlobals {
    pub balance_nlpn: Coin<NLpn>,

    // Rewards
    pub reward_per_token: Option<Price<NLpn, Nls>>,
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

    pub fn deposit<LPN>(
        &mut self,
        storage: &mut dyn Storage,
        amount_lpn: Coin<LPN>,
        price: NTokenPrice<LPN>,
    ) -> Result<Coin<NLpn>,ContractError>
    where
        LPN: Currency + Serialize + DeserializeOwned,
    {
        if amount_lpn.is_zero() {
            return Err(ContractError::NoDeposit);
        }

        let mut globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        self.update_rewards(&globals);

        let deposited_nlpn = price::total(amount_lpn, price.get().inv());
        self.data.deposited_nlpn = self.data.deposited_nlpn + deposited_nlpn;

        Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;

        globals.balance_nlpn = globals.balance_nlpn + deposited_nlpn;

        Self::GLOBALS.save(storage, &globals)?;

        Ok(deposited_nlpn)
    }

    /// return optional reward payment msg in case of deleting account
    pub fn withdraw(
        &mut self,
        storage: &mut dyn Storage,
        amount_nlpn: Coin<NLpn>,
    ) -> Result<Option<Coin<Nls>>, ContractError> {
        if self.data.deposited_nlpn < amount_nlpn {
            return Err(ContractError::InsufficientBalance);
        }

        let mut globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        self.update_rewards(&globals);

        self.data.deposited_nlpn = self.data.deposited_nlpn - amount_nlpn;
        globals.balance_nlpn = globals.balance_nlpn - amount_nlpn;

        let maybe_reward = if self.data.deposited_nlpn.is_zero() {
            Self::DEPOSITS.remove(storage, self.addr.clone());
            if self.data.pending_rewards_nls.is_zero() {
                None
            } else {
                Some(self.data.pending_rewards_nls)
            }
        } else {
            Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;
            None
        };

        Self::GLOBALS.save(storage, &globals)?;

        Ok(maybe_reward)
    }

    pub fn distribute_rewards(deps: DepsMut, rewards: Coin<Nls>) -> StdResult<()> {
        let mut globals = Self::GLOBALS.may_load(deps.storage)?.unwrap_or_default();

        if !globals.balance_nlpn.is_zero() && !rewards.is_zero() {
            let partial_price = price::total_of(globals.balance_nlpn).is(rewards);

            if let Some(ref mut reward_per_token) = globals.reward_per_token {
                *reward_per_token = reward_per_token.lossy_add(partial_price);
            } else {
                globals.reward_per_token = Some(partial_price);
            }

            Self::GLOBALS.save(deps.storage, &globals)
        } else {
            Ok(())
        }
    }

    fn update_rewards(&mut self, globals: &DepositsGlobals) {
        self.data.pending_rewards_nls = self.calculate_reward(globals);
        self.data.reward_per_token = globals.reward_per_token;
    }

    fn calculate_reward(&self, globals: &DepositsGlobals) -> Coin<Nls> {
        let deposit = &self.data;

        let global_reward = globals.reward_per_token.map(|price| 
            price::total(deposit.deposited_nlpn, price)
        ).unwrap_or_default();

        let deposit_reward = deposit.reward_per_token.map(|price| 
            price::total(deposit.deposited_nlpn, price)
        ).unwrap_or_default();

        deposit.pending_rewards_nls + global_reward - deposit_reward
    }

    /// query accounted rewards
    pub fn query_rewards(&self, storage: &dyn Storage) -> StdResult<Coin<Nls>> {
        let globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        Ok(self.calculate_reward(&globals))
    }

    /// pay accounted rewards to the deposit owner or optional recipient
    pub fn claim_rewards(&mut self, storage: &mut dyn Storage) -> StdResult<Coin<Nls>> {
        let globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        self.update_rewards(&globals);

        let reward = self.data.pending_rewards_nls;
        self.data.pending_rewards_nls = Coin::new(0);

        Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;

        Ok(reward)
    }

    /// lpp derivative tokens balance
    pub fn balance_nlpn(storage: &dyn Storage) -> StdResult<Coin<NLpn>> {
        Ok(Self::GLOBALS
            .may_load(storage)?
            .unwrap_or_default()
            .balance_nlpn)
    }

    /// deposit derivative tokens balance
    pub fn query_balance_nlpn(storage: &dyn Storage, addr: Addr) -> StdResult<Option<Coin<NLpn>>> {
        let maybe_balance = Self::DEPOSITS
            .may_load(storage, addr)?
            .map(|data| data.deposited_nlpn);
        Ok(maybe_balance)
    }

}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lpp::NTokenPrice;
    use cosmwasm_std::testing;
    use finance::currency::Usdc;

    type TheCurrency = Usdc;

    #[test]
    fn test_deposit_and_withdraw() {
        let mut deps = testing::mock_dependencies();
        let addr1 = Addr::unchecked("depositor1");
        let addr2 = Addr::unchecked("depositor2");
        let price = NTokenPrice::<TheCurrency>::mock(Coin::new(1), Coin::new(1));

        let mut deposit1 =
            Deposit::load(deps.as_ref().storage, addr1.clone()).expect("should load");
        deposit1
            .deposit(deps.as_mut().storage, 1000u128.into(), price)
            .expect("should deposit");

        Deposit::distribute_rewards(deps.as_mut(), Coin::new(1000))
            .expect("should distribute rewards");

        let price = NTokenPrice::<TheCurrency>::mock(Coin::new(1), Coin::new(2));
        let mut deposit2 =
            Deposit::load(deps.as_ref().storage, addr2.clone()).expect("should load");
        deposit2
            .deposit(deps.as_mut().storage, 1000u128.into(), price)
            .expect("should deposit");

        let balance_nlpn =
            Deposit::balance_nlpn(deps.as_ref().storage).expect("should query balance_nlpn");
        assert_eq!(balance_nlpn, 1500u128.into());

        let balance2 = Deposit::query_balance_nlpn(deps.as_ref().storage, addr2)
            .expect("should query deposit balance_nlpn")
            .expect("should be some balance");
        assert_eq!(balance2, 500u128.into());

        let reward = deposit1
            .query_rewards(deps.as_ref().storage)
            .expect("should query rewards");

        assert_eq!(reward, Coin::new(1000));

        let reward = deposit2
            .query_rewards(deps.as_ref().storage)
            .expect("should query rewards");

        assert_eq!(reward, Coin::new(0));

        Deposit::distribute_rewards(deps.as_mut(), Coin::new(1500))
            .expect("should distribute rewards");

        let reward = deposit1
            .query_rewards(deps.as_ref().storage)
            .expect("should query rewards");

        assert_eq!(reward, Coin::new(2000));

        let reward = deposit2
            .query_rewards(deps.as_ref().storage)
            .expect("should query rewards");

        assert_eq!(reward, Coin::new(500));

        let some_rewards = deposit1
            .withdraw(deps.as_mut().storage, 500u128.into())
            .expect("should withdraw");
        assert!(some_rewards.is_none());

        let amount = deposit1
            .claim_rewards(deps.as_mut().storage)
            .expect("should claim rewards");
        assert_eq!(amount, Coin::<Nls>::new(2000));

        let amount = deposit2
            .claim_rewards(deps.as_mut().storage)
            .expect("should claim rewards");
        assert_eq!(amount, Coin::<Nls>::new(500));

        Deposit::distribute_rewards(deps.as_mut(), Coin::new(1000))
            .expect("should distribute rewards");

        let reward = deposit1
            .query_rewards(deps.as_ref().storage)
            .expect("should query rewards");

        assert_eq!(reward, Coin::new(500));

        let reward = deposit2
            .query_rewards(deps.as_ref().storage)
            .expect("should query rewards");

        assert_eq!(reward, Coin::new(500));

        // withdraw all, return rewards, close deposit
        let rewards = deposit1
            .withdraw(deps.as_mut().storage, 500u128.into())
            .expect("should withdraw")
            .expect("should be some rewards");
        assert_eq!(rewards, Coin::<Nls>::new(500));
        let response =
            Deposit::query_balance_nlpn(deps.as_mut().storage, addr1).expect("should query");
        assert!(response.is_none());
    }

    #[test]
    fn test_rewards_zero_balance() {
        let mut deps = testing::mock_dependencies();
        let price = NTokenPrice::<TheCurrency>::mock(Coin::new(1), Coin::new(1));
        let addr = Addr::unchecked("depositor");

        let mut deposit =
            Deposit::load(deps.as_ref().storage, addr)
            .expect("should load");

        // balance_nls = 0, balance_nlpn = 0
        let rewards = deposit.query_rewards(deps.as_ref().storage)
            .expect("should query");
        assert_eq!(Coin::<Nls>::new(0), rewards);

        // balance_nls = 0, balance_nlpn != 0
        deposit.deposit(deps.as_mut().storage, Coin::<Usdc>::new(1000), price)
            .expect("should deposit");

        let rewards = deposit.query_rewards(deps.as_ref().storage)
            .expect("should query");
        assert_eq!(Coin::<Nls>::new(0), rewards);
    }

    #[test]
    fn test_zero_funds_rewards() {
        let mut deps = testing::mock_dependencies();
        let price = NTokenPrice::<TheCurrency>::mock(Coin::new(1), Coin::new(1));
        let addr = Addr::unchecked("depositor");

        let rewards = Coin::<Nls>::new(1000);

        let mut deposit =
            Deposit::load(deps.as_ref().storage, addr)
            .expect("should load");

        deposit.deposit(deps.as_mut().storage, Coin::<Usdc>::new(1000), price)
            .expect("should deposit");

        // shouldn't change anything
        Deposit::distribute_rewards(deps.as_mut(), Coin::new(0))
            .expect("should distribute rewards");

        let rewards_res = deposit.query_rewards(deps.as_ref().storage)
            .expect("should query");
        assert_eq!(Coin::<Nls>::new(0), rewards_res);

        Deposit::distribute_rewards(deps.as_mut(), rewards)
            .expect("should distribute rewards");

        let rewards_res = deposit.query_rewards(deps.as_ref().storage)
            .expect("should query");
        assert_eq!(rewards, rewards_res);

        // shouldn't change anything
        Deposit::distribute_rewards(deps.as_mut(), Coin::new(0))
            .expect("should distribute rewards");

        let rewards_res = deposit.query_rewards(deps.as_ref().storage)
            .expect("should query");
        assert_eq!(rewards, rewards_res);

    }
}
