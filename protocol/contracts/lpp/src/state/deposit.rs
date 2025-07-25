use serde::{Deserialize, Serialize};

use currency::platform::Nls;
use finance::{
    coin::Coin,
    price::{self, Price},
    zero::Zero,
};
use lpp_platform::NLpn;
use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage},
    cw_storage_plus::{Item, Map},
};

use crate::contract::{ContractError, Result};

#[derive(Debug)]
pub struct Deposit {
    addr: Addr,
    data: DepositData,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
struct DepositData {
    deposited_nlpn: Coin<NLpn>,

    // Rewards
    reward_per_token: Option<Price<NLpn, Nls>>,
    pending_rewards_nls: Coin<Nls>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
struct DepositsGlobals {
    // Rewards
    reward_per_token: Option<Price<NLpn, Nls>>,
}

impl Deposit {
    const DEPOSITS: Map<Addr, DepositData> = Map::new("deposits");
    // take Globals::reward_per_token out in a dedicated abstraction and keep working Deposit -> <Rewards>
    const GLOBALS: Item<DepositsGlobals> = Item::new("deposits_globals");

    pub fn load_or_default(storage: &dyn Storage, addr: Addr) -> Result<Self> {
        Self::may_load(storage, addr.clone()).map(|may_deposit| {
            may_deposit.unwrap_or_else(|| Deposit {
                addr,
                data: DepositData::default(),
            })
        })
    }

    pub fn may_load(storage: &dyn Storage, addr: Addr) -> Result<Option<Self>> {
        Self::DEPOSITS
            .may_load(storage, addr.clone())
            .map_err(Into::into)
            .map(|may_data| may_data.map(|data| Self { addr, data }))
    }

    pub fn deposit(
        &mut self,
        storage: &mut dyn Storage,
        deposited_nlpn: Coin<NLpn>,
    ) -> Result<Coin<NLpn>> {
        if deposited_nlpn.is_zero() {
            return Err(ContractError::ZeroDepositFunds);
        }

        let globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        self.update_rewards(&globals);

        self.data.deposited_nlpn += deposited_nlpn;

        Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;

        Ok(deposited_nlpn)
    }

    /// return optional NLS reward in case of deleting account
    pub fn withdraw(
        &mut self,
        storage: &mut dyn Storage,
        amount_nlpn: Coin<NLpn>,
    ) -> Result<Option<Coin<Nls>>> {
        if self.data.deposited_nlpn < amount_nlpn {
            return Err(ContractError::InsufficientBalance);
        }

        let globals = Self::GLOBALS.may_load(storage)?.unwrap_or_default();
        self.update_rewards(&globals);

        self.data.deposited_nlpn -= amount_nlpn;

        let maybe_reward = if self.data.deposited_nlpn.is_zero() {
            Self::DEPOSITS.remove(storage, self.addr.clone());
            Some(self.data.pending_rewards_nls)
        } else {
            Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;
            None
        };

        Ok(maybe_reward)
    }

    pub fn receipts(&self) -> Coin<NLpn> {
        self.data.deposited_nlpn
    }

    pub fn distribute_rewards(
        store: &mut dyn Storage,
        rewards: Coin<Nls>,
        total_receipts: Coin<NLpn>,
    ) -> Result<()> {
        if total_receipts.is_zero() {
            return Err(ContractError::ZeroBalanceRewards {});
        }

        if rewards.is_zero() {
            return Err(ContractError::ZeroRewardsFunds {});
        }

        let new_reward_per_receipt = price::total_of(total_receipts).is(rewards);

        let mut globals = Self::GLOBALS.may_load(store)?.unwrap_or_default();
        if let Some(ref mut reward_per_token) = globals.reward_per_token {
            *reward_per_token += new_reward_per_receipt;
        } else {
            globals.reward_per_token = Some(new_reward_per_receipt);
        }

        Ok(Self::GLOBALS.save(store, &globals)?)
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
        self.data.pending_rewards_nls = Coin::ZERO;

        Self::DEPOSITS.save(storage, self.addr.clone(), &self.data)?;

        Ok(reward)
    }

    fn update_rewards(&mut self, globals: &DepositsGlobals) {
        self.data.pending_rewards_nls = self.calculate_reward(globals);
        self.data.reward_per_token = globals.reward_per_token;
    }

    fn calculate_reward(&self, globals: &DepositsGlobals) -> Coin<Nls> {
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
}

#[cfg(test)]
mod test {
    use finance::{coin::Coin, zero::Zero};
    use sdk::cosmwasm_std::{Addr, testing::MockStorage};

    use crate::state::Deposit;

    #[test]
    fn test_deposit_and_withdraw() {
        let mut store = MockStorage::default();
        let addr1 = Addr::unchecked("depositor1");
        let addr2 = Addr::unchecked("depositor2");

        let mut deposit1 = Deposit::load_or_default(&store, addr1.clone()).unwrap();
        let deposit1_1 = 1000.into();
        let withdraw1_1 = 500.into();
        let deposit2_1 = 500.into();
        deposit1.deposit(&mut store, deposit1_1).unwrap();

        // for simplicity, to maintain price 1:1, we keep rewards amount equal to the total receipts
        Deposit::distribute_rewards(&mut store, Coin::new(deposit1_1.into()), deposit1_1).unwrap();

        let mut deposit2 = Deposit::load_or_default(&store, addr2.clone()).unwrap();
        deposit2.deposit(&mut store, deposit2_1).unwrap();

        assert_eq!(deposit2_1, deposit2.receipts());

        assert_eq!(
            Coin::new(deposit1_1.into()),
            deposit1.query_rewards(&store).unwrap()
        );
        assert_eq!(Coin::ZERO, deposit2.query_rewards(&store).unwrap());

        Deposit::distribute_rewards(
            &mut store,
            Coin::new((deposit1_1 + deposit2_1).into()),
            deposit1_1 + deposit2_1,
        )
        .unwrap();

        let rewards1_1 = Coin::new((deposit1_1 + deposit1_1).into());
        assert_eq!(rewards1_1, deposit1.query_rewards(&store).unwrap());
        let rewards2 = Coin::new(deposit2_1.into());
        assert_eq!(rewards2, deposit2.query_rewards(&store).unwrap());

        assert!(
            withdraw1_1 < deposit1_1
                && deposit1
                    .withdraw(&mut store, withdraw1_1)
                    .unwrap()
                    .is_none()
        );

        assert_eq!(rewards1_1, deposit1.claim_rewards(&mut store).unwrap());
        assert_eq!(rewards2, deposit2.claim_rewards(&mut store).unwrap());

        let rewards1_2 = Coin::new((deposit1_1 - withdraw1_1).into());
        Deposit::distribute_rewards(
            &mut store,
            rewards1_2 + rewards2,
            deposit1_1 - withdraw1_1 + deposit2_1,
        )
        .unwrap();

        assert_eq!(rewards1_2, deposit1.query_rewards(&store).unwrap());
        assert_eq!(rewards2, deposit2.query_rewards(&store).unwrap());

        // withdraw all, return rewards, close deposit
        assert_eq!(
            Some(rewards1_2),
            deposit1
                .withdraw(&mut store, deposit1_1 - withdraw1_1)
                .unwrap()
        );
        assert_eq!(
            Some(rewards2),
            deposit2.withdraw(&mut store, deposit2_1).unwrap()
        );
    }

    #[test]
    fn test_query_rewards_zero_balance() {
        let mut store = MockStorage::default();
        let addr = Addr::unchecked("depositor");

        let mut deposit = Deposit::load_or_default(&store, addr).unwrap();

        // balance_nls = 0, balance_nlpn = 0
        assert!(deposit.query_rewards(&store).unwrap().is_zero());

        // balance_nls = 0, balance_nlpn != 0
        deposit.deposit(&mut store, Coin::new(1000)).unwrap();

        assert!(deposit.query_rewards(&store).unwrap().is_zero());
    }

    #[test]
    fn test_zero_funds_rewards() {
        let mut store = MockStorage::default();
        let addr = Addr::unchecked("depositor");

        let mut deposit = Deposit::load_or_default(&store, addr).unwrap();

        let deposited = Coin::new(1000);
        deposit.deposit(&mut store, deposited).unwrap();

        Deposit::distribute_rewards(&mut store, Coin::ZERO, deposited).unwrap_err();
    }

    #[test]
    fn test_zero_balance_distribute_rewards() {
        let mut store = MockStorage::default();
        let rewards = Coin::new(1000);

        Deposit::distribute_rewards(&mut store, rewards, Coin::ZERO).unwrap_err();
    }
}
