use soroban_sdk::{contracterror, contractimpl, contracttype, Address, Env, Symbol};

#[contracterror]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyError {
    ContractPaused = 1,
    InvalidParameters = 2,
    Unauthorized = 3,
    PolicyNotFound = 4,
}

// Pause state key
const PAUSE_STATE_KEY: Symbol = Symbol::short("PAUSED");

// Policy storage key prefix
const POLICY_PREFIX: Symbol = Symbol::short("POLICY");

// Policy count key
const POLICY_COUNT_KEY: Symbol = Symbol::short("COUNT");

// Pause state structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseState {
    pub is_paused: bool,
    pub paused_at: Option<u64>,
    pub paused_by: Option<Address>,
    pub pause_reason: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyEvent {
    PolicyIssued(PolicyContext),
    PolicyRenewed(PolicyContext),
    PolicyCanceled(PolicyContext, Option<String>),
    PolicyExpired(PolicyContext),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyContext {
    pub policy_id: u64,
    pub holder: Address,
    pub coverage_amount: i128,
    pub premium_amount: i128,
    pub duration_days: u32,
    pub policy_type: Symbol,
    pub timestamp: u64,
    pub status: PolicyStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyStatus {
    Active,
    Renewed,
    Cancelled,
    Expired,
}

pub struct PolicyContract;

#[contractimpl]
impl PolicyContract {
    // Initialize pause state
    pub fn initialize_pause(env: &Env) {
        if !env.storage().instance().has(&PAUSE_STATE_KEY) {
            let pause_state = PauseState {
                is_paused: false,
                paused_at: None,
                paused_by: None,
                pause_reason: None,
            };
            env.storage().instance().set(&PAUSE_STATE_KEY, &pause_state);
        }

        // Initialize policy count
        if !env.storage().instance().has(&POLICY_COUNT_KEY) {
            env.storage().instance().set(&POLICY_COUNT_KEY, &0u64);
        }
    }

    // Set pause state (only callable by governance)
    pub fn set_pause_state(env: &Env, is_paused: bool, reason: Option<String>) {
        let caller = env.current_contract_address(); // In real implementation, check governance authorization
        let current_time = env.ledger().timestamp();

        let pause_state = PauseState {
            is_paused,
            paused_at: if is_paused { Some(current_time) } else { None },
            paused_by: if is_paused { Some(caller) } else { None },
            pause_reason: reason,
        };

        env.storage().instance().set(&PAUSE_STATE_KEY, &pause_state);
    }

    // Check if contract is paused
    pub fn is_paused(env: &Env) -> bool {
        let pause_state: PauseState =
            env.storage()
                .instance()
                .get(&PAUSE_STATE_KEY)
                .unwrap_or(PauseState {
                    is_paused: false,
                    paused_at: None,
                    paused_by: None,
                    pause_reason: None,
                });
        pause_state.is_paused
    }

    // Get pause status
    pub fn get_pause_status(env: &Env) -> PauseState {
        env.storage()
            .instance()
            .get(&PAUSE_STATE_KEY)
            .unwrap_or(PauseState {
                is_paused: false,
                paused_at: None,
                paused_by: None,
                pause_reason: None,
            })
    }

    // Issue policy (with pause guard)
    pub fn issue_policy(
        env: &Env,
        holder: Address,
        coverage_amount: i128,
        premium_amount: i128,
        duration_days: u32,
        policy_type: Symbol,
    ) -> Result<u64, PolicyError> {
        // Check if contract is paused
        if Self::is_paused(env) {
            return Err(PolicyError::ContractPaused);
        }

        // Validate parameters
        if coverage_amount <= 0 || premium_amount <= 0 || duration_days == 0 {
            return Err(PolicyError::InvalidParameters);
        }

        // Generate unique policy ID
        let policy_count: u64 = env.storage().instance().get(&POLICY_COUNT_KEY).unwrap_or(0);
        let policy_id = policy_count + 1;
        env.storage().instance().set(&POLICY_COUNT_KEY, &policy_id);

        let timestamp = env.ledger().timestamp();

        let context = PolicyContext {
            policy_id,
            holder: holder.clone(),
            coverage_amount,
            premium_amount,
            duration_days,
            policy_type: policy_type.clone(),
            timestamp,
            status: PolicyStatus::Active,
        };

        // Store policy
        let policy_key = (POLICY_PREFIX.clone(), policy_id);
        env.storage().persistent().set(&policy_key, &context);

        // Emit comprehensive event for policy issuance
        env.events().publish(
            (Symbol::short("POLICY"), Symbol::short("ISSUED")),
            PolicyEvent::PolicyIssued(context.clone()),
        );

        Ok(policy_id)
    }

    // Renew policy (with pause guard)
    pub fn renew_policy(
        env: &Env,
        policy_id: u64,
        holder: Address,
        new_premium: i128,
        new_duration_days: Option<u32>,
    ) -> Result<(), PolicyError> {
        // Check if contract is paused
        if Self::is_paused(env) {
            return Err(PolicyError::ContractPaused);
        }

        // Validate parameters
        if new_premium <= 0 {
            return Err(PolicyError::InvalidParameters);
        }

        // Retrieve existing policy
        let policy_key = (POLICY_PREFIX.clone(), policy_id);
        let mut context: PolicyContext = env
            .storage()
            .persistent()
            .get(&policy_key)
            .ok_or(PolicyError::PolicyNotFound)?;

        // Verify holder authorization
        if context.holder != holder {
            return Err(PolicyError::Unauthorized);
        }

        let timestamp = env.ledger().timestamp();

        // Update policy details
        context.premium_amount = new_premium;
        if let Some(duration) = new_duration_days {
            context.duration_days = duration;
        }
        context.status = PolicyStatus::Renewed;
        context.timestamp = timestamp;

        // Store updated policy
        env.storage().persistent().set(&policy_key, &context);

        // Emit comprehensive event for policy renewal
        env.events().publish(
            (Symbol::short("POLICY"), Symbol::short("RENEWED")),
            PolicyEvent::PolicyRenewed(context.clone()),
        );

        Ok(())
    }

    // Cancel policy (with pause guard)
    pub fn cancel_policy(
        env: &Env,
        policy_id: u64,
        holder: Address,
        cancellation_reason: Option<String>,
    ) -> Result<(), PolicyError> {
        // Check if contract is paused
        if Self::is_paused(env) {
            return Err(PolicyError::ContractPaused);
        }

        // Retrieve existing policy
        let policy_key = (POLICY_PREFIX.clone(), policy_id);
        let mut context: PolicyContext = env
            .storage()
            .persistent()
            .get(&policy_key)
            .ok_or(PolicyError::PolicyNotFound)?;

        // Verify holder authorization
        if context.holder != holder {
            return Err(PolicyError::Unauthorized);
        }

        let timestamp = env.ledger().timestamp();

        // Update policy status
        context.status = PolicyStatus::Cancelled;
        context.timestamp = timestamp;
        context.policy_type = Symbol::short("CANCEL");

        // Store updated policy
        env.storage().persistent().set(&policy_key, &context);

        // Emit comprehensive event for policy cancellation
        env.events().publish(
            (Symbol::short("POLICY"), Symbol::short("CANCELED")),
            PolicyEvent::PolicyCanceled(context.clone(), cancellation_reason.clone()),
        );

        Ok(())
    }

    // Expire policy (with pause guard)
    pub fn expire_policy(env: &Env, policy_id: u64) -> Result<(), PolicyError> {
        // Check if contract is paused
        if Self::is_paused(env) {
            return Err(PolicyError::ContractPaused);
        }

        // Retrieve existing policy
        let policy_key = (POLICY_PREFIX.clone(), policy_id);
        let mut context: PolicyContext = env
            .storage()
            .persistent()
            .get(&policy_key)
            .ok_or(PolicyError::PolicyNotFound)?;

        let timestamp = env.ledger().timestamp();

        // Update policy status
        context.status = PolicyStatus::Expired;
        context.timestamp = timestamp;

        // Store updated policy
        env.storage().persistent().set(&policy_key, &context);

        // Emit comprehensive event for policy expiration
        env.events().publish(
            (Symbol::short("POLICY"), Symbol::short("EXPIRED")),
            PolicyEvent::PolicyExpired(context.clone()),
        );

        Ok(())
    }

    // Get policy details
    pub fn get_policy(env: &Env, policy_id: u64) -> Result<PolicyContext, PolicyError> {
        let policy_key = (POLICY_PREFIX.clone(), policy_id);
        env.storage()
            .persistent()
            .get(&policy_key)
            .ok_or(PolicyError::PolicyNotFound)
    }

    // Get total policy count
    pub fn get_policy_count(env: &Env) -> u64 {
        env.storage().instance().get(&POLICY_COUNT_KEY).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_initialize_pause() {
        let env = Env::default();
        PolicyContract::initialize_pause(&env);

        assert!(!PolicyContract::is_paused(&env));
        assert_eq!(PolicyContract::get_policy_count(&env), 0);
    }

    #[test]
    fn test_issue_policy_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        PolicyContract::initialize_pause(&env);

        let holder = Address::generate(&env);
        let coverage_amount = 100_000i128;
        let premium_amount = 1_000i128;
        let duration_days = 365u32;
        let policy_type = Symbol::short("STD");

        let policy_id = PolicyContract::issue_policy(
            &env,
            holder.clone(),
            coverage_amount,
            premium_amount,
            duration_days,
            policy_type.clone(),
        )
        .unwrap();

        assert_eq!(policy_id, 1);
        assert_eq!(PolicyContract::get_policy_count(&env), 1);

        // Verify event was emitted
        let events = env.events().all();
        assert_eq!(events.len(), 1);

        // Check event topic
        let event = events.get(0).unwrap();
        let topics = event.0;
        assert_eq!(topics.len(), 2);
        assert_eq!(topics.get(0).unwrap(), Symbol::short("POLICY"));
        assert_eq!(topics.get(1).unwrap(), Symbol::short("ISSUED"));
    }

    #[test]
    fn test_issue_policy_invalid_params() {
        let env = Env::default();
        PolicyContract::initialize_pause(&env);

        let holder = Address::generate(&env);

        // Test zero coverage amount
        let result =
            PolicyContract::issue_policy(&env, holder.clone(), 0, 1000, 365, Symbol::short("STD"));
        assert_eq!(result.unwrap_err(), PolicyError::InvalidParameters);

        // Test zero premium
        let result = PolicyContract::issue_policy(
            &env,
            holder.clone(),
            100000,
            0,
            365,
            Symbol::short("STD"),
        );
        assert_eq!(result.unwrap_err(), PolicyError::InvalidParameters);

        // Test zero duration
        let result =
            PolicyContract::issue_policy(&env, holder, 100000, 1000, 0, Symbol::short("STD"));
        assert_eq!(result.unwrap_err(), PolicyError::InvalidParameters);
    }

    #[test]
    fn test_renew_policy_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        PolicyContract::initialize_pause(&env);

        let holder = Address::generate(&env);

        // Issue a policy first
        let policy_id = PolicyContract::issue_policy(
            &env,
            holder.clone(),
            100_000i128,
            1_000i128,
            365u32,
            Symbol::short("STD"),
        )
        .unwrap();

        // Renew the policy
        let new_premium = 1_200i128;
        let new_duration = Some(730u32);

        PolicyContract::renew_policy(&env, policy_id, holder.clone(), new_premium, new_duration)
            .unwrap();

        // Verify event was emitted
        let events = env.events().all();
        assert_eq!(events.len(), 2); // Issue + Renew events

        let renew_event = events.get(1).unwrap();
        let topics = renew_event.0;
        assert_eq!(topics.get(1).unwrap(), Symbol::short("RENEWED"));
    }

    #[test]
    fn test_renew_policy_unauthorized() {
        let env = Env::default();
        PolicyContract::initialize_pause(&env);

        let holder = Address::generate(&env);
        let unauthorized = Address::generate(&env);

        // Issue a policy
        let policy_id = PolicyContract::issue_policy(
            &env,
            holder.clone(),
            100_000i128,
            1_000i128,
            365u32,
            Symbol::short("STD"),
        )
        .unwrap();

        // Try to renew with wrong holder
        let result = PolicyContract::renew_policy(&env, policy_id, unauthorized, 1_200, None);
        assert_eq!(result.unwrap_err(), PolicyError::Unauthorized);
    }

    #[test]
    fn test_cancel_policy_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        PolicyContract::initialize_pause(&env);

        let holder = Address::generate(&env);

        // Issue a policy first
        let policy_id = PolicyContract::issue_policy(
            &env,
            holder.clone(),
            100_000i128,
            1_000i128,
            365u32,
            Symbol::short("STD"),
        )
        .unwrap();

        // Cancel the policy with reason
        let reason = Some(String::from("User requested cancellation"));

        PolicyContract::cancel_policy(&env, policy_id, holder.clone(), reason.clone()).unwrap();

        // Verify event was emitted
        let events = env.events().all();
        assert_eq!(events.len(), 2); // Issue + Cancel events

        let cancel_event = events.get(1).unwrap();
        let topics = cancel_event.0;
        assert_eq!(topics.get(1).unwrap(), Symbol::short("CANCELED"));
    }

    #[test]
    fn test_expire_policy_emits_event() {
        let env = Env::default();
        PolicyContract::initialize_pause(&env);

        let holder = Address::generate(&env);

        // Issue a policy first
        let policy_id = PolicyContract::issue_policy(
            &env,
            holder.clone(),
            100_000i128,
            1_000i128,
            365u32,
            Symbol::short("STD"),
        )
        .unwrap();

        // Expire the policy
        PolicyContract::expire_policy(&env, policy_id).unwrap();

        // Verify event was emitted
        let events = env.events().all();
        assert_eq!(events.len(), 2); // Issue + Expire events

        let expire_event = events.get(1).unwrap();
        let topics = expire_event.0;
        assert_eq!(topics.get(1).unwrap(), Symbol::short("EXPIRED"));
    }

    #[test]
    fn test_get_policy_details() {
        let env = Env::default();
        PolicyContract::initialize_pause(&env);

        let holder = Address::generate(&env);
        let coverage_amount = 100_000i128;
        let premium_amount = 1_000i128;
        let duration_days = 365u32;
        let policy_type = Symbol::short("STD");

        let policy_id = PolicyContract::issue_policy(
            &env,
            holder.clone(),
            coverage_amount,
            premium_amount,
            duration_days,
            policy_type.clone(),
        )
        .unwrap();

        let policy = PolicyContract::get_policy(&env, policy_id).unwrap();

        assert_eq!(policy.policy_id, policy_id);
        assert_eq!(policy.holder, holder);
        assert_eq!(policy.coverage_amount, coverage_amount);
        assert_eq!(policy.premium_amount, premium_amount);
        assert_eq!(policy.duration_days, duration_days);
        assert_eq!(policy.policy_type, policy_type);
        assert_eq!(policy.status, PolicyStatus::Active);
    }

    #[test]
    fn test_get_policy_not_found() {
        let env = Env::default();
        PolicyContract::initialize_pause(&env);

        let result = PolicyContract::get_policy(&env, 999);
        assert_eq!(result.unwrap_err(), PolicyError::PolicyNotFound);
    }

    #[test]
    fn test_paused_contract_prevents_operations() {
        let env = Env::default();
        PolicyContract::initialize_pause(&env);

        let holder = Address::generate(&env);

        // Pause the contract
        PolicyContract::set_pause_state(&env, true, Some(String::from("Maintenance")));

        // Try to issue a policy
        let result = PolicyContract::issue_policy(
            &env,
            holder.clone(),
            100_000,
            1_000,
            365,
            Symbol::short("STD"),
        );
        assert_eq!(result.unwrap_err(), PolicyError::ContractPaused);
    }
}
