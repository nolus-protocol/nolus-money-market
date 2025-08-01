pub use self::{
    config::Config, deposit::Deposit, migrate_v0_8_12::migrate as migrate_v0_8_12,
    rewards::TotalRewards, total::Total,
};

mod config;
mod deposit;
mod migrate_v0_8_12;
mod rewards;
mod total;
