use serde::{Deserialize, Serialize};

use finance::{
    coin::{Amount, CoinDTO},
    currency::SymbolOwned,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownpaymentDTO {
    pub(super) downpayment: CoinDTO,
}

impl DownpaymentDTO {
    pub(crate) fn new(downpayment: CoinDTO) -> Self {
        Self { downpayment }
    }

    pub(crate) const fn amount(&self) -> Amount {
        self.downpayment.amount()
    }

    pub(crate) const fn symbol(&self) -> &SymbolOwned {
        self.downpayment.symbol()
    }
}
