#![no_std]

// Add your program code

use sails_rs::prelude::*;

pub mod services;

// Import service to be used for the program
use services::service::Service;

pub struct Program;

#[program]
impl Program {
    // Application constructor (it is an associated function)
    // It can be called once per application lifetime.
    pub fn new() -> Self {
        // Init the state
        Service::seed();

        Self
    }

    
    #[route("Service")]
    pub fn service_svc(&self) -> Service {
        Service::new()
    }
}