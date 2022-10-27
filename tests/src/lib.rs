#![cfg(not(target_arch = "wasm32"))]

#[cfg(test)]
mod dispatcher_tests;

#[cfg(test)]
mod profit_tests;

#[cfg(test)]
mod lpp_tests;

#[cfg(test)]
#[allow(dead_code)]
mod common;

#[cfg(test)]
mod leaser_tests;

#[cfg(test)]
mod lease_tests;

#[cfg(test)]
mod oracle_tests;

#[cfg(test)]
mod timealarms_tests;
