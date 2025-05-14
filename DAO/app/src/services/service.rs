
#![no_std]
#![allow(static_mut_refs)]

use sails_rs::{
    collections::HashMap,
    gstd::{msg, exec},
    prelude::*,
};

// Global static state for the voting contract
static mut VOTING_STATE: Option<VotingState> = None;

// State structure for the contract
#[derive(Clone, Default)]
pub struct VotingState {
    pub admin: ActorId,
    pub options: Vec<String>,
    pub votes: HashMap<String, u64>,
    pub has_voted: Vec<ActorId>,
    pub voting_open: bool,
}

// Methods related to VotingState
impl VotingState {
    // Initialize contract state; can only be called once
    pub fn init(admin: ActorId, options: Vec<String>) {
        unsafe {
            VOTING_STATE = Some(Self {
                admin,
                options: options.clone(),
                votes: options.into_iter().map(|opt| (opt, 0u64)).collect(),
                has_voted: Vec::new(),
                voting_open: true,
            });
        }
    }

    // Get a mutable reference to the state
    pub fn state_mut() -> &'static mut VotingState {
        let state = unsafe { VOTING_STATE.as_mut() };
        debug_assert!(state.is_some(), "State not initialized");
        unsafe { state.unwrap_unchecked() }
    }

    // Get an immutable reference to the state
    pub fn state_ref() -> &'static VotingState {
        let state = unsafe { VOTING_STATE.as_ref() };
        debug_assert!(state.is_some(), "State not initialized");
        unsafe { state.unwrap_unchecked() }
    }
}

// Structure for external state queries
#[derive(Encode, Decode, TypeInfo, Clone)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct IoVotingState {
    pub admin: ActorId,
    pub options: Vec<String>,
    pub votes: Vec<(String, u64)>,
    pub voting_open: bool,
}

// Convert internal state to queryable state struct
impl From<VotingState> for IoVotingState {
    fn from(state: VotingState) -> Self {
        Self {
            admin: state.admin,
            options: state.options.clone(),
            votes: state.votes.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            voting_open: state.voting_open,
        }
    }
}

// Events for off-chain tracking
#[derive(Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum Events {
    VoteCast { voter: ActorId, option: String },
    VotingClosed,
    OptionAdded(String),
    Error(String),
}

// Voting service structure
#[derive(Default)]
pub struct Service;

// Service implementation
#[sails_rs::service(events = Events)]
impl Service {
    // Constructor. Does not initialize contract; use `seed` for that.
    pub fn new() -> Self {
        Self
    }

    /// Seed function to initialize voting state (call EXACTLY once)
    pub fn seed(admin: ActorId, options: Vec<String>) {
        // Validate options are not empty and unique
        if options.is_empty() {
            panic!("No voting options provided");
        }
        let mut seen = Vec::new();
        for option in options.iter() {
            if seen.contains(option) {
                panic!("Duplicate voting options are not allowed");
            }
            seen.push(option.clone());
        }
        VotingState::init(admin, options);
    }

    /// Cast a vote on an option. Fails if voting is closed or sender already voted.
    pub fn vote(&mut self, option: String) -> Events {
        let sender = msg::source();
        let voting = VotingState::state_mut();

        // Check voting is open
        if !voting.voting_open {
            return Events::Error("Voting is closed".to_string());
        }
        // Check the user has not voted yet
        if voting.has_voted.contains(&sender) {
            return Events::Error("Already voted".to_string());
        }
        // Check the option exists
        if !voting.options.contains(&option) {
            return Events::Error("Invalid option".to_string());
        }
        let count = voting.votes.get_mut(&option).expect("No such option");
        *count = count.saturating_add(1);

        voting.has_voted.push(sender);

        self.emit_event(Events::VoteCast {
            voter: sender,
            option: option.clone(),
        })
        .expect("Event error");
        Events::VoteCast { voter: sender, option }
    }

    /// Only admin can add an option while voting is still open.
    pub fn add_option(&mut self, option: String) -> Events {
        let sender = msg::source();
        let voting = VotingState::state_mut();

        if sender != voting.admin {
            return Events::Error("Only admin can add options".to_string());
        }
        if !voting.voting_open {
            return Events::Error("Voting must be open".to_string());
        }
        if option.is_empty() {
            return Events::Error("Option cannot be empty".to_string());
        }
        if voting.options.contains(&option) {
            return Events::Error("Option already exists".to_string());
        }

        voting.options.push(option.clone());
        voting.votes.insert(option.clone(), 0u64);

        self.emit_event(Events::OptionAdded(option.clone()))
            .expect("Event error");
        Events::OptionAdded(option)
    }

    /// Close the voting (only admin). Once closed, voting cannot be reopened.
    pub fn close_voting(&mut self) -> Events {
        let sender = msg::source();
        let voting = VotingState::state_mut();

        if sender != voting.admin {
            return Events::Error("Only admin can close voting".to_string());
        }
        if !voting.voting_open {
            return Events::Error("Voting already closed".to_string());
        }
        voting.voting_open = false;

        self.emit_event(Events::VotingClosed)
            .expect("Event error");
        Events::VotingClosed
    }

    /// Query: Returns list of options and their current vote counts
    pub fn query_results(&self) -> Vec<(String, u64)> {
        VotingState::state_ref()
            .votes
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Query: Returns the voting options (without vote counts)
    pub fn query_options(&self) -> Vec<String> {
        VotingState::state_ref().options.clone()
    }

    /// Query: Returns true if voting is open, false otherwise
    pub fn query_voting_open(&self) -> bool {
        VotingState::state_ref().voting_open
    }

    /// Query: Returns the entire state for frontends
    pub fn query_state(&self) -> IoVotingState {
        VotingState::state_ref().clone().into()
    }
}
