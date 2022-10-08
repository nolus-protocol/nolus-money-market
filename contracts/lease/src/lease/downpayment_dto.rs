use serde::{Deserialize, Serialize};

use finance::coin::CoinDTO;

//TODO flatten this out
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownpaymentDTO {
    pub(super) downpayment: CoinDTO,
}

impl DownpaymentDTO {
    pub(crate) fn new(downpayment: CoinDTO) -> Self {
        Self { downpayment }
    }
}

impl From<DownpaymentDTO> for CoinDTO {
    fn from(dto: DownpaymentDTO) -> Self {
        dto.downpayment
    }
}
