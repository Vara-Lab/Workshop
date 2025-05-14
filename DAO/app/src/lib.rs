
#![no_std]

use sails_rs::prelude::*;
pub mod services;
use services::service::Service;

pub struct Program;

#[program]
impl Program {
    /// Constructor for the Voting Program.
    /// Must be called once at deployment, passing the admin and available options.
    pub fn new(admin: ActorId, options: Vec<String>) -> Self {
        Service::seed(admin, options);
        Self
    }

    #[route("Service")]
    pub fn service(&self) -> Service {
        Service::new()
    }
}
