#![no_std]
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

#[contracterror]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyError {
    ContractPaused = 1,
    InvalidParameters = 2,
    Unauthorized = 3,
    PolicyNotFound = 4,
}

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
pub enum PolicyEvent {
    PolicyIssued(u64, Address, i128),
    PolicyCanceled(u64),
    ContractPaused(Address, Option<Symbol>),
    ContractUnpaused(Address, Option<Symbol>),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyRecord {
    pub coverage: i128,
    pub premium: i128,
    pub holder: Address,
    pub status: PolicyStatus,
}

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum PolicyStatus { Active, Expired, Cancelled }

// Risk Assessment structure for pricing
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskReport {
    pub property_id: u64,
    pub risk_score: u32,       // 0-100
    pub location_factor: u32,  // 100 = 1.0x
    pub coverage_ratio: u32,   // basis points
}

const POLICIES: Symbol = symbol_short!("POLICIES");
const ADMIN: Symbol = symbol_short!("ADMIN");
const GUARDIAN: Symbol = symbol_short!("GUARDIAN");
const PAUSE_STATE: Symbol = symbol_short!("PAUSED");

#[contract]
pub struct PolicyContract;

#[contractimpl]
impl PolicyContract {
    pub fn initialize(env: Env, admin: Address, guardian: Address) {
        if env.storage().instance().has(&ADMIN) { panic!("Already initialized"); }
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&GUARDIAN, &guardian);
        env.storage().instance().set(&PAUSE_STATE, &PauseState { is_paused: false, paused_at: None, paused_by: None, reason: None });
    }

    pub fn set_pause_state(env: Env, caller: Address, is_paused: bool, reason: Option<Symbol>) -> Result<(), PolicyError> {
        caller.require_auth();
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        let guardian: Address = env.storage().instance().get(&GUARDIAN).unwrap();

        if caller != admin && caller != guardian { return Err(PolicyError::Unauthorized); }

        let pause_state = PauseState {
            is_paused,
            paused_at: if is_paused { Some(env.ledger().timestamp()) } else { None },
            paused_by: if is_paused { Some(caller.clone()) } else { None },
            reason: reason.clone(),
        };
        env.storage().instance().set(&PAUSE_STATE, &pause_state);

        if is_paused {
            env.events().publish((Symbol::short("PAUSE"), Symbol::short("PAUSED")), PolicyEvent::ContractPaused(caller, reason));
        } else {
            env.events().publish((Symbol::short("PAUSE"), Symbol::short("UNPAUSED")), PolicyEvent::ContractUnpaused(caller, reason));
        }
        Ok(())
    }

    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get::<_, PauseState>(&PAUSE_STATE).map(|s| s.is_paused).unwrap_or(false)
    }

    pub fn calculate_dynamic_premium(
        _env: Env,
        risk_report: RiskReport,
        base_rate: i128,
        market_condition_factor: u32,
    ) -> i128 {
        let risk_multiplier = risk_report.risk_score as i128;
        let location_multiplier = risk_report.location_factor as i128;
        let ratio_multiplier = risk_report.coverage_ratio as i128;

        let mut premium = base_rate;
        premium = (premium * risk_multiplier) / 100;
        premium = (premium * location_multiplier) / 100;
        premium = (premium * market_condition_factor as i128) / 100;
        premium = (premium * ratio_multiplier) / 10000;

        premium
    }

    pub fn issue_policy(env: Env, holder: Address, policy_id: u64, coverage: i128, premium: i128) -> Result<(), PolicyError> {
        if Self::is_paused(env.clone()) { return Err(PolicyError::ContractPaused); }
        let key = (POLICIES, policy_id);
        env.storage().persistent().set(&key, &PolicyRecord { coverage, premium, holder: holder.clone(), status: PolicyStatus::Active });
        env.events().publish((POLICIES, Symbol::short("ISSUE")), PolicyEvent::PolicyIssued(policy_id, holder, coverage));
        Ok(())
    }

    pub fn cancel_policy(env: Env, policy_id: u64) -> Result<(), PolicyError> {
        if Self::is_paused(env.clone()) { return Err(PolicyError::ContractPaused); }
        let key = (POLICIES, policy_id);
        let mut r: PolicyRecord = env.storage().persistent().get(&key).ok_or(PolicyError::PolicyNotFound)?;
        r.status = PolicyStatus::Cancelled;
        env.storage().persistent().set(&key, &r);
        env.events().publish((POLICIES, Symbol::short("CANCEL")), PolicyEvent::PolicyCanceled(policy_id));
        Ok(())
    }

    pub fn expire_policy(env: Env, policy_id: u64) -> Result<(), PolicyError> {
        if Self::is_paused(env.clone()) { return Err(PolicyError::ContractPaused); }
        let key = (POLICIES, policy_id);
        let mut r: PolicyRecord = env.storage().persistent().get(&key).ok_or(PolicyError::PolicyNotFound)?;
        r.status = PolicyStatus::Expired;
        env.storage().persistent().set(&key, &r);
        Ok(())
    }

    pub fn is_policy_active(env: Env, policy_id: u64) -> bool {
        let key = (POLICIES, policy_id);
        match env.storage().persistent().get::<_, PolicyRecord>(&key) {
            Some(r) => r.status == PolicyStatus::Active,
            None => false,
        }
    }

    pub fn get_policy_coverage(env: Env, policy_id: u64) -> i128 {
        let key = (POLICIES, policy_id);
        match env.storage().persistent().get::<_, PolicyRecord>(&key) {
            Some(r) => r.coverage,
            None => 0,
        }
    }
}
