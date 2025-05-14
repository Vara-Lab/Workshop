// Add your service code

use sails_rs::prelude::*;

static mut STATE: Option<u64> = None;

pub struct Service;

#[service]
impl Service {
    pub fn new() -> Self {
        Self {}
    }

    pub fn seed() {
        unsafe {
            STATE = Some(0);
        }
    }

    pub fn change_number(&mut self, number: u64) -> String {
        unsafe {
            let _ = STATE
                .get_or_insert(0)
                .checked_add(number);
        };

        "Number changed".to_string()
    }

    pub fn get_number(&self) -> u64 {
        unsafe { STATE.unwrap_or(0) }
    }
}