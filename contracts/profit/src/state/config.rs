use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Config {
    cadence_hours: u16,
    treasury: Addr,
}

impl Config {
    pub fn new(cadence_hours: u16, treasury: Addr) -> Self {
        Config {
            cadence_hours,
            treasury,
        }
    }

    pub fn update(self, cadence_hours: u16) -> Self {
        Self {
            cadence_hours,
            ..self
        }
    }

    pub fn cadence_hours(&self) -> u16 {
        self.cadence_hours
    }

    pub fn treasury(&self) -> &Addr {
        &self.treasury
    }
}
