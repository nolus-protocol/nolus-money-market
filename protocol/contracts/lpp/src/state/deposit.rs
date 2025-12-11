use serde::{Deserialize, Serialize};

use currency::platform::Nls;
use finance::{coin::Coin, zero::Zero};
use lpp_platform::NLpn;
use sdk::{
    cosmwasm_std::{Addr, Order, Storage},
    cw_storage_plus::Map,
};

use crate::{
    contract::{ContractError, Result},
    state::rewards::Index,
};

#[derive(Debug)]
#[cfg_attr(test, derive(Clone, PartialEq, Eq))]
pub struct Deposit {
    addr: Addr,
    data: DepositData,
    total_rewards: Index,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
struct DepositData {
    deposited_nlpn: Coin<NLpn>,

    #[serde(flatten)]
    pending_rewards_index: Index,
    pending_rewards_nls: Coin<Nls>,
}

impl Deposit {
    const DEPOSITS: Map<Addr, DepositData> = Map::new("deposits");

    pub fn load_or_default(
        storage: &dyn Storage,
        addr: Addr,
        total_rewards: Index,
    ) -> Result<Self> {
        Self::may_load(storage, addr.clone(), total_rewards).map(|may_deposit| {
            may_deposit.unwrap_or_else(|| Deposit {
                addr,
                data: DepositData::default(),
                total_rewards,
            })
        })
    }

    pub fn load(storage: &dyn Storage, addr: Addr, total_rewards: Index) -> Result<Self> {
        Self::may_load(storage, addr.clone(), total_rewards)
            .and_then(|may_deposit| may_deposit.ok_or_else(|| ContractError::NoDeposit {}))
    }

    pub fn iter(
        storage: &dyn Storage,
        total_rewards: Index,
    ) -> impl Iterator<Item = Result<Self>> + use<'_> {
        Self::DEPOSITS
            .prefix(())
            .range(storage, None, None, Order::Ascending)
            .map(move |record| {
                record.map_err(Into::into).map(|(addr, data)| Self {
                    addr,
                    data,
                    total_rewards,
                })
            })
    }

    pub fn save(self, storage: &mut dyn Storage) -> Result<()> {
        if self.data.deposited_nlpn.is_zero() {
            Self::DEPOSITS.remove(storage, self.addr);
            Ok(())
        } else {
            Self::DEPOSITS
                .save(storage, self.addr.clone(), &self.data)
                .map_err(Into::into)
        }
    }

    pub fn owner(&self) -> &Addr {
        &self.addr
    }

    pub fn receipts(&self) -> Coin<NLpn> {
        self.data.deposited_nlpn
    }

    pub fn try_deposit(&mut self, deposited_nlpn: Coin<NLpn>) -> Result<()> {
        debug_assert_ne!(Coin::ZERO, deposited_nlpn);

        self.try_update_rewards()
            .map(|()| self.data.deposited_nlpn += deposited_nlpn)
    }

    /// return optional NLS reward in case of deleting account
    pub fn withdraw(&mut self, amount_nlpn: Coin<NLpn>) -> Result<Option<Coin<Nls>>> {
        if self.data.deposited_nlpn < amount_nlpn {
            return Err(ContractError::InsufficientBalance);
        }

        self.try_update_rewards().map(|()| {
            self.data.deposited_nlpn -= amount_nlpn;

            if self.data.deposited_nlpn.is_zero() {
                Some(self.data.pending_rewards_nls)
            } else {
                None
            }
        })
    }

    /// query accounted rewards
    pub fn query_rewards(&self) -> Result<Coin<Nls>> {
        let deposit = &self.data;

        self.total_rewards
            .may_rewards(deposit.deposited_nlpn)
            .and_then(|global_reward| {
                deposit
                    .pending_rewards_index
                    .may_rewards(deposit.deposited_nlpn)
                    .map(|deposit_reward| {
                        debug_assert!(
                            deposit_reward <= global_reward,
                            "the global rewards index should only go up"
                        );
                        deposit.pending_rewards_nls + global_reward - deposit_reward
                    })
            })
    }

    /// take any pending rewards out
    pub fn may_claim_rewards(&mut self) -> Result<Coin<Nls>> {
        self.try_update_rewards().map(|()| {
            let reward = self.data.pending_rewards_nls;
            self.data.pending_rewards_nls = Coin::ZERO;

            reward
        })
    }

    fn may_load(storage: &dyn Storage, addr: Addr, total_rewards: Index) -> Result<Option<Self>> {
        Self::DEPOSITS
            .may_load(storage, addr.clone())
            .map_err(Into::into)
            .map(|may_data| {
                may_data.map(|data| Self {
                    addr,
                    data,
                    total_rewards,
                })
            })
    }

    fn try_update_rewards(&mut self) -> Result<()> {
        self.query_rewards().map(|rewards| {
            self.data.pending_rewards_nls = rewards;
            self.data.pending_rewards_index = self.total_rewards;
        })
    }
}

#[cfg(test)]
mod test {
    use currency::platform::Nls;
    use finance::{coin::Coin, zero::Zero};
    use lpp_platform::NLpn;
    use sdk::cosmwasm_std::{Addr, testing::MockStorage};

    use crate::{
        contract::ContractError,
        state::{Deposit, TotalRewards},
    };

    #[test]
    fn test_load_not_existent() {
        let store = MockStorage::default();
        let addr1 = Addr::unchecked("depositor1");
        let rewards = TotalRewards::load_or_default(&store).unwrap();
        assert_eq!(
            ContractError::NoDeposit {},
            Deposit::load(&store, addr1, rewards).unwrap_err(),
        );
    }

    #[test]
    fn test_deposit_and_withdraw() {
        let mut store = MockStorage::default();
        let addr1 = Addr::unchecked("depositor1");
        let addr2 = Addr::unchecked("depositor2");

        let rewards = TotalRewards::load_or_default(&store).unwrap();
        let mut deposit1 = Deposit::load_or_default(&store, addr1.clone(), rewards).unwrap();
        let deposit1_1 = Coin::new(1000);
        let withdraw1_1 = Coin::new(500);
        let deposit2_1 = Coin::new(500);
        assert_eq!(
            ContractError::InsufficientBalance {},
            deposit1.withdraw(deposit1_1).unwrap_err()
        );
        deposit1.try_deposit(deposit1_1).unwrap();

        assert_eq!(deposit1_1, deposit1.receipts());
        assert_eq!(Coin::ZERO, deposit1.query_rewards().unwrap());
        deposit1.save(&mut store).unwrap();

        // for simplicity, to maintain price 1:1, we keep rewards amount equal to the total receipts
        let rewards = rewards.add(Coin::new(deposit1_1.into()), deposit1_1);
        let deposit1 = Deposit::load(&store, addr1.clone(), rewards).unwrap();
        assert_eq!(
            Coin::new(deposit1_1.into()),
            deposit1.query_rewards().unwrap()
        );

        let mut deposit2 = Deposit::load_or_default(&store, addr2.clone(), rewards).unwrap();
        deposit2.try_deposit(deposit2_1).unwrap();

        assert_eq!(deposit2_1, deposit2.receipts());
        assert_eq!(Coin::ZERO, deposit2.query_rewards().unwrap());
        deposit2.save(&mut store).unwrap();

        let rewards = rewards.add(
            Coin::new((deposit1_1 + deposit2_1).into()),
            deposit1_1 + deposit2_1,
        );
        let mut deposit1 = Deposit::load(&store, addr1.clone(), rewards).unwrap();
        let mut deposit2 = Deposit::load(&store, addr2.clone(), rewards).unwrap();

        let rewards1_1 = Coin::new((deposit1_1 + deposit1_1).into());
        assert_eq!(rewards1_1, deposit1.query_rewards().unwrap());
        let rewards2 = Coin::new(deposit2_1.into());
        assert_eq!(rewards2, deposit2.query_rewards().unwrap());

        assert!(withdraw1_1 < deposit1_1 && deposit1.withdraw(withdraw1_1).unwrap().is_none());

        assert_eq!(rewards1_1, deposit1.may_claim_rewards().unwrap());
        assert_eq!(rewards2, deposit2.may_claim_rewards().unwrap());

        let rewards1_2 = Coin::new((deposit1_1 - withdraw1_1).into());
        deposit1.save(&mut store).unwrap();
        deposit2.save(&mut store).unwrap();

        let rewards = rewards.add(rewards1_2 + rewards2, deposit1_1 - withdraw1_1 + deposit2_1);
        let mut deposit1 = Deposit::load(&store, addr1.clone(), rewards).unwrap();
        let mut deposit2 = Deposit::load(&store, addr2.clone(), rewards).unwrap();

        assert_eq!(rewards1_2, deposit1.query_rewards().unwrap());
        assert_eq!(rewards2, deposit2.query_rewards().unwrap());

        // withdraw all, return rewards, close deposit
        assert_eq!(
            Some(rewards1_2),
            deposit1.withdraw(deposit1_1 - withdraw1_1).unwrap()
        );
        assert_eq!(Some(rewards2), deposit2.withdraw(deposit2_1).unwrap());
    }

    #[test]
    fn test_query_rewards_zero_balance() {
        let store = MockStorage::default();
        let addr = Addr::unchecked("depositor");
        let rewards = TotalRewards::load_or_default(&store).unwrap();

        let mut deposit = Deposit::load_or_default(&store, addr, rewards).unwrap();

        // balance_nls = 0, balance_nlpn = 0
        assert!(deposit.query_rewards().unwrap().is_zero());

        // balance_nls = 0, balance_nlpn != 0
        deposit.try_deposit(Coin::new(1000)).unwrap();

        assert!(deposit.query_rewards().unwrap().is_zero());
    }

    #[test]
    fn test_zero_balance_distribute_rewards() {
        let mut store = MockStorage::default();
        let addr1 = Addr::unchecked("depositor1");
        let addr2 = Addr::unchecked("depositor2");
        const REWARDS: Coin<Nls> = Coin::new(124);
        const REWARD_DEPOSIT: Coin<Nls> = Coin::new(124 / 2);
        const RECEIPTS: Coin<NLpn> = Coin::new(1000);

        let mut rewards = TotalRewards::load_or_default(&store).unwrap();

        let mut deposit1 = Deposit::load_or_default(&store, addr1.clone(), rewards).unwrap();
        deposit1.try_deposit(RECEIPTS).unwrap();
        deposit1.save(&mut store).unwrap();

        rewards = rewards.add(REWARDS, RECEIPTS);
        let mut deposit1 = Deposit::load_or_default(&store, addr1.clone(), rewards).unwrap();
        assert_eq!(REWARDS, deposit1.query_rewards().unwrap());
        assert_eq!(REWARDS, deposit1.may_claim_rewards().unwrap());
        assert_eq!(Coin::ZERO, deposit1.query_rewards().unwrap());
        deposit1.save(&mut store).unwrap();

        let mut deposit2 = Deposit::load_or_default(&store, addr2.clone(), rewards).unwrap();
        deposit2.try_deposit(RECEIPTS).unwrap();
        assert_eq!(Coin::ZERO, deposit2.query_rewards().unwrap());
        assert_eq!(Coin::ZERO, deposit2.may_claim_rewards().unwrap());
        deposit2.save(&mut store).unwrap();

        rewards = rewards.add(REWARDS, RECEIPTS + RECEIPTS);

        let mut deposit2 = Deposit::load(&store, addr2.clone(), rewards).unwrap();
        assert_eq!(REWARD_DEPOSIT, deposit2.query_rewards().unwrap());
        assert_eq!(REWARD_DEPOSIT, deposit2.may_claim_rewards().unwrap());
        assert_eq!(Coin::ZERO, deposit2.query_rewards().unwrap());

        let mut deposit1 = Deposit::load(&store, addr1, rewards).unwrap();
        assert_eq!(REWARD_DEPOSIT, deposit1.query_rewards().unwrap());
        assert_eq!(REWARD_DEPOSIT, deposit1.may_claim_rewards().unwrap());
        assert_eq!(Coin::ZERO, deposit1.query_rewards().unwrap());
    }

    #[test]
    fn test_empty_iter() {
        let mut store = MockStorage::default();
        let rewards = TotalRewards::load_or_default(&store).unwrap();
        assert_eq!(None, Deposit::iter(&store, rewards).next());

        let addr1 = Addr::unchecked("depositor1");

        let mut deposit1 = Deposit::load_or_default(&store, addr1.clone(), rewards).unwrap();
        assert_eq!(None, Deposit::iter(&store, rewards).next()); //non-saved

        const DEPOSIT_RECEIPTS: Coin<NLpn> = Coin::new(1000);
        const WITHDRAW1_RECEIPTS: Coin<NLpn> = Coin::new(245);
        const WITHDRAW2_RECEIPTS: Option<Coin<NLpn>> =
            DEPOSIT_RECEIPTS.checked_sub(WITHDRAW1_RECEIPTS);
        deposit1.try_deposit(DEPOSIT_RECEIPTS).unwrap();
        deposit1.save(&mut store).unwrap();

        let mut deposit1 = Deposit::load(&store, addr1.clone(), rewards).unwrap();
        {
            let mut deposits = Deposit::iter(&store, rewards);
            assert_eq!(Some(Ok(deposit1.clone())), deposits.next());
            assert_eq!(None, deposits.next());
        }

        assert_eq!(Ok(None), deposit1.withdraw(WITHDRAW1_RECEIPTS));
        deposit1.save(&mut store).unwrap();

        let mut deposit1 = Deposit::load(&store, addr1.clone(), rewards).unwrap();
        assert_eq!(
            Some(Ok(deposit1.clone())),
            Deposit::iter(&store, rewards).next()
        );

        assert_eq!(
            // closing the deposit
            Ok(Some(Coin::ZERO)),
            deposit1.withdraw(WITHDRAW2_RECEIPTS.unwrap())
        );
        assert_eq!(Coin::ZERO, deposit1.receipts());
        deposit1.save(&mut store).unwrap();
        assert_eq!(None, Deposit::iter(&store, rewards).next());
    }

    #[test]
    fn test_iter_deposits() {
        let addr1 = Addr::unchecked("depositor1");
        let addr2 = Addr::unchecked("depositor2");
        const DEPOSIT1_RECEIPTS: Coin<NLpn> = Coin::new(1000);
        const DEPOSIT2_RECEIPTS: Coin<NLpn> = Coin::new(352);

        let mut store = MockStorage::default();
        let rewards = TotalRewards::load_or_default(&store).unwrap();
        assert_eq!(None, Deposit::iter(&store, rewards).next());

        {
            let mut deposit1 = Deposit::load_or_default(&store, addr1.clone(), rewards).unwrap();
            deposit1.try_deposit(DEPOSIT1_RECEIPTS).unwrap();
            deposit1.save(&mut store).unwrap();
        }
        {
            let mut deposit2 = Deposit::load_or_default(&store, addr2.clone(), rewards).unwrap();
            deposit2.try_deposit(DEPOSIT2_RECEIPTS).unwrap();
            deposit2.save(&mut store).unwrap();
        }

        {
            let deposit1 = Deposit::load(&store, addr1.clone(), rewards).unwrap();
            let deposit2 = Deposit::load(&store, addr2.clone(), rewards).unwrap();

            let mut deposits = Deposit::iter(&store, rewards);
            assert_eq!(Some(Ok(deposit1)), deposits.next());
            assert_eq!(Some(Ok(deposit2)), deposits.next());
            assert_eq!(None, deposits.next());
        }
    }
}
