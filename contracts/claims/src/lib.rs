#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

mod policy_client {
    use soroban_sdk::{contractclient, Env};
    #[contractclient(name = "PolicyClient")]
    pub trait PolicyInterface {
        fn is_policy_active(env: Env, policy_id: u64) -> bool;
        fn get_policy_coverage(env: Env, policy_id: u64) -> i128;
    }
}
use policy_client::PolicyClient;

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum ClaimStatus { Pending, Approved, Rejected, Settled }

#[contracttype]
#[derive(Clone)]
pub struct ClaimRecord {
    pub policy_id: u64,
    pub amount: i128,
    pub status: ClaimStatus,
}

const CLAIMS: Symbol = symbol_short!("CLAIMS");
const ADMIN: Symbol = symbol_short!("ADMIN");
const GUARDIAN: Symbol = symbol_short!("GUARDIAN");
const PAUSE_STATE: Symbol = symbol_short!("PAUSED");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseState {
    pub is_paused: bool,
    pub paused_at: Option<u64>,
    pub paused_by: Option<Address>,
    pub reason: Option<Symbol>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimsEvent {
    ClaimSubmitted(u64, Address, i128),
    ClaimApproved(u64),
    ClaimSettled(u64),
    ContractPaused(Address, Option<Symbol>),
    ContractUnpaused(Address, Option<Symbol>),
}

#[derive(soroban_sdk::contracterror, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ClaimError {
    PolicyInactive = 1,
    InsufficientCoverage = 2,
    ClaimNotFound = 3,
    AlreadySettled = 4,
    ClaimNotApproved = 5,
    ContractPaused = 6,
    Unauthorized = 7,
}

#[contract]
pub struct ClaimsContract;

#[contractimpl]
impl ClaimsContract {
    pub fn initialize(env: Env, admin: Address, guardian: Address) {
        if env.storage().instance().has(&ADMIN) { panic!("Already initialized"); }
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&GUARDIAN, &guardian);
        env.storage().instance().set(&PAUSE_STATE, &PauseState { is_paused: false, paused_at: None, paused_by: None, reason: None });
    }

    pub fn set_pause_state(env: Env, caller: Address, is_paused: bool, reason: Option<Symbol>) -> Result<(), ClaimError> {
        caller.require_auth();
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        let guardian: Address = env.storage().instance().get(&GUARDIAN).unwrap();

        if caller != admin && caller != guardian { return Err(ClaimError::Unauthorized); }

        let pause_state = PauseState {
            is_paused,
            paused_at: if is_paused { Some(env.ledger().timestamp()) } else { None },
            paused_by: if is_paused { Some(caller.clone()) } else { None },
            reason: reason.clone(),
        };
        env.storage().instance().set(&PAUSE_STATE, &pause_state);

        if is_paused {
            env.events().publish((Symbol::short("PAUSE"), Symbol::short("PAUSED")), ClaimsEvent::ContractPaused(caller, reason));
        } else {
            env.events().publish((Symbol::short("PAUSE"), Symbol::short("UNPAUSED")), ClaimsEvent::ContractUnpaused(caller, reason));
        }
        Ok(())
    }

    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get::<_, PauseState>(&PAUSE_STATE).map(|s| s.is_paused).unwrap_or(false)
    }

    pub fn submit_claim(env: Env, policy_address: Address, claim_id: u64, policy_id: u64, amount: i128) -> Result<(), ClaimError> {
        if Self::is_paused(env.clone()) { return Err(ClaimError::ContractPaused); }
        let policy = PolicyClient::new(&env, &policy_address);
        if !policy.is_policy_active(&policy_id) { return Err(ClaimError::PolicyInactive); }
        let coverage = policy.get_policy_coverage(&policy_id);
        let fee = amount / 100;
        if coverage <= amount + fee { return Err(ClaimError::InsufficientCoverage); }
        env.storage().persistent().set(&(CLAIMS, claim_id), &ClaimRecord { policy_id, amount, status: ClaimStatus::Pending });
        env.events().publish((CLAIMS, Symbol::short("SUBMIT")), ClaimsEvent::ClaimSubmitted(claim_id, policy_address, amount));
        Ok(())
    }

    pub fn approve_claim(env: Env, claim_id: u64) -> Result<(), ClaimError> {
        if Self::is_paused(env.clone()) { return Err(ClaimError::ContractPaused); }
        let key = (CLAIMS, claim_id);
        let mut r: ClaimRecord = env.storage().persistent().get(&key).ok_or(ClaimError::ClaimNotFound)?;
        r.status = ClaimStatus::Approved;
        env.storage().persistent().set(&key, &r);
        env.events().publish((CLAIMS, Symbol::short("APPROVE")), ClaimsEvent::ClaimApproved(claim_id));
        Ok(())
    }

    pub fn settle_claim(env: Env, claim_id: u64) -> Result<(), ClaimError> {
        if Self::is_paused(env.clone()) { return Err(ClaimError::ContractPaused); }
        let key = (CLAIMS, claim_id);
        let mut r: ClaimRecord = env.storage().persistent().get(&key).ok_or(ClaimError::ClaimNotFound)?;
        if r.status == ClaimStatus::Settled { return Err(ClaimError::AlreadySettled); }
        if r.status != ClaimStatus::Approved { return Err(ClaimError::ClaimNotApproved); }
        r.status = ClaimStatus::Settled;
        env.storage().persistent().set(&key, &r);
        env.events().publish((CLAIMS, Symbol::short("SETTLE")), ClaimsEvent::ClaimSettled(claim_id));
        Ok(())
    }
}
