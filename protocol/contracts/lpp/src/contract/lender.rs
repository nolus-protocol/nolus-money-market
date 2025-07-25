use currency::{CurrencyDef, platform::Nls};
use finance::{
    coin::{Amount, Coin},
    zero::Zero,
};
use lpp_platform::NLpn;
use platform::{
    bank::{BankAccount, BankAccountView},
    batch::Batch,
};
use sdk::cosmwasm_std::{Addr, Storage, Timestamp};

use crate::{
    lpp::LiquidityPool,
    msg::{BalanceResponse, PriceResponse},
    state::{Config, Deposit},
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
    Lpn: 'static + CurrencyDef,
    Bank: BankAccountView,
{
    Config::load(storage)
        .and_then(|config| {
            LiquidityPool::load(storage, &config, bank).and_then(|mut lpp| {
                lpp.deposit(pending_deposit, now)
                    .and_then(|receipts| lpp.save(storage).map(|()| receipts))
            })
        })
        .and_then(|receipts| {
            Deposit::load_or_default(storage, lender.clone())?.deposit(storage, receipts)
        })
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
    Config::load(storage).and_then(|config| {
        LiquidityPool::load(storage, &config, bank)
            .and_then(|lpp| lpp.deposit_capacity(now, Coin::ZERO))
    })
}

/// Withdraw receipts and return the returned amount, potentially some unclaimed `Nls` rewards, and their transfer messages
pub(super) fn try_withdraw<Lpn, Bank>(
    storage: &mut dyn Storage,
    mut bank: Bank,
    lender: Addr,
    amount: Coin<NLpn>,
    now: &Timestamp,
) -> Result<(Coin<Lpn>, Option<Coin<Nls>>, Batch)>
where
    Lpn: CurrencyDef,
    Bank: BankAccount,
{
    if amount.is_zero() {
        return Err(ContractError::ZeroWithdrawFunds);
    }

    let payment_lpn = Config::load(storage).and_then(|config| {
        LiquidityPool::load(storage, &config, &bank).and_then(|mut lpp| {
            lpp.withdraw_lpn(amount, now)
                .and_then(|payment_lpn| lpp.save(storage).map(|()| payment_lpn))
        })
    })?;

    let maybe_reward = Deposit::may_load(storage, lender.clone())?
        .ok_or(ContractError::NoDeposit {})?
        .withdraw(storage, amount)?;

    bank.send(payment_lpn, lender.clone());

    if let Some(reward) = maybe_reward {
        if !reward.is_zero() {
            bank.send(reward, lender.clone());
        }
    }

    Ok((payment_lpn, maybe_reward, bank.into()))
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
    Deposit::load_or_default(storage, addr)
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
        bank::{BankAccountView, BankStub, testing::MockBankView},
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
        F: FnOnce(
            MockStorage,
            ApiConfig,
            BankStub<MockBankView<TheCurrency, TheCurrency>>,
            Timestamp,
        ),
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
        let bank = BankStub::with_view(MockBankView::only_balance(initial_lpp_balance.into()));
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
            use finance::{coin::Coin, zero::Zero};

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
                    DEPOSIT_LPP,
                    DEFAULT_MIN_UTILIZATION,
                    |store, _config, _bank, _now| {
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
                    |mut store, _config, bank, now| {
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            bank,
                            test_tools::lender(),
                            Coin::ZERO,
                            &now,
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
                    |mut store, _config, bank, now| {
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            bank,
                            test_tools::lender(),
                            WITHDRAWN,
                            &now,
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
                    |mut store, _config, bank, now| {
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            bank,
                            test_tools::lender(),
                            DEPOSIT_NLPN,
                            &now,
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
                    |mut store, _config, bank, now| {
                        lender::try_withdraw::<TheCurrency, _>(
                            &mut store,
                            bank,
                            test_tools::lender(),
                            DEPOSIT_NLPN + Coin::new(1),
                            &now,
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

                test::test_case(
                    DEPOSIT_LPP,
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
                                DEPOSIT_LPP + INTEREST,
                            );

                        assert_eq!(
                            price::total_of(DEPOSIT_NLPN).is(DEPOSIT_LPP + INTEREST),
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
    }

    mod min_utilization {
        use finance::{
            coin::{Amount, Coin},
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
                |mut store, config, bank, now| {
                    if borrowed != 0 {
                        let mut lpp = LiquidityPool::load(&store, &config, &bank).unwrap();
                        lpp.try_open_loan(
                            &mut store,
                            now,
                            Addr::unchecked("lease"),
                            Coin::<TheCurrency>::from(borrowed),
                        )
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
