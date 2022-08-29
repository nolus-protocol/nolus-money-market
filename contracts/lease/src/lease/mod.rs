use cosmwasm_std::{Addr, QuerierWrapper, Reply, Timestamp, wasm_execute, WasmMsg};
use serde::Serialize;

use contract_constants::LeaseReplyId;
use finance::{
    coin::{Amount, Coin},
    currency::{Currency, SymbolOwned},
    fraction::Fraction,
    liability::Liability,
    percent::{Percent, Units},
    price::{
        Price,
        total,
        total_of
    },
    ratio::Ratio,
};
use lpp::stub::Lpp as LppTrait;
use market_price_oracle::msg::ExecuteMsg::AddPriceAlarm;
use platform::{
    bank::{BankAccount, BankAccountView},
    batch::{
        Batch,
        BatchMessage
    },
};

use crate::{
    error::{ContractError, ContractResult},
    loan::{Loan, Receipt},
    msg::StateResponse,
};
use crate::lease::liquidation::WarningAndPartialLiquidationInfo;

pub(super) use self::{
    downpayment_dto::DownpaymentDTO,
    dto::LeaseDTO,
    liquidation::LiquidationStatus,
};
use self::{
    factory::Factory,
    open::Result as OpenResult,
};

mod downpayment_dto;
mod dto;
mod factory;
mod open;
mod liquidation;

pub trait WithLease {
    type Output;
    type Error;

    fn exec<Lpn, Lpp>(self, lease: Lease<Lpn, Lpp>) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppTrait<Lpn>;

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error>;
}

pub fn execute<L, O, E>(dto: LeaseDTO, cmd: L, querier: &QuerierWrapper) -> Result<O, E>
where
    L: WithLease<Output = O, Error = E>,
{
    let lpp = dto.loan.lpp().clone();
    lpp.execute(Factory::new(cmd, dto), querier)
}

pub struct Lease<Lpn, Lpp> {
    customer: Addr,
    currency: SymbolOwned,
    liability: Liability,
    loan: Loan<Lpn, Lpp>,
    market_price_oracle: Addr,
}

impl<Lpn, Lpp> Lease<Lpn, Lpp>
where
    Lpn: Currency,
    Lpp: LppTrait<Lpn>,
{
    pub(super) fn from_dto(dto: LeaseDTO, lpp: Lpp) -> Self {
        assert_eq!(
            Lpn::SYMBOL,
            dto.currency,
            "[Single currency version] The LPN '{}' should match the currency of the lease '{}'",
            Lpn::SYMBOL,
            dto.currency
        );

        Self {
            customer: dto.customer,
            currency: dto.currency,
            liability: dto.liability,
            loan: Loan::from_dto(dto.loan, lpp),
            market_price_oracle: dto.market_price_oracle,
        }
    }

    pub(super) fn into_dto(self) -> (LeaseDTO, Lpp) {
        let (loan_dto, lpp) = self.loan.into_dto();
        (
            LeaseDTO::new(
                self.customer,
                self.currency,
                self.liability,
                loan_dto,
                self.market_price_oracle,
            ),
            lpp,
        )
    }

    pub(crate) fn owned_by(&self, addr: &Addr) -> bool {
        &self.customer == addr
    }

    pub(crate) fn sent_oracle(&self, addr: &Addr) -> bool {
        &self.market_price_oracle == addr
    }

    pub(crate) fn open_loan_req(self, downpayment: Coin<Lpn>) -> ContractResult<Batch> {
        // TODO add a type parameter to this function to designate the downpayment currency
        // TODO query the market price oracle to get the price of the downpayment currency to LPN
        //  and calculate `downpayment` in LPN
        let borrow = self.liability.init_borrow_amount(downpayment);

        self.loan.open_loan_req(borrow).map_err(Into::into)
    }

    // TODO lease currency can be different than Lpn, therefore result's type parameter
    pub(crate) fn open_loan_resp<B>(self, lease: Addr, resp: Reply, account: B, now: &Timestamp) -> ContractResult<OpenResult<Lpn>>
    where
        B: BankAccountView,
    {
        let reschedule_msgs = self.initial_alarm_schedule(
            lease,
            account.balance()?,
            now,
            &LiquidationStatus::None,
        )?;

        let mut result = self.loan.open_loan_resp(resp)
            .map({
                // Force move before closure to avoid edition warning from clippy;
                let customer = self.customer;
                let currency = self.currency;

                |result| OpenResult {
                    batch: result.batch,
                    customer,
                    annual_interest_rate: result.annual_interest_rate,
                    currency,
                    loan_pool_id: result.loan_pool_id,
                    loan_amount: result.borrowed,
                }
            })?;

        reschedule_msgs.into_iter().for_each(|msg| result.batch.schedule_execute_batch_message(msg));

        Ok(result)
    }

    // TODO add the lease address as a field in Lease<>
    //  and populate it on LeaseDTO.execute as LeaseFactory
    pub(crate) fn close<B>(self, lease: Addr, mut account: B) -> ContractResult<Batch>
    where
        B: BankAccount,
    {
        let state = self.state(Timestamp::from_nanos(u64::MAX), &account, lease)?;
        match state {
            StateResponse::Opened { .. } => Err(ContractError::LoanNotPaid()),
            StateResponse::Paid(..) => {
                let balance = account.balance::<Lpn>()?;
                account.send(balance, &self.customer);

                Ok(account.into())
            }
            StateResponse::Closed() => Err(ContractError::LoanClosed()),
        }
    }

    pub(crate) fn repay(
        &mut self,
        payment: Coin<Lpn>,
        by: Timestamp,
        lease: Addr,
    ) -> ContractResult<Receipt<Lpn>> {
        assert_eq!(self.currency, Lpn::SYMBOL);
        self.loan.repay(payment, by, lease)
    }

    pub(crate) fn state<B>(
        &self,
        now: Timestamp,
        account: &B,
        lease: Addr,
    ) -> ContractResult<StateResponse<Lpn, Lpn>>
    where
        B: BankAccountView,
    {
        let lease_amount = account.balance::<Lpn>().map_err(ContractError::from)?;

        if lease_amount.is_zero() {
            Ok(StateResponse::Closed())
        } else {
            let loan_state = self.loan.state(now, lease)?;

            loan_state.map_or(Ok(StateResponse::Paid(lease_amount)), |state| {
                Ok(StateResponse::Opened {
                    amount: lease_amount,
                    interest_rate: state.annual_interest,
                    interest_rate_margin: state.annual_interest_margin,
                    principal_due: state.principal_due,
                    previous_margin_due: state.previous_margin_interest_due,
                    previous_interest_due: state.previous_interest_due,
                    current_margin_due: state.current_margin_interest_due,
                    current_interest_due: state.current_interest_due,
                })
            })
        }
    }

    #[inline]
    pub(crate) fn reschedule_from_price_alarm<A>(
        &self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
        liquidation: &LiquidationStatus<Lpn>,
    ) -> ContractResult<Vec<BatchMessage<WasmMsg, LeaseReplyId>>>
    where
        A: Into<Addr>,
    {
        Ok(vec![self.reschedule_price_alarm(lease, lease_amount, now, liquidation)?])
    }

    #[inline]
    pub(crate) fn reschedule_from_repay<A>(
        &self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
        liquidation: &LiquidationStatus<Lpn>,
    ) -> ContractResult<Vec<BatchMessage<WasmMsg, LeaseReplyId>>>
        where
            A: Into<Addr>,
    {
        // Reasoning: "reschedule_from_price_alarm" removes current time alarm,
        // adds a new one, and then updates the price alarm.
        self.reschedule_from_price_alarm(lease, lease_amount, now, liquidation)
    }

    pub(crate) fn run_liquidation<B>(
        &self,
        now: Timestamp,
        account: &B,
        lease: Addr,
        price: Price<Lpn, Lpn>,
    ) -> ContractResult<(LiquidationStatus<Lpn>, Coin<Lpn>)>
    where
        B: BankAccountView,
    {
        let lease_amount = account.balance::<Lpn>().map_err(ContractError::from)?;

        let status = self.liquidation_status(now, lease, lease_amount, price)?;

        // TODO run liquidation

        Ok((status, lease_amount))
    }

    fn liquidation_status(
        &self,
        now: Timestamp,
        lease: Addr,
        lease_amount: Coin<Lpn>,
        market_price: Price<Lpn, Lpn>,
    ) -> ContractResult<LiquidationStatus<Lpn>> {
        self.liability.invariant_held()?;

        Ok(if lease_amount.is_zero() {
            LiquidationStatus::None
        } else {
            let loan_state = self.loan.state(now, lease)?;

            loan_state.map_or(
                LiquidationStatus::None,
                |state| {
                    let liability_lpn = state.principal_due
                        + state.previous_margin_interest_due
                        + state.previous_interest_due
                        + state.current_margin_interest_due
                        + state.current_interest_due;

                    let lease_lpn = total(lease_amount, market_price);

                    let mut info = WarningAndPartialLiquidationInfo {
                        customer: self.customer.clone(),
                        ltv: Percent::default(),
                        ltv_healthy: self.liability.healthy_percent(),
                        lease_asset: self.currency.clone(),
                    };

                    match Percent::from_permille((Amount::from(liability_lpn) * 1000 / Amount::from(lease_lpn)) as Units) {
                        liability_percent if liability_percent < self.liability.first_liq_warn_percent() => LiquidationStatus::None,
                        liability_percent if liability_percent < self.liability.second_liq_warn_percent() => {
                            info.ltv = self.liability.first_liq_warn_percent();

                            LiquidationStatus::FirstWarning(info)
                        }
                        liability_percent if liability_percent < self.liability.third_liq_warn_percent() => {
                            info.ltv = self.liability.second_liq_warn_percent();

                            LiquidationStatus::SecondWarning(info)
                        }
                        liability_percent if liability_percent < self.liability.max_percent() => {
                            info.ltv = self.liability.third_liq_warn_percent();

                            LiquidationStatus::ThirdWarning(info)
                        }
                        _ => {
                            let liquidation_amount = lease_amount.min(
                                Coin::new(
                                    (
                                        Amount::from(
                                            liability_lpn - self.liability.healthy_percent().of(lease_lpn)
                                        ) * Percent::HUNDRED.parts() as Amount
                                    ) / (Percent::HUNDRED - self.liability.healthy_percent()).parts() as Amount,
                                ),
                            );

                            // TODO update contract's "lease_amount"

                            info.ltv = self.liability.max_percent();

                            if liquidation_amount == lease_amount {
                                LiquidationStatus::FullLiquidation(info, lease_amount)
                            } else {
                                LiquidationStatus::PartialLiquidation(info, liquidation_amount)
                            }
                        }
                    }
                },
            )
        })
    }

    #[inline]
    fn initial_alarm_schedule<A>(
        &self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
        liquidation: &LiquidationStatus<Lpn>,
    ) -> ContractResult<Vec<BatchMessage<WasmMsg, LeaseReplyId>>>
        where
            A: Into<Addr>,
    {
        self.reschedule_from_price_alarm(lease, lease_amount, now, liquidation)
    }

    fn reschedule_price_alarm<A>(
        &self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
        liquidation: &LiquidationStatus<Lpn>,
    ) -> ContractResult<BatchMessage<WasmMsg, LeaseReplyId>>
        where
            A: Into<Addr>,
    {
        Ok(BatchMessage::NoReply(wasm_execute(
            self.market_price_oracle.as_str(),
            &AddPriceAlarm {
                target: self.price_alarm_by_percent(
                    lease,
                    lease_amount,
                    now,
                    match liquidation {
                        LiquidationStatus::None
                        | LiquidationStatus::PartialLiquidation(..) => self.liability.first_liq_warn_percent(),
                        LiquidationStatus::FirstWarning(_) => self.liability.second_liq_warn_percent(),
                        LiquidationStatus::SecondWarning(_) => self.liability.third_liq_warn_percent(),
                        LiquidationStatus::ThirdWarning(_) => self.liability.max_percent(),
                        LiquidationStatus::FullLiquidation(..) => unreachable!(),
                    },
                )?.into(),
            },
            Vec::new(),
        )?))
    }

    fn price_alarm_by_percent<A>(
        &self,
        lease: A,
        lease_amount: Coin<Lpn>,
        now: &Timestamp,
        percent: Percent,
    ) -> ContractResult<Price<Lpn, Lpn>>
    where
        A: Into<Addr>,
    {
        let state = self.loan.state(
            *now + self.liability.recalculation_time(),
            lease.into(),
        )?
            .ok_or(ContractError::LoanClosed())?;

        assert!(!lease_amount.is_zero(), "Loan already paid!");

        Ok(
            total_of(
                percent.of(lease_amount),
            ).is(
                state.principal_due
                    + state.previous_margin_interest_due
                    + state.previous_interest_due
                    + state.current_margin_interest_due
                    + state.current_interest_due,
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Addr, Timestamp};
    use serde::{Deserialize, Serialize};

    use finance::{
        coin::Coin,
        currency::{
            Currency,
            Nls,
            Usdc
        },
        duration::Duration,
        interest::InterestPeriod,
        liability::Liability,
        percent::Percent
    };
    use lpp::{
        error::ContractError as LppError,
        msg::{LoanResponse, OutstandingInterest, QueryLoanResponse},
        stub::{Lpp, LppRef}
    };
    use platform::{
        bank::BankAccountView,
        batch::Batch,
        error::Result as PlatformResult
    };

    use crate::loan::{Loan, LoanDTO};
    use crate::msg::StateResponse;

    use super::Lease;

    const MARGIN_INTEREST_RATE: Percent = Percent::from_permille(23);
    const LEASE_START: Timestamp = Timestamp::from_nanos(100);
    const LEASE_STATE_AT: Timestamp = Timestamp::from_nanos(200);
    type TestCurrency = Usdc;
    type LppResult<T> = Result<T, LppError>;

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct BankStub {
        balance: u128,
    }

    impl BankAccountView for BankStub {
        fn balance<C>(&self) -> PlatformResult<Coin<C>>
        where
            C: Currency,
        {
            Ok(Coin::<C>::new(self.balance))
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct LppLocalStub {
        loan: Option<LoanResponse<TestCurrency>>,
    }

    // TODO define a MockLpp trait to avoid implementing Lpp-s from scratch
    impl Lpp<TestCurrency> for LppLocalStub {
        fn id(&self) -> Addr {
            Addr::unchecked("0123456789ABDEF0123456789ABDEF0123456789ABDEF0123456789ABDEF")
        }

        fn open_loan_req(&mut self, _amount: Coin<TestCurrency>) -> LppResult<()> {
            unreachable!()
        }

        fn open_loan_resp(&self, _resp: cosmwasm_std::Reply) -> LppResult<LoanResponse<TestCurrency>> {
            unreachable!()
        }

        fn repay_loan_req(&mut self, _repayment: Coin<TestCurrency>) -> LppResult<()> {
            unreachable!()
        }

        fn loan(&self, _lease: impl Into<Addr>) -> LppResult<QueryLoanResponse<TestCurrency>> {
            Ok(self.loan.clone())
        }

        fn loan_outstanding_interest(
            &self,
            _lease: impl Into<Addr>,
            by: Timestamp,
        ) -> LppResult<lpp::msg::QueryLoanOutstandingInterestResponse<TestCurrency>> {
            Ok(self.loan.as_ref().map(|loan| {
                OutstandingInterest(
                    InterestPeriod::with_interest(loan.annual_interest_rate)
                        .spanning(Duration::between(loan.interest_paid, by))
                        .interest(loan.principal_due),
                )
            }))
        }

        fn quote(&self, _amount: Coin<TestCurrency>) -> LppResult<lpp::msg::QueryQuoteResponse> {
            unreachable!()
        }

        fn lpp_balance(&self) -> LppResult<lpp::msg::LppBalanceResponse<TestCurrency>> {
            unreachable!()
        }

        fn nlpn_price(&self) -> LppResult<lpp::msg::PriceResponse<TestCurrency>> {
            unreachable!()
        }

        fn config(&self) -> LppResult<lpp::msg::QueryConfigResponse> {
            unreachable!()
        }

        fn nlpn_balance(&self, _lender: impl Into<Addr>) -> LppResult<lpp::msg::BalanceResponse> {
            unreachable!()
        }

        fn rewards(&self, _lender: impl Into<Addr>) -> LppResult<lpp::msg::RewardsResponse> {
            unreachable!()
        }
    }

    impl From<LppLocalStub> for Batch {
        fn from(_: LppLocalStub) -> Self {
            unreachable!()
        }
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct LppLocalStubUnreachable {}

    impl From<LppLocalStubUnreachable> for Batch {
        fn from(_: LppLocalStubUnreachable) -> Self {
            unreachable!()
        }
    }

    impl Lpp<TestCurrency> for LppLocalStubUnreachable {
        fn id(&self) -> Addr {
            unreachable!()
        }

        fn open_loan_req(&mut self, _amount: Coin<TestCurrency>) -> LppResult<()> {
            unreachable!()
        }

        fn open_loan_resp(&self, _resp: cosmwasm_std::Reply) -> LppResult<LoanResponse<TestCurrency>> {
            unreachable!()
        }

        fn repay_loan_req(&mut self, _repayment: Coin<TestCurrency>) -> LppResult<()> {
            unreachable!()
        }

        fn loan(&self, _lease: impl Into<Addr>) -> LppResult<QueryLoanResponse<TestCurrency>> {
            unreachable!()
        }

        fn loan_outstanding_interest(
            &self,
            _lease: impl Into<Addr>,
            _by: Timestamp,
        ) -> LppResult<lpp::msg::QueryLoanOutstandingInterestResponse<TestCurrency>> {
            unreachable!()
        }

        fn quote(&self, _amount: Coin<TestCurrency>) -> LppResult<lpp::msg::QueryQuoteResponse> {
            unreachable!()
        }

        fn lpp_balance(&self) -> LppResult<lpp::msg::LppBalanceResponse<TestCurrency>> {
            unreachable!()
        }

        fn nlpn_price(&self) -> LppResult<lpp::msg::PriceResponse<TestCurrency>> {
            unreachable!()
        }

        fn config(&self) -> LppResult<lpp::msg::QueryConfigResponse> {
            unreachable!()
        }

        fn nlpn_balance(&self, _lender: impl Into<Addr>) -> LppResult<lpp::msg::BalanceResponse> {
            unreachable!()
        }

        fn rewards(&self, _lender: impl Into<Addr>) -> LppResult<lpp::msg::RewardsResponse> {
            unreachable!()
        }
    }

    fn create_lease<L>(lpp: L) -> Lease<TestCurrency, L>
    where
        L: Lpp<TestCurrency>,
    {
        let lpp_ref = LppRef::unchecked::<_, Nls>("lpp_adr");
        let loan_dto = LoanDTO::new(
            LEASE_START,
            lpp_ref,
            MARGIN_INTEREST_RATE,
            Duration::from_secs(100),
            Duration::from_secs(0),
        )
        .unwrap();
        Lease {
            customer: Addr::unchecked("customer"),
            currency: TestCurrency::SYMBOL.to_string(),
            liability: Liability::new(
                Percent::from_percent(65),
                Percent::from_percent(5),
                Percent::from_percent(10),
                Percent::from_percent(2),
                Percent::from_percent(3),
                Percent::from_percent(2),
                10 * 24,
            ),
            loan: Loan::from_dto(loan_dto, lpp),
            market_price_oracle: Addr::unchecked("oracle"),
        }
    }

    fn lease_setup(
        loan_response: Option<LoanResponse<TestCurrency>>,
    ) -> Lease<TestCurrency, LppLocalStub> {
        let lpp_stub = LppLocalStub {
            loan: loan_response,
        };

        create_lease(lpp_stub)
    }

    fn create_bank_account(lease_amount: u128) -> BankStub {
        BankStub {
            balance: lease_amount,
        }
    }

    fn request_state(
        lease: Lease<TestCurrency, LppLocalStub>,
        bank_account: &BankStub,
    ) -> StateResponse<TestCurrency, TestCurrency> {
        lease
            .state(LEASE_STATE_AT, bank_account, Addr::unchecked("unused"))
            .unwrap()
    }

    #[test]
    // Open state -> Lease's balance in the loan's currency > 0, loan exists in the lpp
    fn state_opened() {
        let lease_amount = 1000;
        let interest_rate = Percent::from_permille(50);
        // LPP loan
        let loan = LoanResponse {
            principal_due: coin(300),
            interest_due: coin(0),
            annual_interest_rate: interest_rate,
            interest_paid: Timestamp::from_nanos(0),
        };

        let bank_account = create_bank_account(lease_amount);
        let lease = lease_setup(Some(loan.clone()));

        let res = request_state(lease, &bank_account);
        let exp = StateResponse::Opened {
            amount: coin(lease_amount),
            interest_rate,
            interest_rate_margin: MARGIN_INTEREST_RATE,
            principal_due: loan.principal_due,
            previous_margin_due: coin(0),
            previous_interest_due: coin(0),
            current_margin_due: coin(0),
            current_interest_due: coin(0),
        };

        assert_eq!(exp, res);
    }

    #[test]
    // Paid state -> Lease's balance in the loan's currency > 0, loan doesn't exist in the lpp anymore
    fn state_paid() {
        let lease_amount = 1000;
        let bank_account = create_bank_account(lease_amount);
        let lease = lease_setup(None);

        let res = request_state(lease, &bank_account);
        let exp = StateResponse::Paid(coin(lease_amount));
        assert_eq!(exp, res);
    }

    #[test]
    // Closed state -> Lease's balance in the loan's currency = 0, loan doesn't exist in the lpp anymore
    fn state_closed() {
        let lease_amount = 0;
        let bank_account = create_bank_account(lease_amount);
        let lease = lease_setup(None);

        let res = request_state(lease, &bank_account);
        let exp = StateResponse::Closed();
        assert_eq!(exp, res);
    }

    #[test]
    // Verify that if the Lease's balance is 0, lpp won't be queried for the loan
    fn state_closed_lpp_must_not_be_called() {
        let lpp_stub = LppLocalStubUnreachable {};
        let lease = create_lease(lpp_stub);

        let bank_account = create_bank_account(0);

        let res = lease
            .state(
                Timestamp::from_nanos(0),
                &bank_account,
                Addr::unchecked("unused"),
            )
            .unwrap();

        let exp = StateResponse::Closed();
        assert_eq!(exp, res);
    }

    fn coin(a: u128) -> Coin<TestCurrency> {
        Coin::<TestCurrency>::new(a)
    }
}
