use crate::error::ContractError;
use crate::lpp::NTokenPrice;
use crate::nlpn::NLpn;
use cosmwasm_std::{Addr, Decimal, DepsMut, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};
use finance::coin::Coin;
use finance::currency::{Currency, Nls};
use finance::price;
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
    pub reward_per_token: Decimal,
    pub pending_rewards_nls: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
struct DepositsGlobals {
    pub balance_nlpn: Coin<NLpn>,

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

    pub fn deposit<LPN>(
        &mut self,
        storage: &mut dyn Storage,
        amount_lpn: Coin<LPN>,
        price: NTokenPrice<LPN>,
    ) -> StdResult<()>
    where
        LPN: Currency + Serialize + DeserializeOwned,
    {
        if amount_lpn.is_zero() {
            return Ok(());
        }

        let mut globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        self.update_rewards(&globals);

        let deposited_nlpn = price::total(amount_lpn, price.get().inv());
        self.data.deposited_nlpn = self.data.deposited_nlpn + deposited_nlpn;

        Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;

        globals.balance_nlpn = globals.balance_nlpn + deposited_nlpn;

        Self::GLOBALS.save(storage, &globals)
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
            self.update_rewards(&globals);
            let reward: Uint128 = self.data.pending_rewards_nls * Uint128::new(1);
            Self::DEPOSITS.remove(storage, self.addr.clone());
            Some(Coin::new(reward.u128()))
        } else {
            Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;
            None
        };

        Self::GLOBALS.save(storage, &globals)?;

        Ok(maybe_reward)
    }

    pub fn distribute_rewards(deps: DepsMut, rewards: Coin<Nls>) -> StdResult<()> {
        let mut globals = Self::GLOBALS.may_load(deps.storage)?.unwrap_or_default();

        // TODO: should we throw error in this case?
        if !globals.balance_nlpn.is_zero() {
            let balance_nlpn: u128 = globals.balance_nlpn.into();
            let rewards: u128 = rewards.into();
            globals.reward_per_token += Decimal::from_ratio(rewards, balance_nlpn);

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
        let deposited_nlpn: u128 = deposit.deposited_nlpn.into();
        deposit.pending_rewards_nls
            + (globals.reward_per_token - deposit.reward_per_token)
                * Decimal::from_ratio(deposited_nlpn, 1u128)
    }

    /// query accounted rewards
    pub fn query_rewards(&self, storage: &dyn Storage) -> StdResult<Coin<Nls>> {
        let globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();

        let reward = self.calculate_reward(&globals) * Uint128::new(1);
        Ok(Coin::new(reward.u128()))
    }

    /// pay accounted rewards to the deposit owner or optional recipient
    pub fn claim_rewards(&mut self, storage: &mut dyn Storage) -> StdResult<Coin<Nls>> {
        let globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        self.update_rewards(&globals);
        let reward = self.data.pending_rewards_nls * Uint128::new(1);
        self.data.pending_rewards_nls -= Decimal::from_ratio(reward.u128(), 1u128);

        Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;

        Ok(Coin::new(reward.u128()))
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
}
