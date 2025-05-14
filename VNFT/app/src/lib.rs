
#![no_std]
#![allow(clippy::new_without_default)]

use sails_rs::prelude::*;
pub mod services;
use services::service::ExtendedService;

pub struct Program(());

#[program]
impl Program {
    pub fn new(name: String, symbol: String) -> Self {
        ExtendedService::seed(name, symbol);
        Self(())
    }

    #[route("Service")]
    pub fn service(&self) -> ExtendedService {
        ExtendedService::new()
    }
}
