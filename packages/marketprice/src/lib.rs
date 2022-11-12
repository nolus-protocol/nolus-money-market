use finance::price::dto::PriceDTO;
use swap::SwapGroup;

pub mod alarms;
pub mod error;
pub mod feed;
pub mod feeders;
pub mod market_price;

#[cfg(test)]
mod tests;

pub type SpotPrice = PriceDTO<SwapGroup, SwapGroup>;
