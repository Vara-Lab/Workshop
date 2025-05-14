
#![allow(static_mut_refs)]

use sails_rs::{
    collections::{HashMap},
    gstd::{msg},
    prelude::*,
};
use sails_rs::collections::HashSet;
use vnft_service::utils; // required for macros/types in utils
use vnft_service::utils::TokenId; // required: used as standalone type
use vnft_service::{Service as VnftService, Storage};
use vnft_service::utils::{Error, Result};
use sails_rs::gstd::service;

#[derive(Default)]
pub struct ExtendedStorage {
    token_id: TokenId,
    minters: HashSet<ActorId>,
    burners: HashSet<ActorId>,
    admins: HashSet<ActorId>,
    token_metadata_by_id: HashMap<TokenId, TokenMetadata>,
}

#[derive(Default, Debug, Encode, Decode, TypeInfo, Clone)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct TokenMetadata {
    pub name: String,
    pub description: String,
    pub media: String, 
    pub reference: String, 
}

static mut EXTENDED_STORAGE: Option<ExtendedStorage> = None;

#[derive(Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum Event {
    Minted {
        to: ActorId,
        token_metadata: TokenMetadata,
    },
    Burned {
        from: ActorId,
        token_id: TokenId,
    },
}

#[derive(Clone)]
pub struct ExtendedService {
    vnft: VnftService,
}

impl ExtendedService {
    pub fn seed(name: String, symbol: String) -> Self {
        let admin = msg::source();
        unsafe {
            EXTENDED_STORAGE = Some(ExtendedStorage {
                admins: [admin].into(),
                minters: [admin].into(),
                burners: [admin].into(),
                ..Default::default()
            });
        };
        ExtendedService {
            vnft: <VnftService>::init(name, symbol),
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

#[service(extends = VnftService, events = Event)]
impl ExtendedService {
    pub fn new() -> Self {
        Self {
            vnft: VnftService::new(),
        }
    }

    // Mint a new token. Only minters can mint.
    pub fn mint(&mut self, to: ActorId, token_metadata: TokenMetadata) {
        if !self.get().minters.contains(&msg::source()) {
            panic!("Not allowed to mint")
        };

        utils::panicking(|| {
            mint(
                Storage::owner_by_id(),
                Storage::tokens_for_owner(),
                &mut self.get_mut().token_metadata_by_id,
                &mut self.get_mut().token_id,
                to,
                token_metadata.clone(),
            )
        });
        self.emit_event(Event::Minted { to, token_metadata })
            .expect("Notification Error");
    }

    // Burn a token. Only burners can burn.
    pub fn burn(&mut self, from: ActorId, token_id: TokenId) {
        if !self.get().burners.contains(&msg::source()) {
            panic!("Not allowed to burn")
        };
        utils::panicking(|| {
            burn(
                Storage::owner_by_id(),
                Storage::tokens_for_owner(),
                Storage::token_approvals(),
                &mut self.get_mut().token_metadata_by_id,
                token_id,
            )
        });
        self.emit_event(Event::Burned { from, token_id })
            .expect("Notification Error");
    }

    // Grant admin role. Only admin can grant.
    pub fn grant_admin_role(&mut self, to: ActorId) {
        self.ensure_is_admin();
        self.get_mut().admins.insert(to);
    }

    // Grant minter role. Only admin can grant.
    pub fn grant_minter_role(&mut self, to: ActorId) {
        self.ensure_is_admin();
        self.get_mut().minters.insert(to);
    }

    // Grant burner role. Only admin can grant.
    pub fn grant_burner_role(&mut self, to: ActorId) {
        self.ensure_is_admin();
        self.get_mut().burners.insert(to);
    }

    // Revoke admin role. Only admin can revoke.
    pub fn revoke_admin_role(&mut self, from: ActorId) {
        self.ensure_is_admin();
        self.get_mut().admins.remove(&from);
    }
    // Revoke minter role. Only admin can revoke.
    pub fn revoke_minter_role(&mut self, from: ActorId) {
        self.ensure_is_admin();
        self.get_mut().minters.remove(&from);
    }

    // Revoke burner role. Only admin can revoke.
    pub fn revoke_burner_role(&mut self, from: ActorId) {
        self.ensure_is_admin();
        self.get_mut().burners.remove(&from);
    }

    // Query minters.
    pub fn minters(&self) -> Vec<ActorId> {
        self.get().minters.clone().into_iter().collect()
    }

    // Query burners.
    pub fn burners(&self) -> Vec<ActorId> {
        self.get().burners.clone().into_iter().collect()
    }

    // Query admins.
    pub fn admins(&self) -> Vec<ActorId> {
        self.get().admins.clone().into_iter().collect()
    }

    // Query token_id incrementer.
    pub fn token_id(&self) -> TokenId {
        self.get().token_id
    }

    // Query metadata by id.
    pub fn token_metadata_by_id(&self, token_id: TokenId) -> Option<TokenMetadata> {
        self.get().token_metadata_by_id.get(&token_id).cloned()
    }

    // Query all tokens for an owner (returns vec, not map)
    pub fn tokens_for_owner(&self, owner: ActorId) -> Vec<(TokenId, TokenMetadata)> {
        Storage::tokens_for_owner()
            .get(&owner)
            .map(|token_set| {
                token_set
                    .iter()
                    .filter_map(|token_id| {
                        self.token_metadata_by_id(*token_id)
                            .map(|metadata| (*token_id, metadata))
                    })
                    .collect()
            })
            .unwrap_or_else(|| Vec::new())
    }
}

impl ExtendedService {
    fn ensure_is_admin(&self) {
        if !self.get().admins.contains(&msg::source()) {
            panic!("Not admin")
        };
    }
}

impl AsRef<VnftService> for ExtendedService {
    fn as_ref(&self) -> &VnftService {
        &self.vnft
    }
}

// Mint function - do not modify, reuse as-is.
pub fn mint(
    owner_by_id: &mut HashMap<TokenId, ActorId>,
    tokens_for_owner: &mut HashMap<ActorId, HashSet<TokenId>>,
    token_metadata_by_id: &mut HashMap<TokenId, TokenMetadata>,
    token_id: &mut TokenId,
    to: ActorId,
    token_metadata: TokenMetadata,
) -> Result<()> {
    if token_metadata_by_id.contains_key(token_id) {
        gstd::ext::panic("Token already exists".to_string())
    }
    owner_by_id.insert(*token_id, to);
    tokens_for_owner.entry(to).or_insert_with(HashSet::new).insert(*token_id);
    token_metadata_by_id.insert(*token_id, token_metadata);
    let next = token_id.checked_add(U256::one())
        .ok_or_else(|| gstd::ext::panic("TokenId overflow".to_string()));
    *token_id = next.unwrap();
    Ok(())
}

// Burn function - do not modify, reuse as-is.
pub fn burn(
    owner_by_id: &mut HashMap<TokenId, ActorId>,
    tokens_for_owner: &mut HashMap<ActorId, HashSet<TokenId>>,
    token_approvals: &mut HashMap<TokenId, ActorId>,
    token_metadata_by_id: &mut HashMap<TokenId, TokenMetadata>,
    token_id: TokenId,
) -> Result<()> {
    let owner = owner_by_id.remove(&token_id)
        .ok_or_else(|| gstd::ext::panic("TokenDoesNotExist".to_string()))?;
    if let Some(tokens) = tokens_for_owner.get_mut(&owner) {
        tokens.remove(&token_id);
        if tokens.is_empty() {
            tokens_for_owner.remove(&owner);
        }
    }
    token_approvals.remove(&token_id);
    token_metadata_by_id.remove(&token_id);
    Ok(())
}
