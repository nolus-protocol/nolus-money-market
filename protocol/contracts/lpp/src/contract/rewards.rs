use currency::CurrencyDef;
use finance::coin::Coin;
use lpp_platform::NLpn;
use platform::{
    bank::{self, BankAccount, BankAccountView},
    batch::Batch,
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Storage, Timestamp};

use crate::{
    config::Config as ApiConfig,
    lpp::{LiquidityPool, LppBalances},
    msg::RewardsResponse,
    state::Deposit,
};

use super::error::{ContractError, Result};

pub(super) fn try_distribute_rewards<Lpn, Bank>(
    store: &mut dyn Storage,
    info: MessageInfo,
    config: &ApiConfig,
    bank: &Bank,
) -> Result<MessageResponse>
where
    Lpn: CurrencyDef,
    Bank: BankAccountView,
{
    bank::received_one(&info.funds)
        .map_err(Into::into)
        .and_then(|amount| {
            query_total_receipts::<Lpn, _>(store, config, bank)
                .and_then(|receipts| Deposit::distribute_rewards(store, amount, receipts))
        })
        .map(|()| Default::default())
}

pub(super) fn try_claim_rewards(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    other_recipient: Option<Addr>,
) -> Result<MessageResponse> {
    let recipient = other_recipient
        .map(|recipient| deps.api.addr_validate(recipient.as_str()))
        .transpose()?
        .unwrap_or_else(|| info.sender.clone());

    let mut deposit =
        Deposit::may_load(deps.storage, info.sender)?.ok_or(ContractError::NoDeposit {})?;

    let reward = deposit.claim_rewards(deps.storage)?;

    if reward.is_zero() {
        return Err(ContractError::NoRewards {});
    }

    let mut bank = bank::account(&env.contract.address, deps.querier);
    bank.send(reward, recipient);
    let batch: Batch = bank.into();

    Ok(batch.into())
}

pub(super) fn query_lpp_balance<Lpn, Bank>(
    storage: &dyn Storage,
    config: &ApiConfig,
    bank: &Bank,
    now: &Timestamp,
) -> Result<LppBalances<Lpn>>
where
    Lpn: CurrencyDef,
    Bank: BankAccountView,
{
    LiquidityPool::load(storage, config, bank).and_then(|lpp| lpp.query_lpp_balance(now))
}

pub(super) fn query_total_receipts<Lpn, Bank>(
    storage: &dyn Storage,
    config: &ApiConfig,
    bank: &Bank,
) -> Result<Coin<NLpn>>
where
    Lpn: CurrencyDef,
    Bank: BankAccountView,
{
    LiquidityPool::<Lpn, _>::load(storage, config, bank).map(|lpp| lpp.balance_nlpn())
}

pub(super) fn query_rewards(storage: &dyn Storage, addr: Addr) -> Result<RewardsResponse> {
    let rewards = Deposit::may_load(storage, addr)?
        .ok_or(ContractError::NoDeposit {})?
        .query_rewards(storage)?;

    Ok(RewardsResponse { rewards })
}

#[cfg(test)]
mod test {
    use finance::{
        coin::Coin,
        percent::{Percent, bound::BoundToHundredPercent},
        zero::Zero,
    };
    use platform::{bank::testing::MockBankView, contract::Code};
    use sdk::cosmwasm_std::testing::{mock_dependencies, mock_env};

    use crate::{
        borrow::InterestRate,
        config::Config as ApiConfig,
        contract::{
            error::ContractError,
            lender, rewards,
            test::{self, TheCurrency},
        },
        lpp::LiquidityPool,
        state::Config,
    };

    const BASE_INTEREST_RATE: Percent = Percent::from_permille(70);
    const UTILIZATION_OPTIMAL: Percent = Percent::from_permille(700);
    const ADDON_OPTIMAL_INTEREST_RATE: Percent = Percent::from_permille(20);
    const DEFAULT_MIN_UTILIZATION: BoundToHundredPercent = BoundToHundredPercent::ZERO;

    #[test]
    fn test_claim_zero_rewards() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let now = env.block.time;

        const INITIAL_LPP_BALANCE: Coin<TheCurrency> = Coin::ZERO;
        const DEPOSIT: Coin<TheCurrency> = Coin::new(20_000);

        let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(INITIAL_LPP_BALANCE);
        let config = ApiConfig::new(
            Code::unchecked(1000u64),
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
            DEFAULT_MIN_UTILIZATION,
        );
        Config::store(&config, deps.as_mut().storage).unwrap();
        LiquidityPool::<TheCurrency, _>::new(&config, &bank)
            .save(deps.as_mut().storage)
            .unwrap();

        // no deposit
        let info = test::lender_msg_no_funds();
        let response = super::try_claim_rewards(deps.as_mut(), env.clone(), info, None);
        assert_eq!(response, Err(ContractError::NoDeposit {}));

        let bank =
            MockBankView::<TheCurrency, TheCurrency>::only_balance(INITIAL_LPP_BALANCE + DEPOSIT);

        lender::try_deposit::<TheCurrency, _>(
            &mut deps.storage,
            &bank,
            test::lender(),
            DEPOSIT,
            &now,
        )
        .unwrap();

        // pending rewards == 0
        let info = test::lender_msg_no_funds();
        let response = rewards::try_claim_rewards(deps.as_mut(), env, info, None);
        assert_eq!(response, Err(ContractError::NoRewards {}));
    }
}
