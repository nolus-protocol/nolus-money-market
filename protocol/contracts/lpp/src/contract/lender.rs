use currency::{CurrencyDef, platform::Nls};
use finance::{
    coin::{Amount, Coin},
    zero::Zero,
};
use lpp_platform::NLpn;
use platform::{
    bank::{self, BankAccount, BankAccountView},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Storage, Timestamp};

use crate::{
    event::WithdrawEmitter,
    lpp::LiquidityPool,
    msg::{BalanceResponse, PriceResponse},
    state::{Config, Deposit, TotalRewards},
};

use super::error::{ContractError, Result};

/// Deposit `Lpn`-s and return the amount of receipts
pub(super) fn try_deposit<Lpn, Bank>(
    storage: &mut dyn Storage,
    bank: &Bank,
    lender: Addr,
    pending_deposit: Coin<Lpn>,
    now: &Timestamp,
) -> Result<Coin<NLpn>>
where
    Lpn: CurrencyDef,
    Bank: BankAccountView,
{
    if pending_deposit.is_zero() {
        return Err(ContractError::ZeroDepositFunds);
    }

    Config::load(storage)
        .and_then(|config| {
            LiquidityPool::load(storage, &config, bank).and_then(|mut lpp| {
                lpp.deposit(pending_deposit, now)
                    .and_then(|receipts| lpp.save(storage).map(|()| receipts))
            })
        })
        .and_then(|receipts| {
            TotalRewards::load_or_default(storage)
                .and_then(|total_rewards| {
                    Deposit::load_or_default(storage, lender.clone(), total_rewards).and_then(
                        |mut deposit| {
                            deposit.deposit(receipts);
                            deposit.save(storage)
                        },
                    )
                })
                .map(|()| receipts)
        })
}

pub(super) fn deposit_capacity<Lpn, Bank>(
    storage: &dyn Storage,
    bank: &Bank,
    now: &Timestamp,
) -> Result<Option<Coin<Lpn>>>
where
    Lpn: CurrencyDef,
    Bank: BankAccountView,
{
    Config::load(storage).and_then(|config| {
        LiquidityPool::load(storage, &config, bank)
            .and_then(|lpp| lpp.deposit_capacity(now, Coin::ZERO))
    })
}

/// Withdraw receipts and return the returned amount, potentially some unclaimed `Nls` rewards, and their transfer messages
///
/// Note: Due to the asynchronous execution of bank transfers this operation should be run at most once at a transaction.
/// Should one need more times then balance adjustment/forecasting is needed, e.g. subtracting the transfer from the balance,
/// or exposing the notion of 'commited' and 'uncommited' amounts at this level.
pub(super) fn try_withdraw<Lpn, Bank>(
    storage: &mut dyn Storage,
    mut pool_account: Bank,
    lender: Addr,
    receipts: Coin<NLpn>,
    now: &Timestamp,
    mut emitter: WithdrawEmitter<'_, Lpn>,
) -> Result<MessageResponse>
where
    Lpn: CurrencyDef,
    Bank: BankAccount,
{
    if receipts.is_zero() {
        return Err(ContractError::ZeroWithdrawFunds);
    }

    Config::load(storage)
        .and_then(|config| {
            TotalRewards::load_or_default(storage).and_then(|total_rewards| {
                LiquidityPool::<Lpn, _>::load(storage, &config, &pool_account).and_then(
                    |mut lpp| {
                        Deposit::load(storage, lender.clone(), total_rewards)
                            .and_then(|mut deposit| {
                                withdraw(&mut deposit, &mut lpp, receipts, now, Coin::ZERO)
                                    .and_then(|(payment_out, may_reward)| {
                                        deposit.save(storage).map(|()| (payment_out, may_reward))
                                    })
                            })
                            .and_then(|(payment_out, may_reward)| {
                                lpp.save(storage).map(|()| (payment_out, may_reward))
                            })
                    },
                )
            })
        })
        .map(|(payment_out, may_reward)| {
            transfer_to(lender.clone(), payment_out, may_reward, &mut pool_account);

            emitter.on_withdraw(lender, receipts, payment_out, may_reward)
        })
        .map(|()| MessageResponse::messages_with_events(pool_account.into(), emitter))
}

pub(super) fn try_close_all<Lpn, BankView, Bank>(
    storage: &mut dyn Storage,
    pool_view: BankView, // acceptable trick since the bank transfers get visible after the ransaction finishes
    mut pool_account: Bank, // enable transfer scheduling while in process of reading balances
    now: &Timestamp,
    mut emitter: WithdrawEmitter<'_, Lpn>,
) -> Result<MessageResponse>
where
    Lpn: CurrencyDef,
    BankView: BankAccountView,
    Bank: BankAccount,
{
    Config::load(storage).and_then(|config| {
        TotalRewards::load_or_default(storage).and_then(|total_rewards| {
            LiquidityPool::<Lpn, _>::load(storage, &config, &bank::cache::<Lpn, _>(pool_view))
                .and_then(|mut lpp| {
                    let (deposits, _) = Deposit::iter(storage, total_rewards).try_fold(
                        (Vec::<Deposit>::default(), Coin::ZERO),
                        |(mut deposits, pending_withdraw), may_deposit| {
                            may_deposit.and_then(|mut deposit| {
                                let receipts = deposit.receipts();
                                withdraw(&mut deposit, &mut lpp, receipts, now, pending_withdraw)
                                    .map(|(payment_out, may_reward)| {
                                        transfer_to(
                                            deposit.owner().clone(),
                                            payment_out,
                                            may_reward,
                                            &mut pool_account,
                                        );
                                        emitter.on_withdraw(
                                            deposit.owner().clone(),
                                            receipts,
                                            payment_out,
                                            may_reward,
                                        );
                                        deposits.push(deposit);
                                        (deposits, pending_withdraw + payment_out)
                                    })
                            })
                        },
                    )?;

                    deposits
                        .into_iter()
                        .try_for_each(|deposit| deposit.save(storage))
                        .map(|()| {
                            MessageResponse::messages_with_events(pool_account.into(), emitter)
                        })
                        .and_then(|resp| lpp.save(storage).map(|()| resp))
                })
        })
    })
}

fn withdraw<Lpn, Bank>(
    deposit: &mut Deposit,
    lpp: &mut LiquidityPool<'_, '_, Lpn, Bank>,
    receipts: Coin<NLpn>,
    now: &Timestamp,
    pending_withdraw: Coin<Lpn>,
) -> Result<(Coin<Lpn>, Option<Coin<Nls>>)>
where
    Lpn: CurrencyDef,
    Bank: BankAccountView,
{
    deposit.withdraw(receipts).and_then(|may_rewards| {
        lpp.withdraw_lpn(receipts, pending_withdraw, now)
            .map(|payment_lpn| (payment_lpn, may_rewards))
    })
}

fn transfer_to<Lpn, Bank>(
    lender: Addr,
    payment_out: Coin<Lpn>,
    may_reward: Option<Coin<Nls>>,
    pool_account: &mut Bank,
) where
    Lpn: CurrencyDef,
    Bank: BankAccount,
{
    pool_account.send(payment_out, lender.clone());

    match may_reward {
        Some(reward) if !reward.is_zero() => pool_account.send(reward, lender),
        _ => {}
    }
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
    Config::load(storage)
        .and_then(|config| {
            LiquidityPool::load(storage, &config, bank)
                .and_then(|lpp| lpp.calculate_price(now, Coin::ZERO))
        })
        .map(PriceResponse)
}

pub fn query_balance(storage: &dyn Storage, addr: Addr) -> Result<BalanceResponse> {
    TotalRewards::load_or_default(storage)
        .and_then(|total_rewards| Deposit::load_or_default(storage, addr, total_rewards))
        .map(|ref deposit| deposit.receipts())
        .map(|receipts| BalanceResponse {
            balance: Amount::from(receipts).into(),
        })
}

#[cfg(test)]
mod test {
    use finance::{
        coin::Coin,
        percent::{Percent, bound::BoundToHundredPercent},
    };
    use platform::{
        bank::{BankAccountView, testing::MockBankView},
        contract::Code,
    };
    use sdk::cosmwasm_std::{
        Storage, Timestamp,
        testing::{self, MockStorage},
    };

    use crate::{
        borrow::InterestRate,
        config::Config as ApiConfig,
        contract::{
            lender,
            test::{self as test_tools, TheCurrency},
        },
        state::{Config, TotalRewards},
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
        F: FnOnce(MockStorage, ApiConfig, MockBankView<TheCurrency, TheCurrency>, Timestamp),
    {
        let mut store = testing::MockStorage::default();
        let now = Timestamp::from_nanos(1_571_897_419_879_405_538);

        let config = ApiConfig::new(
            Code::unchecked(0xDEADC0DE_u64),
            InterestRate::new(
                BASE_INTEREST_RATE,
                UTILIZATION_OPTIMAL,
                ADDON_OPTIMAL_INTEREST_RATE,
            )
            .expect("Couldn't construct interest rate value!"),
            BoundToHundredPercent::ZERO,
        );
        let bank = MockBankView::only_balance(initial_lpp_balance.into());
        setup_storage(&mut store, &config, &bank);

        if !initial_lpp_balance.into().is_zero() {
            lender::try_deposit::<TheCurrency, _>(
                &mut store,
                &bank,
                test_tools::lender(),
                initial_lpp_balance.into(),
                &now,
            )
            .unwrap();
        }

        let config_custom =
            ApiConfig::new(config.lease_code(), *config.borrow_rate(), min_utilization);
        Config::store(&config_custom, &mut store).unwrap();
        f(store, config_custom, bank, now)
    }

    fn setup_storage<Bank>(storage: &mut dyn Storage, config: &ApiConfig, bank: &Bank)
    where
        Bank: BankAccountView,
    {
        Config::store(config, storage).unwrap();
        LiquidityPool::<TheCurrency, _>::new(config, bank)
            .save(storage)
            .unwrap();
        TotalRewards::save(&TotalRewards::load_or_default(storage).unwrap(), storage).unwrap();
    }

    mod deposit_withdraw_price {
        use finance::coin::{Amount, Coin};
        use sdk::cosmwasm_std::{Addr, Storage};

        use lpp_platform::NLpn;

        use crate::contract::lender;

        use super::TheCurrency;

        const TEST_AMOUNT: Amount = 100;
        const DEPOSIT_LPN: Coin<TheCurrency> = Coin::new(TEST_AMOUNT);
        // as long as there is no interest amount paid, the NLPN price stays equal to 1 LPN
        const DEPOSIT_NLPN: Coin<NLpn> = Coin::new(TEST_AMOUNT);

        mod deposit {
            use finance::{
                coin::Coin,
                zero::Zero,
            };

            use crate::contract::{
                lender::{
                    self,
                    test::{
                        self, DEFAULT_MIN_UTILIZATION,
                        deposit_withdraw_price::{DEPOSIT_NLPN, query_balance},
                    },
                },
                test as test_tools,
            };

            use super::{DEPOSIT_LPN, TheCurrency};

            #[test]
            fn test_deposit_zero() {
                test::test_case(
                    0,
                    DEFAULT_MIN_UTILIZATION,
                    |mut store, _config, bank, now| {
                        lender::try_deposit::<TheCurrency, _>(
                            &mut store,
                            &bank,
                            test_tools::lender(),
                            Coin::ZERO,
                            &now,
                        )
                        .unwrap_err();
                    },
                )
            }

            #[test]
            fn test_deposit() {
                test::test_case(
                    DEPOSIT_LPN,
                    DEFAULT_MIN_UTILIZATION,
                    |store, _config, _bank, _now| {
                        assert_eq!(query_balance(&store, test_tools::lender()), DEPOSIT_NLPN)
                    },
                )
            }
        }

        mod withdraw {
            use currency::platform::Nls;
            use finance::{
                coin::Coin,
                zero::Zero,
            };
            use lpp_platform::NLpn;
            use platform::bank::{
                BankAccountView,
                testing::{self, MockBankView},
            };
            use sdk::cosmwasm_std::{Addr, testing as sdk_testing};

            use crate::{
                contract::{
                    ContractError,
                    lender::{
                        self,
                        test::{
                            self, DEFAULT_MIN_UTILIZATION,
                            deposit_withdraw_price::{
                                DEPOSIT_NLPN, TEST_AMOUNT, query_balance,
                                query_balance_the_currency,
                            },
                        },
                    },
                    test as test_tools,
                },
                event::WithdrawEmitter,
                lpp::LiquidityPool,
                state::TotalRewards,
            };

            use super::{DEPOSIT_LPN, TheCurrency};

            #[test]
            fn test_withdraw_zero() {
                test::test_case(
                    DEPOSIT_LPN,
                    DEFAULT_MIN_UTILIZATION,
                    |mut store, _config, bank, now| {
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            testing::no_transfers(bank),
                            test_tools::lender(),
                            Coin::ZERO,
                            &now,
                            WithdrawEmitter::new(&sdk_testing::mock_env()),
                        )
                        .unwrap_err();
                    },
                )
            }

            #[test]
            fn test_partial_withdraw() {
                const WITHDRAWN: Coin<NLpn> = Coin::new(TEST_AMOUNT.checked_div(2).unwrap());
                const WITHDRAWN_LPN: Coin<TheCurrency> =
                    Coin::new(TEST_AMOUNT.checked_div(2).unwrap());
                const LEFTOVER: Coin<NLpn> = DEPOSIT_NLPN.checked_sub(WITHDRAWN).unwrap();

                test::test_case(
                    DEPOSIT_LPN,
                    DEFAULT_MIN_UTILIZATION,
                    |mut store, _config, bank, now| {
                        let lender = test_tools::lender();
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            testing::one_transfer(WITHDRAWN_LPN, lender.clone(), bank),
                            lender.clone(),
                            WITHDRAWN,
                            &now,
                            WithdrawEmitter::new(&sdk_testing::mock_env()),
                        )
                        .unwrap();

                        assert_eq!(query_balance(&store, lender), LEFTOVER);
                    },
                )
            }

            #[test]
            fn test_full_withdraw() {
                test::test_case(
                    DEPOSIT_LPN,
                    DEFAULT_MIN_UTILIZATION,
                    |mut store, _config, bank, now| {
                        let lender = test_tools::lender();
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            testing::one_transfer(DEPOSIT_LPN, lender.clone(), bank),
                            lender,
                            DEPOSIT_NLPN,
                            &now,
                            WithdrawEmitter::new(&sdk_testing::mock_env()),
                        )
                        .unwrap();

                        assert!(query_balance_the_currency(&store, test_tools::lender()).is_zero());
                    },
                )
            }

            #[test]
            fn test_full_withdraw_reward() {
                const REWARDS: Coin<Nls> = Coin::new(422);

                test::test_case(
                    DEPOSIT_LPN,
                    DEFAULT_MIN_UTILIZATION,
                    |mut store, _config, bank, now| {
                        let rewards = TotalRewards::load_or_default(&store)
                            .unwrap()
                            .add(REWARDS, DEPOSIT_NLPN);
                        TotalRewards::save(&rewards, &mut store).unwrap();

                        let lender = test_tools::lender();
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            testing::two_transfers(
                                DEPOSIT_LPN,
                                lender.clone(),
                                REWARDS,
                                lender.clone(),
                                bank,
                            ),
                            lender,
                            DEPOSIT_NLPN,
                            &now,
                            WithdrawEmitter::new(&sdk_testing::mock_env()),
                        )
                        .unwrap();

                        assert!(query_balance_the_currency(&store, test_tools::lender()).is_zero());
                    },
                )
            }

            #[test]
            fn test_full_withdraw_interest() {
                const INTEREST_TOTAL: Coin<TheCurrency> = Coin::new(48);
                const INTEREST_A_DEPOSIT: Coin<TheCurrency> =
                    INTEREST_TOTAL.checked_div(2).unwrap();
                let other_lender = Addr::unchecked("other_lender");
                test::test_case(
                    DEPOSIT_LPN,
                    DEFAULT_MIN_UTILIZATION,
                    |mut store, _config, bank, now| {
                        let lender = test_tools::lender();
                        let plus_deposit = MockBankView::<_, TheCurrency>::only_balance(
                            bank.balance::<TheCurrency>().unwrap() + DEPOSIT_LPN,
                        );

                        lender::try_deposit(
                            &mut store,
                            &plus_deposit,
                            other_lender.clone(),
                            DEPOSIT_LPN,
                            &now,
                        )
                        .unwrap();

                        let plus_deposit_and_interest =
                            MockBankView::<_, TheCurrency>::only_balance(
                                plus_deposit.balance().unwrap() + INTEREST_TOTAL,
                            );
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            testing::one_transfer(
                                DEPOSIT_LPN + INTEREST_A_DEPOSIT,
                                lender.clone(),
                                plus_deposit_and_interest.clone(),
                            ),
                            lender,
                            DEPOSIT_NLPN,
                            &now,
                            WithdrawEmitter::new(&sdk_testing::mock_env()),
                        )
                        .unwrap();

                        assert!(query_balance_the_currency(&store, test_tools::lender()).is_zero());

                        //simulate finality of the previous withdraw
                        let past_withdrawal = MockBankView::<_, TheCurrency>::only_balance(
                            plus_deposit_and_interest.balance().unwrap()
                                - DEPOSIT_LPN
                                - INTEREST_A_DEPOSIT,
                        );
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            testing::one_transfer(
                                DEPOSIT_LPN + INTEREST_A_DEPOSIT,
                                other_lender.clone(),
                                past_withdrawal,
                            ),
                            other_lender,
                            DEPOSIT_NLPN,
                            &now,
                            WithdrawEmitter::new(&sdk_testing::mock_env()),
                        )
                        .unwrap();
                    },
                )
            }

            #[test]
            fn test_overwithdraw() {
                test::test_case(
                    DEPOSIT_LPN,
                    DEFAULT_MIN_UTILIZATION,
                    |mut store, _config, bank, now| {
                        assert_eq!(
                            ContractError::InsufficientBalance {},
                            lender::try_withdraw::<TheCurrency, _>(
                                &mut store,
                                testing::no_transfers(bank),
                                test_tools::lender(),
                                DEPOSIT_NLPN + Coin::new(1),
                                &now,
                                WithdrawEmitter::new(&sdk_testing::mock_env()),
                            )
                            .unwrap_err()
                        );
                    },
                )
            }

            #[test]
            fn test_no_liquidity() {
                const LOAN_AMOUNT: Coin<TheCurrency> = Coin::new(1);

                test::test_case(
                    DEPOSIT_LPN,
                    DEFAULT_MIN_UTILIZATION,
                    |mut store, config, bank, now| {
                        let mut lpp = LiquidityPool::load(&store, &config, &bank).unwrap();
                        lpp.try_open_loan(now, LOAN_AMOUNT).unwrap();
                        lpp.save(&mut store).unwrap();

                        assert_eq!(
                            ContractError::NoLiquidity {},
                            lender::try_withdraw::<TheCurrency, _>(
                                &mut store,
                                testing::no_transfers(bank),
                                test_tools::lender(),
                                DEPOSIT_NLPN,
                                &now,
                                WithdrawEmitter::new(&sdk_testing::mock_env()),
                            )
                            .unwrap_err()
                        );
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

            use super::{DEPOSIT_LPN, TheCurrency};

            #[test]
            fn test_nlpn_price() {
                const INTEREST: Coin<TheCurrency> = DEPOSIT_LPN.checked_div(2).unwrap();

                test::test_case(
                    DEPOSIT_LPN,
                    DEFAULT_MIN_UTILIZATION,
                    |store, _config, bank, now| {
                        assert_eq!(
                            lender::query_ntoken_price::<TheCurrency, _>(&store, &bank, &now)
                                .unwrap()
                                .0,
                            Price::identity(),
                        );

                        let bank_got_interest =
                            MockBankView::<TheCurrency, TheCurrency>::only_balance(
                                DEPOSIT_LPN + INTEREST,
                            );

                        assert_eq!(
                            price::total_of(DEPOSIT_NLPN).is(DEPOSIT_LPN + INTEREST),
                            lender::query_ntoken_price::<TheCurrency, _>(
                                &store,
                                &bank_got_interest,
                                &now
                            )
                            .unwrap()
                            .0,
                        );
                    },
                )
            }
        }

        mod close_all {
            use finance::coin::Coin;
            use platform::bank::{
                BankAccountView,
                testing::{self, MockBankView},
            };
            use sdk::cosmwasm_std::{Addr, testing as sdk_testing};

            use crate::{
                contract::{
                    lender::{
                        self,
                        test::{
                            self, DEFAULT_MIN_UTILIZATION,
                            deposit_withdraw_price::{DEPOSIT_LPN, query_balance_the_currency},
                        },
                    },
                    test::{self as test_tools, TheCurrency},
                },
                event::WithdrawEmitter,
            };

            #[test]
            fn test_close_all() {
                const INTEREST_TOTAL: Coin<TheCurrency> = Coin::new(48);
                const INTEREST_A_DEPOSIT: Coin<TheCurrency> =
                    INTEREST_TOTAL.checked_div(2).unwrap();
                let other_lender = Addr::unchecked("other_lender");
                test::test_case(
                    DEPOSIT_LPN,
                    DEFAULT_MIN_UTILIZATION,
                    |mut store, _config, bank, now| {
                        let lender = test_tools::lender();
                        let plus_deposit = MockBankView::<_, TheCurrency>::only_balance(
                            bank.balance::<TheCurrency>().unwrap() + DEPOSIT_LPN,
                        );

                        lender::try_deposit(
                            &mut store,
                            &plus_deposit,
                            other_lender.clone(),
                            DEPOSIT_LPN,
                            &now,
                        )
                        .unwrap();

                        let plus_deposit_and_interest =
                            MockBankView::<_, TheCurrency>::only_balance(
                                plus_deposit.balance().unwrap() + INTEREST_TOTAL,
                            );
                        lender::try_close_all::<TheCurrency, _, _>(
                            &mut store,
                            plus_deposit_and_interest.clone(),
                            testing::two_transfers(
                                DEPOSIT_LPN + INTEREST_A_DEPOSIT,
                                lender.clone(),
                                DEPOSIT_LPN + INTEREST_A_DEPOSIT,
                                other_lender.clone(),
                                plus_deposit_and_interest,
                            ),
                            &now,
                            WithdrawEmitter::new(&sdk_testing::mock_env()),
                        )
                        .unwrap();

                        assert!(query_balance_the_currency(&store, test_tools::lender()).is_zero());
                        assert!(query_balance_the_currency(&store, other_lender).is_zero());
                    },
                )
            }
        }

        pub(super) fn query_balance<C>(storage: &dyn Storage, addr: Addr) -> Coin<C> {
            Coin::<C>::new(Amount::from(
                lender::query_balance(storage, addr).unwrap().balance,
            ))
        }

        pub(super) fn query_balance_the_currency(
            storage: &dyn Storage,
            addr: Addr,
        ) -> Coin<TheCurrency> {
            query_balance(storage, addr)
        }
    }

    mod min_utilization {
        use finance::{
            coin::{Amount, Coin},
            percent::{Percent, bound::BoundToHundredPercent},
        };
        use platform::bank::testing::MockBankView;

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
                |mut store, config, bank, now| {
                    if borrowed != 0 {
                        let mut lpp = LiquidityPool::load(&store, &config, &bank).unwrap();
                        lpp.try_open_loan(now, Coin::<TheCurrency>::from(borrowed))
                            .unwrap();
                        lpp.save(&mut store).unwrap();
                    }

                    //do deposit
                    let bank = MockBankView::<TheCurrency, TheCurrency>::only_balance(
                        (lpp_balance_at_deposit + deposit).into(),
                    );

                    let result = lender::try_deposit::<TheCurrency, _>(
                        &mut store,
                        &bank,
                        test_tools::lender(),
                        deposit.into(),
                        &now,
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
