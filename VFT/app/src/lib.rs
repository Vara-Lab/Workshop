
#![no_std]
#![allow(clippy::new_without_default)]

use sails_rs::prelude::*;
pub mod services;
use services::service::ExtendedService;

pub struct Program(());

#[program]
impl Program {
    pub fn new(name: String, symbol: String, decimals: u8) -> Self {
        ExtendedService::seed(name, symbol, decimals);
        Self(())
    }

    #[route("Vft")]
    pub fn vft(&self) -> ExtendedService {
        ExtendedService::new()
    }
}
