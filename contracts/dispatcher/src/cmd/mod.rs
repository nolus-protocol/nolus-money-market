use currency::{lpn::Lpns, native::Native};
use finance::coin::CoinDTO;
use platform::batch::Batch;

mod dispatch;

pub use dispatch::Dispatch;

pub struct Result {
    pub batch: Batch,
    pub receipt: Receipt,
}

#[derive(Eq, PartialEq)]
pub struct Receipt {
    pub in_stable: CoinDTO<Lpns>,
    pub in_nls: CoinDTO<Native>,
}
