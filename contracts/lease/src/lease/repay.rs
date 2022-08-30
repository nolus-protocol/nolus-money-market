use finance::currency::Currency;
use platform::batch::Batch;

use crate::{
    lease::LeaseDTO,
    loan::Receipt
};

pub(crate) struct RepayResult<Lpn>
where
    Lpn: Currency,
{
    pub batch: Batch,
    pub lease_dto: LeaseDTO,
    pub receipt: Receipt<Lpn>,
}
