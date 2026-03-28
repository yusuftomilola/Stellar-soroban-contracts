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
    PolicyIssued(u64, Address, i128, u64), // policy_id, holder, coverage, expires_at
    PolicyCanceled(u64),
    PolicyExpired(u64),
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
    pub issued_at: u64,
    pub expires_at: u64,
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
const POLICY_INDEX: Symbol = symbol_short!("POLICY_INDEX");
const POLICY_COUNT: Symbol = symbol_short!("POLICY_COUNT");
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

    const DEFAULT_POLICY_DURATION_SECS: u64 = 365 * 86400;

    fn increment_policy_count(env: &Env) -> u64 {
        let count: u64 = env.storage().instance().get(&POLICY_COUNT).unwrap_or(0);
        let next = count + 1;
        env.storage().instance().set(&POLICY_COUNT, &next);
        next
    }

    fn record_policy_index(env: &Env, index: u64, policy_id: u64) {
        env.storage().persistent().set(&(POLICY_INDEX, index), &policy_id);
    }

    pub fn issue_policy(env: Env, holder: Address, policy_id: u64, coverage: i128, premium: i128) -> Result<(), PolicyError> {
        if Self::is_paused(env.clone()) { return Err(PolicyError::ContractPaused); }

        let now = env.ledger().timestamp();
        let expires_at = now.saturating_add(Self::DEFAULT_POLICY_DURATION_SECS);

        let key = (POLICIES, policy_id);
        env.storage().persistent().set(&key, &PolicyRecord { coverage, premium, holder: holder.clone(), status: PolicyStatus::Active, issued_at: now, expires_at });

        let next_index = Self::increment_policy_count(&env);
        Self::record_policy_index(&env, next_index, policy_id);

        env.events().publish((POLICIES, Symbol::short("ISSUE")), PolicyEvent::PolicyIssued(policy_id, holder, coverage, expires_at));
        Ok(())
    }

    pub fn issue_policy_with_duration(env: Env, holder: Address, policy_id: u64, coverage: i128, premium: i128, duration_secs: u64) -> Result<(), PolicyError> {
        if duration_secs == 0 { return Err(PolicyError::InvalidParameters); }
        if Self::is_paused(env.clone()) { return Err(PolicyError::ContractPaused); }

        let now = env.ledger().timestamp();
        let expires_at = now.saturating_add(duration_secs);

        let key = (POLICIES, policy_id);
        env.storage().persistent().set(&key, &PolicyRecord { coverage, premium, holder: holder.clone(), status: PolicyStatus::Active, issued_at: now, expires_at });

        let next_index = Self::increment_policy_count(&env);
        Self::record_policy_index(&env, next_index, policy_id);

        env.events().publish((POLICIES, Symbol::short("ISSUE")), PolicyEvent::PolicyIssued(policy_id, holder, coverage, expires_at));
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

    fn expire_policy_internal(env: &Env, policy_id: u64) -> Result<(), PolicyError> {
        let key = (POLICIES, policy_id);
        let mut r: PolicyRecord = env.storage().persistent().get(&key).ok_or(PolicyError::PolicyNotFound)?;
        if r.status != PolicyStatus::Active {
            return Ok(());
        }
        r.status = PolicyStatus::Expired;
        env.storage().persistent().set(&key, &r);
        env.events().publish((POLICIES, Symbol::short("EXPIRED")), PolicyEvent::PolicyExpired(policy_id));
        Ok(())
    }

    pub fn expire_policy(env: Env, policy_id: u64) -> Result<(), PolicyError> {
        if Self::is_paused(env.clone()) { return Err(PolicyError::ContractPaused); }
        Self::expire_policy_internal(&env, policy_id)
    }

    pub fn check_and_expire_policies(env: Env, start_index: u64, max_items: u64) -> Result<(u64, u64), PolicyError> {
        if Self::is_paused(env.clone()) { return Err(PolicyError::ContractPaused); }

        let total = env.storage().instance().get(&POLICY_COUNT).unwrap_or(0);
        if start_index >= total { return Ok((0, start_index)); }

        let end_index = core::cmp::min(start_index + max_items, total);
        let now = env.ledger().timestamp();
        let mut expired_count: u64 = 0;

        for idx in start_index..end_index {
            let policy_id: u64 = env.storage().persistent().get(&(POLICY_INDEX, idx + 1)).unwrap();
            let key = (POLICIES, policy_id);
            if let Some(policy) = env.storage().persistent().get::<_, PolicyRecord>(&key) {
                if policy.status == PolicyStatus::Active && policy.expires_at <= now {
                    Self::expire_policy_internal(&env, policy_id)?;
                    expired_count += 1;
                }
            }
        }

        Ok((expired_count, end_index))
    }

    pub fn query_active_policies_by_expiration(env: Env, start_index: u64, max_items: u64, until_ts: u64) -> Vec<u64> {
        let total = env.storage().instance().get(&POLICY_COUNT).unwrap_or(0);
        let end_index = core::cmp::min(start_index + max_items, total);
        let mut results = Vec::new(env);

        for idx in start_index..end_index {
            let policy_id: u64 = env.storage().persistent().get(&(POLICY_INDEX, idx + 1)).unwrap();
            let key = (POLICIES, policy_id);
            if let Some(policy) = env.storage().persistent().get::<_, PolicyRecord>(&key) {
                if policy.status == PolicyStatus::Active && policy.expires_at <= until_ts {
                    results.push_back(policy_id);
                }
            }
        }

        results
    }

    pub fn get_policy_count(env: Env) -> u64 {
        env.storage().instance().get(&POLICY_COUNT).unwrap_or(0)
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

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

    #[test]
    fn test_check_and_expire_policies() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let guardian = Address::generate(&env);

        PolicyContract::initialize(env.clone(), admin.clone(), guardian.clone());

        let holder = Address::generate(&env);

        PolicyContract::issue_policy_with_duration(
            env.clone(),
            holder.clone(),
            1,
            100_000i128,
            1_000i128,
            1,
        )
        .unwrap();

        PolicyContract::issue_policy_with_duration(
            env.clone(),
            holder.clone(),
            2,
            200_000i128,
            2_000i128,
            2,
        )
        .unwrap();

        assert_eq!(PolicyContract::get_policy_count(env.clone()), 2);

        env.ledger().set_timestamp(env.ledger().timestamp() + 3);

        let (expired, next_index) = PolicyContract::check_and_expire_policies(env.clone(), 0, 5).unwrap();
        assert_eq!(expired, 2);
        assert_eq!(next_index, 2);

        assert_eq!(PolicyContract::is_policy_active(env.clone(), 1), false);
        assert_eq!(PolicyContract::is_policy_active(env.clone(), 2), false);

        let expired_list = PolicyContract::query_active_policies_by_expiration(env.clone(), 0, 5, env.ledger().timestamp());
        assert_eq!(expired_list.len(), 0);
    }
}
