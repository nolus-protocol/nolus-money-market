use serde::{Deserialize, Serialize};

use finance::percent::Percent;
use sdk::{
    cosmwasm_std::Timestamp,
    schemars::{self, JsonSchema},
};

use super::{DownpaymentCoin, LeaseCoin, LpnCoin};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StateQuery {}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
#[serde(rename_all = "snake_case")]
pub enum StateResponse {
    Opening {
        downpayment: DownpaymentCoin,
        loan: DownpaymentCoin, // TODO replace with LpnCoin,
        loan_interest_rate: Percent,
        in_progress: opening::OngoingTrx,
    },
    Opened {
        amount: LeaseCoin,
        loan_interest_rate: Percent,
        margin_interest_rate: Percent,
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

pub mod opening {
    use sdk::schemars::{self, JsonSchema};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
    #[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
    #[serde(rename_all = "snake_case")]
    pub enum OngoingTrx {
        OpenIcaAccount,
        TransferOut { ica_account: String },
        BuyAsset { ica_account: String },
    }
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
