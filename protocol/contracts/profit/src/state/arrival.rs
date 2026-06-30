use currency::{CurrencyDef, Group, MemberOf};
use finance::{
    coin::{Amount, Coin, CoinDTO, WithCoin},
    zero::Zero,
};
use platform::{bank, error::Error as PlatformError};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

/// Snapshot the local-account balance, per drained currency, at drain entry
///
/// One entry per unique currency among `expected`, carrying that currency's
/// balance on `account` at the moment of the call. Taken once at drain
/// construction, before any coin is drained back, and persisted with the task
/// so [`arrived_over_baseline`] measures every later poll against the pre-drain
/// balances. Returns the raw bank error so each caller maps it into its own
/// error type.
pub fn snapshot_baseline<G>(
    expected: &[CoinDTO<G>],
    account: &Addr,
    querier: QuerierWrapper<'_>,
) -> Result<Vec<CoinDTO<G>>, PlatformError>
where
    G: Group,
{
    unique_currencies(expected.iter().copied())
        .map(|coin| account_balance(&coin, account, querier))
        .collect()
}

/// Have all the drained coins arrived over their entry baseline?
///
/// Each currency clears once its measured balance has risen above its entry
/// baseline by at least the aggregate amount expected in that currency. Two
/// properties make this safe where an absolute balance check is not. The
/// baseline is subtracted, so a balance the account already held — including
/// one an attacker bank-sent to force an early finish — is not mistaken for an
/// arrival. And coins are grouped by currency and summed, so two legs in the
/// same currency must both land before the check passes.
pub fn arrived_over_baseline<G>(
    expected: &[CoinDTO<G>],
    baseline: &[CoinDTO<G>],
    account: &Addr,
    querier: QuerierWrapper<'_>,
) -> dex::DexResult<bool>
where
    G: Group,
{
    // A drain always carries at least one coin; an empty `expected` would let
    // the `try_fold` seed report a no-transfer drain as complete, so fail closed
    // in release rather than rely on a debug-only invariant.
    if expected.is_empty() {
        return Ok(false);
    }
    unique_currencies(expected.iter().copied()).try_fold(true, |all_received, currency| {
        let expected_amount = aggregate_amount(expected.iter().copied(), &currency);
        let baseline_amount = aggregate_amount(baseline.iter().copied(), &currency);
        account_balance(&currency, account, querier)
            .map_err(Into::into)
            .map(|arrived| {
                all_received && expected_amount <= arrived.amount().saturating_sub(baseline_amount)
            })
    })
}

/// Deduplicate a coin list down to one representative per currency
///
/// A drain handles a handful of coins, so the linear scan is trivial; the
/// representative carries the currency only — its amount is irrelevant to the
/// per-currency aggregation done by [`aggregate_amount`].
fn unique_currencies<G, I>(coins: I) -> impl Iterator<Item = CoinDTO<G>>
where
    G: Group,
    I: IntoIterator<Item = CoinDTO<G>>,
{
    let mut seen: Vec<CoinDTO<G>> = Vec::new();
    for coin in coins {
        if !seen.iter().any(|kept| kept.currency() == coin.currency()) {
            seen.push(coin);
        }
    }
    seen.into_iter()
}

fn aggregate_amount<G, I>(coins: I, currency: &CoinDTO<G>) -> Amount
where
    G: Group,
    I: IntoIterator<Item = CoinDTO<G>>,
{
    coins
        .into_iter()
        .filter(|coin| coin.currency() == currency.currency())
        .map(|coin| coin.amount())
        .fold(Amount::ZERO, Amount::saturating_add)
}

/// Returns the raw bank error so each caller maps it into its own type — the
/// entry snapshot into `ContractError`, the arrival check into `dex::Error`.
fn account_balance<G>(
    coin: &CoinDTO<G>,
    account: &Addr,
    querier: QuerierWrapper<'_>,
) -> Result<CoinDTO<G>, PlatformError>
where
    G: Group,
{
    struct Balance<'account, 'querier> {
        account: &'account Addr,
        querier: QuerierWrapper<'querier>,
    }

    impl<G> WithCoin<G> for Balance<'_, '_>
    where
        G: Group,
    {
        type Outcome = Result<CoinDTO<G>, PlatformError>;

        fn on<C>(self, _coin: Coin<C>) -> Self::Outcome
        where
            C: CurrencyDef,
            C::Group: MemberOf<G> + MemberOf<G::TopG>,
        {
            bank::balance::<C>(self.account, self.querier).map(CoinDTO::from)
        }
    }

    coin.with_coin(Balance { account, querier })
}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use currencies::{Native, Nls};
    use currency::CurrencyDef;
    use finance::coin::{Amount, Coin, CoinDTO};
    use sdk::cosmwasm_std::{Addr, Coin as CwCoin, Empty, QuerierWrapper, testing::MockQuerier};

    const ACCOUNT: &str = "account";
    const PROCEEDS: Amount = 1_000;

    /// A pre-existing balance in the drained currency is not mistaken for an
    /// arrival: the entry baseline is subtracted, so the same balance at the
    /// arrival check leaves nothing received.
    #[test]
    fn pre_existing_balance_is_not_an_arrival() {
        let held = balances(PROCEEDS);
        let baseline = snapshot(&proceeds(), &held);

        assert_eq!(Ok(false), arrived(&proceeds(), &baseline, &held));
    }

    /// Zero-baseline arrival is unchanged from an absolute check: it completes
    /// exactly when the balance reaches the expected amount.
    #[test]
    fn zero_baseline_arrives_at_the_expected_amount() {
        let baseline = snapshot(&proceeds(), &balances(0));

        assert_eq!(
            Ok(false),
            arrived(&proceeds(), &baseline, &balances(PROCEEDS - 1))
        );
        assert_eq!(
            Ok(true),
            arrived(&proceeds(), &baseline, &balances(PROCEEDS))
        );
    }

    /// Arrival is measured over the baseline: a pre-existing balance plus only
    /// part of the proceeds is still short.
    #[test]
    fn arrival_measured_over_baseline() {
        const PRE_EXISTING: Amount = 500;
        let baseline = snapshot(&proceeds(), &balances(PRE_EXISTING));

        assert_eq!(
            Ok(false),
            arrived(
                &proceeds(),
                &baseline,
                &balances(PRE_EXISTING + PROCEEDS - 1)
            )
        );
        assert_eq!(
            Ok(true),
            arrived(&proceeds(), &baseline, &balances(PRE_EXISTING + PROCEEDS))
        );
    }

    /// An empty drain fails closed in release: the gate never reports a
    /// no-transfer drain as complete.
    #[test]
    fn empty_expected_never_completes() {
        assert_eq!(Ok(false), arrived(&[], &[], &balances(0)));
    }

    fn proceeds() -> [CoinDTO<Native>; 1] {
        [Coin::<Nls>::new(PROCEEDS).into()]
    }

    fn arrived(
        expected: &[CoinDTO<Native>],
        baseline: &[CoinDTO<Native>],
        querier: &MockQuerier<Empty>,
    ) -> Result<bool, String> {
        super::arrived_over_baseline(
            expected,
            baseline,
            &Addr::unchecked(ACCOUNT),
            QuerierWrapper::new(querier),
        )
        .map_err(|err| err.to_string())
    }

    fn snapshot(
        expected: &[CoinDTO<Native>],
        querier: &MockQuerier<Empty>,
    ) -> Vec<CoinDTO<Native>> {
        super::snapshot_baseline(
            expected,
            &Addr::unchecked(ACCOUNT),
            QuerierWrapper::new(querier),
        )
        .expect("the baseline snapshot succeeds")
    }

    fn balances(balance: Amount) -> MockQuerier<Empty> {
        MockQuerier::<Empty>::new(&[(
            ACCOUNT,
            &[CwCoin::new(balance, Nls::dto().definition().bank_symbol)],
        )])
    }
}
