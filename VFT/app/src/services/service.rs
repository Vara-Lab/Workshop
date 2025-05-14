

#![allow(static_mut_refs)]

use sails_rs::{
    gstd::msg,
    collections::HashSet,
    prelude::*,
};
use vft_service::utils;
use vft_service::{Service as VftService, Storage};
use vft_service::{
    funcs,
    utils::{Error, Result, *},
};

#[derive(Default)]
pub struct ExtendedStorage {
    minters: HashSet<ActorId>,
    burners: HashSet<ActorId>,
    admins: HashSet<ActorId>,
}

static mut EXTENDED_STORAGE: Option<ExtendedStorage> = None;

#[derive(Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum Event {
    Minted { to: ActorId, value: U256 },
    Burned { from: ActorId, value: U256 },
}

#[derive(Clone)]
pub struct ExtendedService {
    vft: VftService,
}

impl ExtendedService {
    /// Initialize storage with caller as admin, minter, and burner, and seed chain state for vft.
    pub fn seed(name: String, symbol: String, decimals: u8) -> Self {
        let admin = msg::source();
        unsafe {
            EXTENDED_STORAGE = Some(ExtendedStorage {
                admins: [admin].into(),
                minters: [admin].into(),
                burners: [admin].into(),
            });
        };
        ExtendedService {
            vft: <VftService>::seed(name, symbol, decimals),
        }
    }

    pub fn get_mut(&mut self) -> &'static mut ExtendedStorage {
        unsafe {
            EXTENDED_STORAGE
                .as_mut()
                .expect("Extended vft is not initialized")
        }
    }
    pub fn get(&self) -> &'static ExtendedStorage {
        unsafe {
            EXTENDED_STORAGE
                .as_ref()
                .expect("Extended vft is not initialized")
        }
    }
}

#[service(extends = VftService, events = Event)]
impl ExtendedService {
    /// Service constructor
    pub fn new() -> Self {
        Self {
            vft: VftService::new(),
        }
    }

    /// Mint new tokens; must be allowed by minter.
    pub fn mint(&mut self, to: ActorId, value: U256) -> bool {
        // Only minters are allowed
        if !self.get().minters.contains(&msg::source()) {
            panic!("Not allowed to mint")
        };

        let mutated = utils::panicking(|| {
             mint(Storage::balances(), Storage::total_supply(), to, value)
        });
        if mutated {
            self.emit_event(Event::Minted { to, value })
                .expect("Notification Error");
        }
        mutated
    }

    /// Burn tokens from account; must be allowed by burner.
    pub fn burn(&mut self, from: ActorId, value: U256) -> bool {
        if !self.get().burners.contains(&msg::source()) {
            panic!("Not allowed to burn")
        };

        let mutated = utils::panicking(|| {
            burn(Storage::balances(), Storage::total_supply(), from, value)
        });
        if mutated {
            self.emit_event(Event::Burned { from, value })
                .expect("Notification Error");
        }
        mutated
    }

    /// Grant admin role to another ActorId; requires admin rights.
    pub fn grant_admin_role(&mut self, to: ActorId) {
        self.ensure_is_admin();
        self.get_mut().admins.insert(to);
    }
    /// Grant minter role; requires admin rights.
    pub fn grant_minter_role(&mut self, to: ActorId) {
        self.ensure_is_admin();
        self.get_mut().minters.insert(to);
    }
    /// Grant burner role; requires admin rights.
    pub fn grant_burner_role(&mut self, to: ActorId) {
        self.ensure_is_admin();
        self.get_mut().burners.insert(to);
    }

    /// Revoke admin role; requires admin rights.
    pub fn revoke_admin_role(&mut self, from: ActorId) {
        self.ensure_is_admin();
        self.get_mut().admins.remove(&from);
    }
    /// Revoke minter role; requires admin rights.
    pub fn revoke_minter_role(&mut self, from: ActorId) {
        self.ensure_is_admin();
        self.get_mut().minters.remove(&from);
    }
    /// Revoke burner role; requires admin rights.
    pub fn revoke_burner_role(&mut self, from: ActorId) {
        self.ensure_is_admin();
        self.get_mut().burners.remove(&from);
    }

    /// List all minters
    pub fn minters(&self) -> Vec<ActorId> {
        self.get().minters.clone().into_iter().collect()
    }
    /// List all burners
    pub fn burners(&self) -> Vec<ActorId> {
        self.get().burners.clone().into_iter().collect()
    }
    /// List all admins
    pub fn admins(&self) -> Vec<ActorId> {
        self.get().admins.clone().into_iter().collect()
    }
}

impl ExtendedService {
    fn ensure_is_admin(&self) {
        if !self.get().admins.contains(&msg::source()) {
            panic!("Not admin")
        };
    }
}
impl AsRef<VftService> for ExtendedService {
    fn as_ref(&self) -> &VftService {
        &self.vft
    }
}

/// Mint VFT tokens with overflow check.
pub fn mint(
    balances: &mut BalancesMap,
    total_supply: &mut U256,
    to: ActorId,
    value: U256,
) -> Result<bool> {
    if value.is_zero() {
        return Ok(false);
    }

    let new_total_supply = total_supply
        .checked_add(value)
        .ok_or(Error::NumericOverflow)?;

    let new_to = funcs::balance_of(balances, to)
        .checked_add(value)
        .ok_or(Error::NumericOverflow)?;

    balances.insert(to, new_to);
    *total_supply = new_total_supply;

    Ok(true)
}

/// Burn VFT tokens with underflow check.
pub fn burn(
    balances: &mut BalancesMap,
    total_supply: &mut U256,
    from: ActorId,
    value: U256,
) -> Result<bool> {
    if value.is_zero() {
        return Ok(false);
    }
    let new_total_supply = total_supply.checked_sub(value).ok_or(Error::Underflow)?;

    let new_from = funcs::balance_of(balances, from)
        .checked_sub(value)
        .ok_or(Error::InsufficientBalance)?;

    if !new_from.is_zero() {
        balances.insert(from, new_from);
    } else {
        balances.remove(&from);
    }

    *total_supply = new_total_supply;
    Ok(true)
}
