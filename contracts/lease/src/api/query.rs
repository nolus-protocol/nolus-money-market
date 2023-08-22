use serde::{Deserialize, Serialize};

use finance::percent::Percent;
use sdk::{
    cosmwasm_std::Timestamp,
    schemars::{self, JsonSchema},
};

use super::{DownpaymentCoin, LeaseCoin, LpnCoin};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    State {},
    IsClosed {},
}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum StateResponse {
    Opening {
        downpayment: DownpaymentCoin,
        loan: LpnCoin,
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
    Liquidated(),
}

pub mod opening {
    use serde::{Deserialize, Serialize};

    use sdk::schemars::{self, JsonSchema};

    #[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
    #[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub enum OngoingTrx {
        OpenIcaAccount,
        TransferOut { ica_account: String },
        BuyAsset { ica_account: String },
    }
}

pub mod opened {
    use serde::{Deserialize, Serialize};

    use sdk::schemars::{self, JsonSchema};

    use crate::api::{LeaseCoin, PaymentCoin};

    #[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
    #[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub enum OngoingTrx {
        Repayment {
            payment: PaymentCoin,
            in_progress: RepayTrx,
        },
        Liquidation {
            liquidation: LeaseCoin,
            in_progress: LiquidateTrx,
        },
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
    #[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub enum RepayTrx {
        TransferOut,
        Swap,
        TransferInInit,
        TransferInFinish,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
    #[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub enum LiquidateTrx {
        Swap,
        TransferInInit,
        TransferInFinish,
    }
}

pub mod paid {
    use serde::{Deserialize, Serialize};

    use sdk::schemars::{self, JsonSchema};

    #[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
    #[cfg_attr(any(test, feature = "testing"), derive(Clone, Debug))]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub enum ClosingTrx {
        TransferInInit,
        TransferInFinish,
    }
}
