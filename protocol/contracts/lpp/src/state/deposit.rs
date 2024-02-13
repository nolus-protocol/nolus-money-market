use serde::{de::DeserializeOwned, Deserialize, Serialize};

use currency::{Currency, NlsPlatform};
use finance::{
    coin::Coin,
    price::{self, Price},
    zero::Zero,
};
use lpp_platform::NLpn;
use sdk::{
    cosmwasm_ext::as_dyn::{storage, AsDyn},
    cosmwasm_std::{Addr, DepsMut, StdResult},
    cw_storage_plus::{Item, Map},
};

use crate::{
    error::{ContractError, Result},
    lpp::NTokenPrice,
};

#[derive(Debug)]
pub struct Deposit {
    addr: Addr,
    data: DepositData,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
struct DepositData {
    deposited_nlpn: Coin<NLpn>,

    // Rewards
    reward_per_token: Option<Price<NLpn, NlsPlatform>>,
    pending_rewards_nls: Coin<NlsPlatform>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
struct DepositsGlobals {
    balance_nlpn: Coin<NLpn>,

    // Rewards
    reward_per_token: Option<Price<NLpn, NlsPlatform>>,
}

impl Deposit {
    const DEPOSITS: Map<'static, Addr, DepositData> = Map::new("deposits");
    const GLOBALS: Item<'static, DepositsGlobals> = Item::new("deposits_globals");

    pub fn load_or_default<S>(storage: &S, addr: Addr) -> StdResult<Self>
    where
        S: storage::Dyn + ?Sized,
    {
        let data = Self::DEPOSITS
            .may_load(storage.as_dyn(), addr.clone())?
            .unwrap_or_default();

        Ok(Self { addr, data })
    }

    pub fn may_load<S>(storage: &S, addr: Addr) -> StdResult<Option<Self>>
    where
        S: storage::Dyn + ?Sized,
    {
        let result = Self::DEPOSITS
            .may_load(storage.as_dyn(), addr.clone())?
            .map(|data| Self { addr, data });
        Ok(result)
    }

    pub fn deposit<S, Lpn>(
        &mut self,
        storage: &mut S,
        amount_lpn: Coin<Lpn>,
        price: NTokenPrice<Lpn>,
    ) -> Result<Coin<NLpn>>
    where
        S: storage::DynMut + ?Sized,
        Lpn: Currency + Serialize + DeserializeOwned,
    {
        if amount_lpn.is_zero() {
            return Err(ContractError::ZeroDepositFunds);
        }

        let mut globals = Self::GLOBALS
            .may_load(storage.as_dyn())?
            .unwrap_or_default();
        self.update_rewards(&globals);

        let deposited_nlpn = price::total(amount_lpn, price.get().inv());
        self.data.deposited_nlpn += deposited_nlpn;

        Self::DEPOSITS.save(storage.as_dyn_mut(), self.addr.clone(), &self.data)?;

        globals.balance_nlpn = globals
            .balance_nlpn
            .checked_add(deposited_nlpn)
            .ok_or(ContractError::OverflowError)?;

        Self::GLOBALS.save(storage.as_dyn_mut(), &globals)?;

        Ok(deposited_nlpn)
    }

    /// return optional reward payment msg in case of deleting account
    pub fn withdraw<S>(
        &mut self,
        storage: &mut S,
        amount_nlpn: Coin<NLpn>,
    ) -> Result<Option<Coin<NlsPlatform>>>
    where
        S: storage::DynMut + ?Sized,
    {
        if self.data.deposited_nlpn < amount_nlpn {
            return Err(ContractError::InsufficientBalance);
        }

        let storage = storage.as_dyn_mut();

        let mut globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        self.update_rewards(&globals);

        self.data.deposited_nlpn -= amount_nlpn;
        globals.balance_nlpn -= amount_nlpn;

        let maybe_reward = if self.data.deposited_nlpn.is_zero() {
            Self::DEPOSITS.remove(storage, self.addr.clone());
            Some(self.data.pending_rewards_nls)
        } else {
            Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;
            None
        };

        Self::GLOBALS.save(storage, &globals)?;

        Ok(maybe_reward)
    }

    pub fn distribute_rewards(deps: DepsMut<'_>, rewards: Coin<NlsPlatform>) -> Result<()> {
        let mut globals = Self::GLOBALS.may_load(deps.storage)?.unwrap_or_default();

        if globals.balance_nlpn.is_zero() {
            return Err(ContractError::ZeroBalanceRewards {});
        }

        if rewards.is_zero() {
            return Err(ContractError::ZeroRewardsFunds {});
        }

        let partial_price = price::total_of(globals.balance_nlpn).is(rewards);

        if let Some(ref mut reward_per_token) = globals.reward_per_token {
            *reward_per_token += partial_price;
        } else {
            globals.reward_per_token = Some(partial_price);
        }

        Ok(Self::GLOBALS.save(deps.storage, &globals)?)
    }

    fn update_rewards(&mut self, globals: &DepositsGlobals) {
        self.data.pending_rewards_nls = self.calculate_reward(globals);
        self.data.reward_per_token = globals.reward_per_token;
    }

    fn calculate_reward(&self, globals: &DepositsGlobals) -> Coin<NlsPlatform> {
        let deposit = &self.data;

        let global_reward = globals
            .reward_per_token
            .map(|price| price::total(deposit.deposited_nlpn, price))
            .unwrap_or_default();

        let deposit_reward = deposit
            .reward_per_token
            .map(|price| price::total(deposit.deposited_nlpn, price))
            .unwrap_or_default();

        deposit.pending_rewards_nls + global_reward - deposit_reward
    }

    /// query accounted rewards
    pub fn query_rewards<S>(&self, storage: &S) -> StdResult<Coin<NlsPlatform>>
    where
        S: storage::Dyn + ?Sized,
    {
        let globals = Self::GLOBALS
            .may_load(storage.as_dyn())?
            .unwrap_or_default();
        Ok(self.calculate_reward(&globals))
    }

    /// pay accounted rewards to the deposit owner or optional recipient
    pub fn claim_rewards<S>(&mut self, storage: &mut S) -> StdResult<Coin<NlsPlatform>>
    where
        S: storage::DynMut + ?Sized,
    {
        let globals = Self::GLOBALS
            .may_load(storage.as_dyn())?
            .unwrap_or_default();
        self.update_rewards(&globals);

        let reward = self.data.pending_rewards_nls;
        self.data.pending_rewards_nls = Coin::ZERO;

        Self::DEPOSITS.save(storage.as_dyn_mut(), self.addr.clone(), &self.data)?;

        Ok(reward)
    }

    /// lpp derivative tokens balance
    pub fn balance_nlpn<S>(storage: &S) -> StdResult<Coin<NLpn>>
    where
        S: storage::Dyn + ?Sized,
    {
        Ok(Self::GLOBALS
            .may_load(storage.as_dyn())?
            .unwrap_or_default()
            .balance_nlpn)
    }

    /// deposit derivative tokens balance
    pub fn query_balance_nlpn<S>(storage: &S, addr: Addr) -> StdResult<Option<Coin<NLpn>>>
    where
        S: storage::Dyn + ?Sized,
    {
        let maybe_balance = Self::DEPOSITS
            .may_load(storage.as_dyn(), addr)?
            .map(|data| data.deposited_nlpn);
        Ok(maybe_balance)
    }
}

#[cfg(test)]
mod test {
    use currencies::test::StableC1;
    use sdk::cosmwasm_std::testing;

    use crate::lpp::NTokenPrice;

    use super::*;

    type TheCurrency = StableC1;

    #[test]
    fn test_deposit_and_withdraw() {
        let mut deps = testing::mock_dependencies();
        let addr1 = Addr::unchecked("depositor1");
        let addr2 = Addr::unchecked("depositor2");
        let price = NTokenPrice::<TheCurrency>::mock(Coin::new(1), Coin::new(1));

        let mut deposit1 =
            Deposit::load_or_default(deps.as_ref().storage, addr1.clone()).expect("should load");
        deposit1
            .deposit(deps.as_mut().storage, 1000u128.into(), price)
            .expect("should deposit");

        Deposit::distribute_rewards(deps.as_mut(), Coin::new(1000))
            .expect("should distribute rewards");

        let price = NTokenPrice::<TheCurrency>::mock(Coin::new(1), Coin::new(2));
        let mut deposit2 =
            Deposit::load_or_default(deps.as_ref().storage, addr2.clone()).expect("should load");
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
        assert_eq!(amount, Coin::<NlsPlatform>::new(2000));

        let amount = deposit2
            .claim_rewards(deps.as_mut().storage)
            .expect("should claim rewards");
        assert_eq!(amount, Coin::<NlsPlatform>::new(500));

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
        assert_eq!(rewards, Coin::<NlsPlatform>::new(500));
        let response =
            Deposit::query_balance_nlpn(deps.as_mut().storage, addr1).expect("should query");
        assert!(response.is_none());
    }

    #[test]
    fn test_query_rewards_zero_balance() {
        let mut deps = testing::mock_dependencies();
        let price = NTokenPrice::<TheCurrency>::mock(Coin::new(1), Coin::new(1));
        let addr = Addr::unchecked("depositor");

        let mut deposit =
            Deposit::load_or_default(deps.as_ref().storage, addr).expect("should load");

        // balance_nls = 0, balance_nlpn = 0
        let rewards = deposit
            .query_rewards(deps.as_ref().storage)
            .expect("should query");
        assert_eq!(Coin::<NlsPlatform>::new(0), rewards);

        // balance_nls = 0, balance_nlpn != 0
        deposit
            .deposit(deps.as_mut().storage, Coin::<StableC1>::new(1000), price)
            .expect("should deposit");

        let rewards = deposit
            .query_rewards(deps.as_ref().storage)
            .expect("should query");
        assert_eq!(Coin::<NlsPlatform>::new(0), rewards);
    }

    #[test]
    fn test_zero_funds_rewards() {
        let mut deps = testing::mock_dependencies();
        let price = NTokenPrice::<TheCurrency>::mock(Coin::new(1), Coin::new(1));
        let addr = Addr::unchecked("depositor");

        let mut deposit =
            Deposit::load_or_default(deps.as_ref().storage, addr).expect("should load");

        deposit
            .deposit(deps.as_mut().storage, Coin::<StableC1>::new(1000), price)
            .expect("should deposit");

        // shouldn't change anything
        Deposit::distribute_rewards(deps.as_mut(), Coin::new(0)).unwrap_err();
    }

    #[test]
    fn test_zero_balance_distribute_rewards() {
        let mut deps = testing::mock_dependencies();
        let rewards = Coin::new(1000);

        Deposit::distribute_rewards(deps.as_mut(), rewards).unwrap_err();
    }
}
