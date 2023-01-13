use serde::{Deserialize, Serialize};

use finance::percent::Percent;
use sdk::{
    cosmwasm_std::Timestamp,
    schemars::{self, JsonSchema},
};

use super::{LeaseCoin, LpnCoin};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StateQuery {}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
#[serde(rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)] // TODO consider removing it once have added all intermediate states
pub enum StateResponse {
    Opened {
        amount: LeaseCoin,
        interest_rate: Percent,
        interest_rate_margin: Percent,
        principal_due: LpnCoin,
        previous_margin_due: LpnCoin,
        previous_interest_due: LpnCoin,
        current_margin_due: LpnCoin,
        current_interest_due: LpnCoin,
        validity: Timestamp,
        in_progress: Option<opened::OngoingTrx>,
    },
    Paid {
        amount: LeaseCoin,
        in_progress: Option<paid::ClosingTrx>,
    },
    Closed(),
}

pub mod opened {
    use sdk::schemars::{self, JsonSchema};
    use serde::{Deserialize, Serialize};

    use crate::api::{DownpaymentCoin, LpnCoin};

    #[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
    #[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
    #[serde(rename_all = "snake_case")]
    pub enum OngoingTrx {
        Repayment {
            payment: DownpaymentCoin,
            in_progress: RepayTrx,
        },
        Liquidation {
            amount_out: LpnCoin,
            in_progress: LiquidateTrx,
        },
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
    #[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
    #[serde(rename_all = "snake_case")]
    pub enum RepayTrx {
        TransferOut,
        Swap,
        TransferIn,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
    #[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
    #[serde(rename_all = "snake_case")]
    pub enum LiquidateTrx {
        Swap,
        TransferIn,
    }
}

pub mod paid {
    use sdk::schemars::{self, JsonSchema};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
    #[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
    #[serde(rename_all = "snake_case")]
    pub enum ClosingTrx {
        TransferIn,
    }
}
