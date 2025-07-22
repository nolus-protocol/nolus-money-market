use currency::CurrencyDef;
use finance::{coin::Coin, price, zero::Zero};
use lpp_platform::NLpn;
use platform::{
    bank::{self, BankAccount, BankAccountView},
    batch::Batch,
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Env, MessageInfo, Storage, Timestamp};

use crate::{
    event,
    lpp::LiquidityPool,
    msg::{BalanceResponse, PriceResponse},
    state::Deposit,
};

use super::error::{ContractError, Result};

pub(super) fn try_deposit<Lpn, Bank>(
    storage: &mut dyn Storage,
    bank: &Bank,
    env: Env,
    info: MessageInfo,
) -> Result<MessageResponse>
where
    Lpn: 'static + CurrencyDef,
    Bank: BankAccountView,
{
    let lender_addr = info.sender;
    let pending_deposit = bank::received_one(&info.funds)?;

    let receipts = {
        let now = &env.block.time;
        let lpp = LiquidityPool::<Lpn, _>::load(storage, bank)?;

        if lpp
            .deposit_capacity(now, pending_deposit)?
            .map(|capacity| pending_deposit > capacity)
            .unwrap_or_default()
        {
            return Err(ContractError::UtilizationBelowMinimalRates);
        }
        let price = lpp.calculate_price(storage, now, pending_deposit)?;

        let deposit_nlpn = price::total(pending_deposit, price.inv());

        Deposit::load_or_default(storage, lender_addr.clone())?.deposit(storage, deposit_nlpn)?
    };

    Ok(event::emit_deposit(env, lender_addr, pending_deposit, receipts).into())
}

pub(super) fn deposit_capacity<Lpn, Bank>(
    storage: &dyn Storage,
    bank: &Bank,
    now: &Timestamp,
) -> Result<Option<Coin<Lpn>>>
where
    Lpn: 'static + CurrencyDef,
    Bank: BankAccountView,
{
    LiquidityPool::<'_, Lpn, Bank>::load(storage, bank)
        .and_then(|lpp| lpp.deposit_capacity(now, Coin::ZERO))
}

pub(super) fn try_withdraw<Lpn, Bank>(
    storage: &mut dyn Storage,
    mut bank: Bank,
    env: Env,
    info: MessageInfo,
    amount: Coin<NLpn>,
) -> Result<MessageResponse>
where
    Lpn: CurrencyDef,
    Bank: BankAccount,
{
    if amount.is_zero() {
        return Err(ContractError::ZeroWithdrawFunds);
    }

    let lender_addr = info.sender;

    let lpp = LiquidityPool::<'_, Lpn, _>::load(storage, &bank)?;
    let payment_lpn = lpp.withdraw_lpn(storage, &env.block.time, amount)?;

    let maybe_reward = Deposit::may_load(storage, lender_addr.clone())?
        .ok_or(ContractError::NoDeposit {})?
        .withdraw(storage, amount)?;

    bank.send(payment_lpn, lender_addr.clone());

    if let Some(reward) = maybe_reward {
        if !reward.is_zero() {
            bank.send(reward, lender_addr.clone());
        }
    }

    let batch: Batch = bank.into();
    Ok(MessageResponse::messages_with_events(
        batch,
        event::emit_withdraw(
            env,
            lender_addr,
            payment_lpn,
            amount,
            maybe_reward.is_some(),
        ),
    ))
}

pub fn query_ntoken_price<Lpn, Bank>(
    storage: &dyn Storage,
    bank: &Bank,
    now: &Timestamp,
) -> Result<PriceResponse<Lpn>>
where
    Lpn: CurrencyDef,
    Bank: BankAccountView,
{
    LiquidityPool::load(storage, bank).and_then(|lpp| {
        lpp.calculate_price(storage, now, Coin::ZERO)
            .map(PriceResponse)
    })
}

pub fn query_balance(storage: &dyn Storage, addr: Addr) -> Result<BalanceResponse> {
    let balance: u128 = Deposit::query_balance_nlpn(storage, addr)?
        .unwrap_or_default()
        .into();
    Ok(BalanceResponse {
        balance: balance.into(),
    })
}

#[cfg(test)]
mod test {
    use std::ops::DerefMut as _;

    use access_control::ContractOwnerAccess;

    use finance::{
        coin::Coin,
        percent::{Percent, bound::BoundToHundredPercent},
    };
    use platform::{
        bank::{BankAccountView, BankStub, testing::MockBankView},
        contract::Code,
    };
    use sdk::cosmwasm_std::{
        Addr, Env, Storage,
        testing::{self, MockStorage},
    };

    use crate::{
        borrow::InterestRate,
        config::Config as ApiConfig,
        contract::{
            lender,
            test::{self as test_tools, TheCurrency},
        },
        state::Config,
    };

    use super::LiquidityPool;

    const BASE_INTEREST_RATE: Percent = Percent::from_permille(70);
    const UTILIZATION_OPTIMAL: Percent = Percent::from_permille(700);
    const ADDON_OPTIMAL_INTEREST_RATE: Percent = Percent::from_permille(20);
    const DEFAULT_MIN_UTILIZATION: BoundToHundredPercent = BoundToHundredPercent::ZERO;

    fn test_case<Balance, F>(
        initial_lpp_balance: Balance,
        min_utilization: BoundToHundredPercent,
        f: F,
    ) where
        Balance: Copy + Into<Coin<TheCurrency>>,
        F: FnOnce(MockStorage, BankStub<MockBankView<TheCurrency, TheCurrency>>, Env),
    {
        let mut store = testing::MockStorage::default();
        let env = testing::mock_env();

        let bank = BankStub::with_view(MockBankView::only_balance(initial_lpp_balance.into()));
        setup_storage(&mut store, &bank, BoundToHundredPercent::ZERO); //

        if !initial_lpp_balance.into().is_zero() {
            lender::try_deposit::<TheCurrency, _>(
                &mut store,
                &bank,
                env.clone(),
                test_tools::lender_msg_with_funds(initial_lpp_balance),
            )
            .unwrap();
        }

        Config::update_min_utilization(&mut store, min_utilization).unwrap();
        f(store, bank, env)
    }

    fn setup_storage<Bank>(
        mut storage: &mut dyn Storage,
        bank: &Bank,
        min_utilization: BoundToHundredPercent,
    ) where
        Bank: BankAccountView,
    {
        ContractOwnerAccess::new(storage.deref_mut())
            .grant_to(&Addr::unchecked("admin"))
            .unwrap();

        LiquidityPool::<TheCurrency, _>::new(
            ApiConfig::new(
                Code::unchecked(0xDEADC0DE_u64),
                InterestRate::new(
                    BASE_INTEREST_RATE,
                    UTILIZATION_OPTIMAL,
                    ADDON_OPTIMAL_INTEREST_RATE,
                )
                .expect("Couldn't construct interest rate value!"),
                min_utilization,
            ),
            bank,
        )
        .save(storage)
        .unwrap();
    }

    mod deposit_withdraw_price {
        use finance::coin::{Amount, Coin};

        use lpp_platform::NLpn;

        use super::TheCurrency;

        const TEST_AMOUNT: Amount = 100;
        const DEPOSIT_LPP: Coin<TheCurrency> = Coin::new(TEST_AMOUNT);
        // as long as there is no interest amount paid, the NLPN price stays equal to 1 LPN
        const DEPOSIT_NLPN: Coin<NLpn> = Coin::new(TEST_AMOUNT);

        mod deposit {
            use finance::coin::Coin;

            use crate::contract::{
                lender::{
                    self,
                    test::{self, DEFAULT_MIN_UTILIZATION, deposit_withdraw_price::DEPOSIT_NLPN},
                },
                test as test_tools,
            };

            use super::{DEPOSIT_LPP, TheCurrency};

            #[test]
            fn test_deposit_zero() {
                test::test_case(0, DEFAULT_MIN_UTILIZATION, |mut store, bank, env| {
                    lender::try_deposit::<TheCurrency, _>(
                        &mut store,
                        &bank,
                        env,
                        test_tools::lender_msg_no_funds(),
                    )
                    .unwrap_err();
                })
            }

            #[test]
            fn test_deposit() {
                test::test_case(
                    DEPOSIT_LPP,
                    DEFAULT_MIN_UTILIZATION,
                    |store, _bank, _env| {
                        assert_eq!(
                            Coin::from(
                                lender::query_balance(&store, test_tools::lender())
                                    .unwrap()
                                    .balance
                                    .u128()
                            ),
                            DEPOSIT_NLPN
                        );
                    },
                )
            }
        }

        mod withdraw {
            use finance::{coin::Coin, zero::Zero};
            use lpp_platform::NLpn;

            use crate::contract::{
                lender::{
                    self,
                    test::{self, DEFAULT_MIN_UTILIZATION, deposit_withdraw_price::DEPOSIT_NLPN},
                },
                test as test_tools,
            };

            use super::{DEPOSIT_LPP, TheCurrency};

            #[test]
            fn test_withdraw_zero() {
                test::test_case(
                    DEPOSIT_LPP,
                    DEFAULT_MIN_UTILIZATION,
                    |mut store, bank, env| {
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            bank,
                            env,
                            test_tools::lender_msg_no_funds(),
                            Coin::ZERO,
                        )
                        .unwrap_err();
                    },
                )
            }

            #[test]
            fn test_partial_withdraw() {
                const WITHDRAWN: Coin<NLpn> = DEPOSIT_NLPN.checked_div(2).unwrap();
                const LEFTOVER: Coin<NLpn> = DEPOSIT_NLPN.checked_sub(WITHDRAWN).unwrap();

                test::test_case(
                    DEPOSIT_LPP,
                    DEFAULT_MIN_UTILIZATION,
                    |mut store, bank, env| {
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            bank,
                            env,
                            test_tools::lender_msg_no_funds(),
                            WITHDRAWN,
                        )
                        .unwrap();

                        assert_eq!(
                            Coin::new(
                                lender::query_balance(&store, test_tools::lender())
                                    .unwrap()
                                    .balance
                                    .u128()
                            ),
                            LEFTOVER
                        );
                    },
                )
            }

            #[test]
            fn test_full_withdraw() {
                test::test_case(
                    DEPOSIT_LPP,
                    DEFAULT_MIN_UTILIZATION,
                    |mut store, bank, env| {
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            bank,
                            env,
                            test_tools::lender_msg_no_funds(),
                            DEPOSIT_NLPN,
                        )
                        .unwrap();

                        assert!(
                            Coin::<TheCurrency>::new(
                                lender::query_balance(&store, test_tools::lender())
                                    .unwrap()
                                    .balance
                                    .u128()
                            )
                            .is_zero()
                        );
                    },
                )
            }

            #[test]
            fn test_overwithdraw() {
                test::test_case(
                    DEPOSIT_LPP,
                    DEFAULT_MIN_UTILIZATION,
                    |mut store, bank, env| {
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            bank,
                            env,
                            test_tools::lender_msg_no_funds(),
                            DEPOSIT_NLPN + Coin::new(1),
                        )
                        .unwrap_err();
                    },
                )
            }
        }

        mod nlpn_price {
            use crate::contract::lender::{
                self,
                test::{self, DEFAULT_MIN_UTILIZATION, deposit_withdraw_price::DEPOSIT_NLPN},
            };
            use finance::{
                coin::Coin,
                price::{self, Price},
            };
            use platform::bank::testing::MockBankView;

            use super::{DEPOSIT_LPP, TheCurrency};

            #[test]
            fn test_nlpn_price() {
                const INTEREST: Coin<TheCurrency> = DEPOSIT_LPP.checked_div(2).unwrap();

                test::test_case(DEPOSIT_LPP, DEFAULT_MIN_UTILIZATION, |store, bank, env| {
                    let now = &env.block.time;
                    assert_eq!(
                        lender::query_ntoken_price::<TheCurrency, _>(&store, &bank, now)
                            .unwrap()
                            .0,
                        Price::identity(),
                    );

                    let bank_got_interest = MockBankView::<TheCurrency, TheCurrency>::only_balance(
                        DEPOSIT_LPP + INTEREST,
                    );

                    assert_eq!(
                        price::total_of(DEPOSIT_NLPN).is(DEPOSIT_LPP + INTEREST),
                        lender::query_ntoken_price::<TheCurrency, _>(
                            &store,
                            &bank_got_interest,
                            now
                        )
                        .unwrap()
                        .0,
                    );
                })
            }
        }
    }

    mod min_utilization {
        use finance::{
            coin::Amount,
            percent::{Percent, bound::BoundToHundredPercent},
        };
        use platform::bank::testing::MockBankView;
        use sdk::cosmwasm_std::Addr;

        use crate::contract::{
            ContractError,
            lender::{self, test},
            test as test_tools,
        };

        use super::{LiquidityPool, TheCurrency};

        const FIFTY: Percent = Percent::from_permille(500);

        fn test_case(
            lpp_balance_at_deposit: Amount,
            borrowed: Amount,
            deposit: Amount,
            min_utilization: Percent,
            expect_error: bool,
        ) {
            debug_assert!(deposit != 0);
            test::test_case(
                lpp_balance_at_deposit + borrowed,
                BoundToHundredPercent::try_from_percent(min_utilization).unwrap(),
                |mut store, bank, env| {
                    if borrowed != 0 {
                        LiquidityPool::<'_, TheCurrency, _>::load(&store, &bank)
                            .unwrap()
                            .try_open_loan(
                                &mut store,
                                env.block.time,
                                Addr::unchecked("lease"),
                                borrowed.into(),
                            )
                            .unwrap();
                    }

                    //do deposit
                    let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(
                        (lpp_balance_at_deposit + deposit).into(),
                    );

                    let result = lender::try_deposit::<TheCurrency, _>(
                        &mut store,
                        &bank,
                        env,
                        test_tools::lender_msg_with_funds(deposit),
                    );

                    if expect_error {
                        assert_eq!(
                            ContractError::UtilizationBelowMinimalRates,
                            result.unwrap_err()
                        );
                    } else {
                        assert!(result.is_ok(), "{result:#?}");
                    }
                },
            )
        }

        #[test]
        fn test_no_leases() {
            test_case(0, 0, 100, FIFTY, true);
        }

        #[test]
        fn test_below_before_deposit() {
            test_case(100, 0, 100, FIFTY, true);
        }

        #[test]
        fn test_below_on_pending_deposit() {
            test_case(50, 50, 100, FIFTY, true);
        }

        #[test]
        fn test_at_limit_on_pending_deposit() {
            test_case(0, 50, 50, FIFTY, false);
        }

        #[test]
        fn test_at_limit_after_deposit() {
            test_case(0, 50, 50, FIFTY, false);
        }

        #[test]
        fn test_above_after_deposit() {
            test_case(0, 100, 50, FIFTY, false);
        }

        #[test]
        fn test_uncapped() {
            test_case(50, 0, 50, Percent::ZERO, false);
        }
    }
}
