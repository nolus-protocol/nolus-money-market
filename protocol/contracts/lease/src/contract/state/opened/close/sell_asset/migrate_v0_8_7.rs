use serde::{Deserialize, Serialize};

use dex::{AcceptAnyNonZeroSwap, DexResult, MaxSlippage, SlippageCalculator};
use finance::{
    coin::{Coin, CoinDTO},
    percent::{Percent, bound::BoundToHundredPercent},
};
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    api::LeaseAssetCurrencies,
    finance::{LpnCurrencies, LpnCurrency},
};

/// Provide a slippage calculator for leases that have started liquidation at v0.8.7
///
/// The key lies in the serde untagged enum support allowing both variants to be deserialized.
// TODO clean-up on the next release
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", untagged)]
pub enum CompoundCalculator {
    MaxSlippage(MaxSlippage<LeaseAssetCurrencies, LpnCurrency, LpnCurrencies>),
    AnySlippage(AcceptAnyNonZeroSwap<LeaseAssetCurrencies, LpnCurrency>),
}

impl CompoundCalculator {
    pub fn threshold(&self) -> BoundToHundredPercent {
        match self {
            Self::AnySlippage(_calc) => BoundToHundredPercent::try_from_percent(Percent::HUNDRED)
                .expect("100% to be a valid slippage"),
            Self::MaxSlippage(calc) => calc.threshold(),
        }
    }
}

impl Default for CompoundCalculator {
    fn default() -> Self {
        Self::AnySlippage(AcceptAnyNonZeroSwap::default())
    }
}

impl SlippageCalculator<LeaseAssetCurrencies> for CompoundCalculator {
    type OutC = LpnCurrency;

    fn min_output(
        &self,
        input: &CoinDTO<LeaseAssetCurrencies>,
        querier: QuerierWrapper<'_>,
    ) -> DexResult<Coin<Self::OutC>> {
        match self {
            Self::AnySlippage(calc) => calc.min_output(input, querier),
            Self::MaxSlippage(calc) => calc.min_output(input, querier),
        }
    }
}

impl From<MaxSlippage<LeaseAssetCurrencies, LpnCurrency, LpnCurrencies>> for CompoundCalculator {
    fn from(value: MaxSlippage<LeaseAssetCurrencies, LpnCurrency, LpnCurrencies>) -> Self {
        Self::MaxSlippage(value)
    }
}

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};

    use super::CompoundCalculator;

    #[derive(Serialize)]
    struct PrevState {
        val1: u8,
        val2: bool,
    }

    #[derive(Deserialize)]
    struct CurrenctState {
        val1: u8,
        val2: bool,
        #[serde(default = "CompoundCalculator::default")]
        slippage_calc: CompoundCalculator,
    }

    #[test]
    fn load_previous() {
        let prev = PrevState {
            val1: 12,
            val2: true,
        };
        let prev_bin = cosmwasm_std::to_json_vec(&prev).expect("serialization succeed");
        let current: CurrenctState =
            cosmwasm_std::from_json(prev_bin).expect("deserialization succeed");
        assert_eq!(prev.val1, current.val1);
        assert_eq!(prev.val2, current.val2);
        assert!(matches!(
            current.slippage_calc,
            CompoundCalculator::AnySlippage(_),
        ));
    }
}
