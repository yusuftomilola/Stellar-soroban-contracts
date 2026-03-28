use sha2::{Digest, Sha256};
use soroban_sdk::{
    contracterror, contractimpl, contracttype, env, unwrap_or_panic, Address, BytesN, Map, Symbol,
    Vec, U256,
};

#[cfg(test)]
mod tests;

// Contract storage keys
const DATA_KEY: Symbol = Symbol::short("DATA");
const PROPOSAL_KEY: Symbol = Symbol::short("PROPOSAL");
const VOTE_KEY: Symbol = Symbol::short("VOTE");
const PROPOSER_STATS_KEY: Symbol = Symbol::short("PROPOSER_STATS");
const PROPOSAL_HASHES_KEY: Symbol = Symbol::short("PROPOSAL_HASHES");
const COMMIT_REVEAL_KEY: Symbol = Symbol::short("COMMIT_REVEAL");
const PAUSE_STATE_KEY: Symbol = Symbol::short("PAUSE_STATE");
const PAUSE_HISTORY_KEY: Symbol = Symbol::short("PAUSE_HISTORY");
// Multi-sig storage keys
const MULTI_SIG_CONFIG_KEY: Symbol = Symbol::short("MSIG_CFG");
const MULTI_SIG_PROPOSAL_KEY: Symbol = Symbol::short("MSIG_PRP");
const MULTI_SIG_CONFIRMATIONS_KEY: Symbol = Symbol::short("MSIG_CFM");

// Custom errors
#[contracterror]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GovernanceError {
    InsufficientDeposit = 1,
    ProposalTooFrequent = 2,
    ProposalDuplicate = 3,
    ProposalNotFound = 4,
    NotAuthorized = 5,
    VotingPeriodNotEnded = 6,
    VotingPeriodEnded = 7,
    AlreadyVoted = 8,
    QuorumNotMet = 9,
    ThresholdNotMet = 10,
    InvalidCommitReveal = 11,
    RevealPeriodNotEnded = 12,
    TimeLockNotExpired = 13,
    ContractAlreadyPaused = 14,
    ContractNotPaused = 15,
    InvalidPauseAction = 16,
    // Multi-sig errors
    MultiSigNotConfigured = 17,
    MultiSigThresholdNotMet = 18,
    MultiSigAlreadyConfirmed = 19,
    MultiSigNotConfirmed = 20,
    MultiSigInvalidSigner = 21,
    MultiSigProposalAlreadyExecuted = 22,
}

// Proposal status
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Executed,
    Committed, // For commit-reveal mechanism
}

// Proposal structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub title: String,
    pub description: String,
    pub execution_data: BytesN<32>,
    pub threshold_percentage: u32,
    pub deposit_amount: i128,
    pub created_at: u64,
    pub voting_deadline: u64,
    pub reveal_deadline: Option<u64>,  // For commit-reveal
    pub time_lock_expiry: Option<u64>, // For time-lock
    pub status: ProposalStatus,
    pub yes_votes: i128,
    pub no_votes: i128,
    pub total_voting_power: i128,
}

// Vote record
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vote {
    pub voter: Address,
    pub weight: i128,
    pub is_yes: bool,
    pub timestamp: u64,
}

// Commit-reveal structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitReveal {
    pub commitment: BytesN<32>,
    pub reveal: Option<BytesN<32>>,
    pub revealed_at: Option<u64>,
}

// Contract data
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceData {
    pub admin: Address,
    pub token_contract: Address,
    pub voting_period_days: u32,
    pub min_voting_percentage: u32,
    pub min_quorum_percentage: u32,
    pub min_proposal_deposit: i128,
    pub max_proposals_per_proposer: u32,
    pub proposal_cooldown_seconds: u32,
    pub commit_reveal_enabled: bool,
    pub commit_period_days: u32,
    pub reveal_period_days: u32,
    pub time_lock_enabled: bool,
    pub time_lock_seconds: u32,
    pub next_proposal_id: u64,
    pub pause_quorum_percentage: u32,
    pub pause_threshold_percentage: u32,
}

// Proposer statistics
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposerStats {
    pub active_proposal_count: u32,
    pub total_proposal_count: u32,
    pub last_proposal_at: u64,
}

// Pause state structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseState {
    pub is_paused: bool,
    pub paused_at: Option<u64>,
    pub paused_by: Option<Address>,
    pub pause_reason: Option<String>,
}

// Pause history entry
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseHistoryEntry {
    pub timestamp: u64,
    pub action: PauseAction,
    pub actor: Address,
    pub reason: Option<String>,
    pub proposal_id: Option<u64>,
}

// Pause action types
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PauseAction {
    Pause,
    Unpause,
}

// =====================================================================
// MULTI-SIGNATURE TYPES
// =====================================================================

// Signer information with weight
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerInfo {
    pub address: Address,
    pub weight: u32,
    pub active: bool,
}

// Multi-signature configuration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigConfig {
    pub signers: Vec<SignerInfo>,
    pub threshold_weight: u32,
    pub enabled: bool,
    pub configured_at: u64,
    pub configured_by: Address,
}

// Multi-signature proposal for critical operations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigProposal {
    pub id: u64,
    pub operation_type: OperationType,
    pub operation_data: Map<Symbol, String>,
    pub target_contract: Option<Address>,
    pub created_at: u64,
    pub expires_at: u64,
    pub created_by: Address,
    pub total_weight: u32,
    pub confirmed_weight: u32,
    pub executed: bool,
    pub executed_at: Option<u64>,
}

// Operation types that require multi-sig
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationType {
    Pause,
    Unpause,
    UpdateParameter(Symbol), // Parameter name
    EmergencyShutdown,
    UpgradeContract,
    TreasuryWithdraw,
    AddSigner,
    RemoveSigner,
    UpdateThreshold,
}

// Confirmation record
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Confirmation {
    pub signer: Address,
    pub weight: u32,
    pub confirmed_at: u64,
}

pub struct GovernanceContract;

#[contractimpl]
impl GovernanceContract {
    // Initialize the governance contract
    pub fn initialize(
        env: &Env,
        admin: Address,
        token_contract: Address,
        voting_period_days: u32,
        min_voting_percentage: u32,
        min_quorum_percentage: u32,
        min_proposal_deposit: i128,
        max_proposals_per_proposer: u32,
        proposal_cooldown_seconds: u32,
        commit_reveal_enabled: bool,
        commit_period_days: u32,
        reveal_period_days: u32,
        time_lock_enabled: bool,
        time_lock_seconds: u32,
        pause_quorum_percentage: u32,
        pause_threshold_percentage: u32,
    ) {
        // Only allow initialization once
        if env.storage().instance().has(&DATA_KEY) {
            panic!("Contract already initialized");
        }

        let data = GovernanceData {
            admin: admin.clone(),
            token_contract,
            voting_period_days,
            min_voting_percentage,
            min_quorum_percentage,
            min_proposal_deposit,
            max_proposals_per_proposer,
            proposal_cooldown_seconds,
            commit_reveal_enabled,
            commit_period_days,
            reveal_period_days,
            time_lock_enabled,
            time_lock_seconds,
            next_proposal_id: 1,
            pause_quorum_percentage,
            pause_threshold_percentage,
        };

        env.storage().instance().set(&DATA_KEY, &data);

        // Initialize pause state
        let pause_state = PauseState {
            is_paused: false,
            paused_at: None,
            paused_by: None,
            pause_reason: None,
        };
        env.storage().instance().set(&PAUSE_STATE_KEY, &pause_state);
    }

    // =====================================================================
    // MULTI-SIGNATURE CONFIGURATION FUNCTIONS
    // =====================================================================

    // Configure multi-signature wallet for critical operations
    pub fn configure_multi_sig(
        env: &Env,
        admin: Address,
        signers: Vec<SignerInfo>,
        threshold_weight: u32,
    ) -> Result<(), GovernanceError> {
        let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();

        // Only admin can configure multi-sig
        if admin != data.admin {
            return Err(GovernanceError::NotAuthorized);
        }

        // Validate signers
        if signers.is_empty() {
            panic!("At least one signer required");
        }

        // Validate threshold
        let mut total_weight = 0u32;
        for signer in signers.iter() {
            if !signer.active {
                continue;
            }
            total_weight += signer.weight;
        }

        if threshold_weight == 0 || threshold_weight > total_weight {
            panic!("Invalid threshold weight");
        }

        // Create multi-sig configuration
        let config = MultiSigConfig {
            signers: signers.clone(),
            threshold_weight,
            enabled: true,
            configured_at: env.ledger().timestamp(),
            configured_by: admin.clone(),
        };

        env.storage().instance().set(&MULTI_SIG_CONFIG_KEY, &config);

        Ok(())
    }

    // Update multi-sig threshold (requires multi-sig approval if already configured)
    pub fn update_multi_sig_threshold(
        env: &Env,
        caller: Address,
        new_threshold: u32,
    ) -> Result<(), GovernanceError> {
        let mut config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&MULTI_SIG_CONFIG_KEY)
            .ok_or(GovernanceError::MultiSigNotConfigured)?;

        let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();

        // Either admin or multi-sig approval required
        let is_admin = caller == data.admin;
        let is_multi_sig_approved = Self::is_signer(env, &caller, &config);

        if !is_admin && !is_multi_sig_approved {
            return Err(GovernanceError::NotAuthorized);
        }

        // Validate new threshold
        let mut total_weight = 0u32;
        for signer in config.signers.iter() {
            if signer.active {
                total_weight += signer.weight;
            }
        }

        if new_threshold == 0 || new_threshold > total_weight {
            panic!("Invalid threshold weight");
        }

        config.threshold_weight = new_threshold;
        env.storage().instance().set(&MULTI_SIG_CONFIG_KEY, &config);

        Ok(())
    }

    // Add a new signer (requires multi-sig approval)
    pub fn add_signer(
        env: &Env,
        caller: Address,
        signer_address: Address,
        weight: u32,
    ) -> Result<(), GovernanceError> {
        let mut config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&MULTI_SIG_CONFIG_KEY)
            .ok_or(GovernanceError::MultiSigNotConfigured)?;

        // Check if already a signer
        for signer in config.signers.iter() {
            if signer.address == signer_address {
                panic!("Signer already exists");
            }
        }

        // Require multi-sig approval for adding signers
        if !Self::is_signer(env, &caller, &config) {
            return Err(GovernanceError::NotAuthorized);
        }

        // Add new signer
        let new_signer = SignerInfo {
            address: signer_address.clone(),
            weight,
            active: true,
        };

        let mut signers = config.signers;
        signers.push_back(new_signer);
        config.signers = signers;

        env.storage().instance().set(&MULTI_SIG_CONFIG_KEY, &config);

        Ok(())
    }

    // Remove a signer (requires multi-sig approval)
    pub fn remove_signer(
        env: &Env,
        caller: Address,
        signer_address: Address,
    ) -> Result<(), GovernanceError> {
        let mut config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&MULTI_SIG_CONFIG_KEY)
            .ok_or(GovernanceError::MultiSigNotConfigured)?;

        // Require multi-sig approval for removing signers
        if !Self::is_signer(env, &caller, &config) {
            return Err(GovernanceError::NotAuthorized);
        }

        // Remove signer
        let mut new_signers = Vec::new(env);
        for signer in config.signers.iter() {
            if signer.address != signer_address {
                new_signers.push_back(signer);
            }
        }

        if new_signers.len() == config.signers.len() {
            panic!("Signer not found");
        }

        config.signers = new_signers;
        env.storage().instance().set(&MULTI_SIG_CONFIG_KEY, &config);

        Ok(())
    }

    // Get multi-sig configuration
    pub fn get_multi_sig_config(env: &Env) -> Option<MultiSigConfig> {
        env.storage().instance().get(&MULTI_SIG_CONFIG_KEY)
    }

    // Check if an address is a valid signer
    pub fn is_signer(env: &Env, address: &Address, config: &MultiSigConfig) -> bool {
        for signer in config.signers.iter() {
            if signer.address == *address && signer.active {
                return true;
            }
        }
        false
    }

    // Get signer weight
    pub fn get_signer_weight(env: &Env, address: &Address, config: &MultiSigConfig) -> u32 {
        for signer in config.signers.iter() {
            if signer.address == *address && signer.active {
                return signer.weight;
            }
        }
        0
    }

    // =====================================================================
    // MULTI-SIGNATURE PROPOSAL AND CONFIRMATION FUNCTIONS
    // =====================================================================

    // Create a multi-signature proposal for critical operations
    pub fn create_multi_sig_proposal(
        env: &Env,
        creator: Address,
        operation_type: OperationType,
        operation_data: Map<Symbol, String>,
        target_contract: Option<Address>,
        expiry_seconds: u64,
    ) -> Result<u64, GovernanceError> {
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&MULTI_SIG_CONFIG_KEY)
            .ok_or(GovernanceError::MultiSigNotConfigured)?;

        // Creator must be a signer
        if !Self::is_signer(env, &creator, &config) {
            return Err(GovernanceError::MultiSigInvalidSigner);
        }

        // Get next proposal ID
        let next_proposal_id: u64 = env
            .storage()
            .instance()
            .get(&Symbol::short("MSIG_NEXT_ID"))
            .unwrap_or(1);

        let current_time = env.ledger().timestamp();
        let expires_at = current_time + expiry_seconds;

        // Create multi-sig proposal
        let proposal = MultiSigProposal {
            id: next_proposal_id,
            operation_type: operation_type.clone(),
            operation_data: operation_data.clone(),
            target_contract,
            created_at: current_time,
            expires_at,
            created_by: creator.clone(),
            total_weight: config.threshold_weight,
            confirmed_weight: 0,
            executed: false,
            executed_at: None,
        };

        // Store proposal
        let proposal_key = (MULTI_SIG_PROPOSAL_KEY, next_proposal_id);
        env.storage().persistent().set(&proposal_key, &proposal);

        // Auto-confirm by creator
        Self::confirm_multi_sig_proposal(env, creator.clone(), next_proposal_id)?;

        // Update next proposal ID
        env.storage()
            .instance()
            .set(&Symbol::short("MSIG_NEXT_ID"), &(next_proposal_id + 1));

        Ok(next_proposal_id)
    }

    // Confirm a multi-signature proposal
    pub fn confirm_multi_sig_proposal(
        env: &Env,
        signer: Address,
        proposal_id: u64,
    ) -> Result<(), GovernanceError> {
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&MULTI_SIG_CONFIG_KEY)
            .ok_or(GovernanceError::MultiSigNotConfigured)?;

        // Check if signer is valid
        if !Self::is_signer(env, &signer, &config) {
            return Err(GovernanceError::MultiSigInvalidSigner);
        }

        // Get proposal
        let proposal_key = (MULTI_SIG_PROPOSAL_KEY, proposal_id);
        let mut proposal: MultiSigProposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .ok_or(GovernanceError::ProposalNotFound)?;

        // Check if already executed
        if proposal.executed {
            return Err(GovernanceError::MultiSigProposalAlreadyExecuted);
        }

        // Check if expired
        let current_time = env.ledger().timestamp();
        if current_time > proposal.expires_at {
            panic!("Proposal expired");
        }

        // Check if already confirmed
        let confirmation_key = (MULTI_SIG_CONFIRMATIONS_KEY, proposal_id, signer.clone());
        if env.storage().persistent().has(&confirmation_key) {
            return Err(GovernanceError::MultiSigAlreadyConfirmed);
        }

        // Get signer weight
        let signer_weight = Self::get_signer_weight(env, &signer, &config);

        // Record confirmation
        let confirmation = Confirmation {
            signer: signer.clone(),
            weight: signer_weight,
            confirmed_at: current_time,
        };
        env.storage()
            .persistent()
            .set(&confirmation_key, &confirmation);

        // Update proposal confirmed weight
        proposal.confirmed_weight += signer_weight;
        env.storage().persistent().set(&proposal_key, &proposal);

        Ok(())
    }

    // Revoke confirmation from a multi-sig proposal
    pub fn revoke_multi_sig_confirmation(
        env: &Env,
        signer: Address,
        proposal_id: u64,
    ) -> Result<(), GovernanceError> {
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&MULTI_SIG_CONFIG_KEY)
            .ok_or(GovernanceError::MultiSigNotConfigured)?;

        // Get proposal
        let proposal_key = (MULTI_SIG_PROPOSAL_KEY, proposal_id);
        let mut proposal: MultiSigProposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .ok_or(GovernanceError::ProposalNotFound)?;

        // Check if already executed
        if proposal.executed {
            return Err(GovernanceError::MultiSigProposalAlreadyExecuted);
        }

        // Get confirmation
        let confirmation_key = (MULTI_SIG_CONFIRMATIONS_KEY, proposal_id, signer.clone());
        let confirmation: Confirmation = env
            .storage()
            .persistent()
            .get(&confirmation_key)
            .ok_or(GovernanceError::MultiSigNotConfirmed)?;

        // Remove confirmation
        env.storage().persistent().remove(&confirmation_key);

        // Update proposal confirmed weight
        proposal.confirmed_weight -= confirmation.weight;
        env.storage().persistent().set(&proposal_key, &proposal);

        Ok(())
    }

    // Execute a multi-signature proposal if threshold is met
    pub fn execute_multi_sig_proposal(
        env: &Env,
        executor: Address,
        proposal_id: u64,
    ) -> Result<(), GovernanceError> {
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&MULTI_SIG_CONFIG_KEY)
            .ok_or(GovernanceError::MultiSigNotConfigured)?;

        // Get proposal
        let proposal_key = (MULTI_SIG_PROPOSAL_KEY, proposal_id);
        let mut proposal: MultiSigProposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .ok_or(GovernanceError::ProposalNotFound)?;

        // Check if already executed
        if proposal.executed {
            return Err(GovernanceError::MultiSigProposalAlreadyExecuted);
        }

        // Check if expired
        let current_time = env.ledger().timestamp();
        if current_time > proposal.expires_at {
            panic!("Proposal expired");
        }

        // Check if threshold met
        if proposal.confirmed_weight < config.threshold_weight {
            return Err(GovernanceError::MultiSigThresholdNotMet);
        }

        // Execute based on operation type
        match &proposal.operation_type {
            OperationType::Pause => {
                let reason = proposal
                    .operation_data
                    .get(&Symbol::short("reason"))
                    .unwrap_or(String::from_str(env, "Multi-sig pause"));
                Self::execute_multi_sig_pause(env, proposal_id, reason)?;
            }
            OperationType::Unpause => {
                let reason = proposal
                    .operation_data
                    .get(&Symbol::short("reason"))
                    .unwrap_or(String::from_str(env, "Multi-sig unpause"));
                Self::execute_multi_sig_unpause(env, proposal_id, reason)?;
            }
            OperationType::UpdateParameter(param_name) => {
                let new_value = proposal
                    .operation_data
                    .get(&Symbol::short("value"))
                    .ok_or(GovernanceError::NotAuthorized)?;
                Self::execute_multi_sig_parameter_update(env, param_name.clone(), new_value)?;
            }
            OperationType::EmergencyShutdown => {
                Self::execute_multi_sig_emergency_shutdown(env, proposal_id)?;
            }
            OperationType::UpgradeContract => {
                let new_code_hash = proposal
                    .operation_data
                    .get(&Symbol::short("code_hash"))
                    .ok_or(GovernanceError::NotAuthorized)?;
                Self::execute_multi_sig_upgrade(env, proposal_id, new_code_hash)?;
            }
            OperationType::TreasuryWithdraw => {
                let amount_str = proposal
                    .operation_data
                    .get(&Symbol::short("amount"))
                    .ok_or(GovernanceError::NotAuthorized)?;
                let recipient_str = proposal
                    .operation_data
                    .get(&Symbol::short("recipient"))
                    .ok_or(GovernanceError::NotAuthorized)?;
                Self::execute_multi_sig_treasury_withdraw(
                    env,
                    proposal_id,
                    amount_str,
                    recipient_str,
                )?;
            }
            _ => {
                panic!("Unsupported operation type");
            }
        }

        // Mark as executed
        proposal.executed = true;
        proposal.executed_at = Some(current_time);
        env.storage().persistent().set(&proposal_key, &proposal);

        Ok(())
    }

    // Get multi-sig proposal details
    pub fn get_multi_sig_proposal(
        env: &Env,
        proposal_id: u64,
    ) -> Result<MultiSigProposal, GovernanceError> {
        let proposal_key = (MULTI_SIG_PROPOSAL_KEY, proposal_id);
        env.storage()
            .persistent()
            .get(&proposal_key)
            .ok_or(GovernanceError::ProposalNotFound)
    }

    // Get confirmations for a multi-sig proposal
    pub fn get_multi_sig_confirmations(env: &Env, proposal_id: u64) -> Vec<Confirmation> {
        let mut confirmations = Vec::new(env);

        // Iterate through all storage keys to find confirmations for this proposal
        // Note: In production, you'd want a more efficient indexing mechanism
        let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();

        for i in 1..data.next_proposal_id {
            // Try to get confirmation for each potential signer
            // This is a simplified approach - in production use a proper index
            let config: Option<MultiSigConfig> =
                env.storage().instance().get(&MULTI_SIG_CONFIG_KEY);
            if let Some(config) = config {
                for signer in config.signers.iter() {
                    let confirmation_key = (
                        MULTI_SIG_CONFIRMATIONS_KEY,
                        proposal_id,
                        signer.address.clone(),
                    );
                    if let Some(confirmation) = env.storage().persistent().get(&confirmation_key) {
                        confirmations.push_back(confirmation);
                    }
                }
            }
        }

        confirmations
    }

    // Check if multi-sig is enabled and configured
    pub fn is_multi_sig_enabled(env: &Env) -> bool {
        if let Some(config) = Self::get_multi_sig_config(env) {
            config.enabled && !config.signers.is_empty()
        } else {
            false
        }
    }

    // =====================================================================
    // MULTI-SIG OPERATION EXECUTORS (INTERNAL)
    // =====================================================================

    // Execute pause via multi-sig
    fn execute_multi_sig_pause(
        env: &Env,
        proposal_id: u64,
        reason: String,
    ) -> Result<(), GovernanceError> {
        let current_time = env.ledger().timestamp();

        // Get proposal to find creator
        let proposal_key = (MULTI_SIG_PROPOSAL_KEY, proposal_id);
        let proposal: MultiSigProposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .ok_or(GovernanceError::ProposalNotFound)?;

        // Check if already paused
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

        if pause_state.is_paused {
            return Err(GovernanceError::ContractAlreadyPaused);
        }

        // Execute pause
        let new_pause_state = PauseState {
            is_paused: true,
            paused_at: Some(current_time),
            paused_by: Some(proposal.created_by.clone()),
            pause_reason: Some(reason.clone()),
        };

        env.storage()
            .instance()
            .set(&PAUSE_STATE_KEY, &new_pause_state);

        // Add to pause history
        Self::add_pause_history_entry(
            env,
            PauseAction::Pause,
            proposal.created_by.clone(),
            Some(reason),
            Some(proposal_id),
        );

        Ok(())
    }

    // Execute unpause via multi-sig
    fn execute_multi_sig_unpause(
        env: &Env,
        proposal_id: u64,
        reason: String,
    ) -> Result<(), GovernanceError> {
        let current_time = env.ledger().timestamp();

        // Get proposal to find creator
        let proposal_key = (MULTI_SIG_PROPOSAL_KEY, proposal_id);
        let proposal: MultiSigProposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .ok_or(GovernanceError::ProposalNotFound)?;

        // Check if not paused
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

        if !pause_state.is_paused {
            return Err(GovernanceError::ContractNotPaused);
        }

        // Execute unpause
        let new_pause_state = PauseState {
            is_paused: false,
            paused_at: None,
            paused_by: None,
            pause_reason: None,
        };

        env.storage()
            .instance()
            .set(&PAUSE_STATE_KEY, &new_pause_state);

        // Add to pause history
        Self::add_pause_history_entry(
            env,
            PauseAction::Unpause,
            proposal.created_by.clone(),
            Some(reason),
            Some(proposal_id),
        );

        Ok(())
    }

    // Execute parameter update via multi-sig
    fn execute_multi_sig_parameter_update(
        env: &Env,
        param_name: Symbol,
        new_value: String,
    ) -> Result<(), GovernanceError> {
        let mut data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();

        // Parse and update parameter based on name
        match param_name.to_string().as_str() {
            "voting_period_days" => {
                data.voting_period_days = new_value.parse().unwrap_or_panic();
            }
            "min_voting_percentage" => {
                data.min_voting_percentage = new_value.parse().unwrap_or_panic();
            }
            "min_quorum_percentage" => {
                data.min_quorum_percentage = new_value.parse().unwrap_or_panic();
            }
            "min_proposal_deposit" => {
                data.min_proposal_deposit = new_value.parse().unwrap_or_panic();
            }
            "pause_threshold_percentage" => {
                data.pause_threshold_percentage = new_value.parse().unwrap_or_panic();
            }
            "pause_quorum_percentage" => {
                data.pause_quorum_percentage = new_value.parse().unwrap_or_panic();
            }
            _ => {
                panic!("Unknown parameter");
            }
        }

        env.storage().instance().set(&DATA_KEY, &data);
        Ok(())
    }

    // Execute emergency shutdown via multi-sig
    fn execute_multi_sig_emergency_shutdown(
        env: &Env,
        proposal_id: u64,
    ) -> Result<(), GovernanceError> {
        // Emergency shutdown - pause all operations
        let current_time = env.ledger().timestamp();

        let proposal_key = (MULTI_SIG_PROPOSAL_KEY, proposal_id);
        let proposal: MultiSigProposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .ok_or(GovernanceError::ProposalNotFound)?;

        let new_pause_state = PauseState {
            is_paused: true,
            paused_at: Some(current_time),
            paused_by: Some(proposal.created_by.clone()),
            pause_reason: Some(String::from_str(env, "Emergency shutdown")),
        };

        env.storage()
            .instance()
            .set(&PAUSE_STATE_KEY, &new_pause_state);

        Ok(())
    }

    // Execute contract upgrade via multi-sig
    fn execute_multi_sig_upgrade(
        env: &Env,
        proposal_id: u64,
        new_code_hash: String,
    ) -> Result<(), GovernanceError> {
        // Note: Actual contract upgrade would require Soroban's upgrade mechanism
        // This is a placeholder for the upgrade logic

        // Store the new code hash for reference
        let upgrade_key = (Symbol::short("PENDING_UPGRADE"), proposal_id);
        env.storage().persistent().set(&upgrade_key, &new_code_hash);

        Ok(())
    }

    // Execute treasury withdraw via multi-sig
    fn execute_multi_sig_treasury_withdraw(
        env: &Env,
        proposal_id: u64,
        amount: String,
        recipient: String,
    ) -> Result<(), GovernanceError> {
        // Note: Actual withdrawal would require token contract integration
        // This is a placeholder for the withdrawal logic

        // Store withdrawal request for processing
        let withdraw_key = (Symbol::short("PENDING_WITHDRAW"), proposal_id);
        let withdraw_data = Map::from_array(
            env,
            &[
                (Symbol::short("amount"), amount),
                (Symbol::short("recipient"), recipient),
            ],
        );
        env.storage()
            .persistent()
            .set(&withdraw_key, &withdraw_data);

        Ok(())
    }

    // Create a new proposal with spam protection
    pub fn create_proposal(
        env: &Env,
        proposer: Address,
        title: String,
        description: String,
        execution_data: BytesN<32>,
        threshold_percentage: u32,
        deposit_amount: i128,
        commitment: Option<BytesN<32>>, // For commit-reveal
    ) -> Result<u64, GovernanceError> {
        // Check if contract is paused
        if Self::is_paused(env) {
            return Err(GovernanceError::ContractAlreadyPaused);
        }

        let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();

        // Check minimum deposit
        if deposit_amount < data.min_proposal_deposit {
            return Err(GovernanceError::InsufficientDeposit);
        }

        // Check proposer cooldown
        let proposer_key = (PROPOSER_STATS_KEY, proposer.clone());
        let proposer_stats: Option<ProposerStats> = env.storage().persistent().get(&proposer_key);

        if let Some(stats) = proposer_stats {
            let current_time = env.ledger().timestamp();
            if current_time < stats.last_proposal_at + data.proposal_cooldown_seconds as u64 {
                return Err(GovernanceError::ProposalTooFrequent);
            }

            // Check max active proposals per proposer
            if stats.active_proposal_count >= data.max_proposals_per_proposer {
                return Err(GovernanceError::ProposalTooFrequent);
            }
        }

        // Check proposal uniqueness by hash
        let proposal_hash = Self::generate_proposal_hash(&title, &execution_data);
        let hashes_key = (PROPOSAL_HASHES_KEY, proposal_hash);
        if env.storage().persistent().has(&hashes_key) {
            return Err(GovernanceError::ProposalDuplicate);
        }

        // Handle commit-reveal if enabled
        let current_time = env.ledger().timestamp();
        let voting_deadline = current_time + (data.voting_period_days as u64 * 86400);
        let mut reveal_deadline: Option<u64> = None;
        let mut time_lock_expiry: Option<u64> = None;

        if data.commit_reveal_enabled {
            if let Some(commit) = commitment {
                let commit_key = (COMMIT_REVEAL_KEY, data.next_proposal_id);
                let commit_reveal = CommitReveal {
                    commitment: commit,
                    reveal: None,
                    revealed_at: None,
                };
                env.storage().persistent().set(&commit_key, &commit_reveal);
                reveal_deadline = Some(voting_deadline + (data.reveal_period_days as u64 * 86400));
            } else {
                return Err(GovernanceError::InvalidCommitReveal);
            }
        }

        if data.time_lock_enabled {
            time_lock_expiry = Some(current_time + data.time_lock_seconds as u64);
        }

        // Create proposal
        let proposal = Proposal {
            id: data.next_proposal_id,
            proposer: proposer.clone(),
            title: title.clone(),
            description,
            execution_data: execution_data.clone(),
            threshold_percentage,
            deposit_amount,
            created_at: current_time,
            voting_deadline,
            reveal_deadline,
            time_lock_expiry,
            status: if data.commit_reveal_enabled {
                ProposalStatus::Committed
            } else {
                ProposalStatus::Active
            },
            yes_votes: 0,
            no_votes: 0,
            total_voting_power: 0,
        };

        // Store proposal
        let proposal_key = (PROPOSAL_KEY, data.next_proposal_id);
        env.storage().persistent().set(&proposal_key, &proposal);

        // Store proposal hash for uniqueness check
        env.storage()
            .persistent()
            .set(&hashes_key, &data.next_proposal_id);

        // Update proposer stats
        let new_stats = ProposerStats {
            active_proposal_count: proposer_stats.map_or(1, |s| s.active_proposal_count + 1),
            total_proposal_count: proposer_stats.map_or(1, |s| s.total_proposal_count + 1),
            last_proposal_at: current_time,
        };
        env.storage().persistent().set(&proposer_key, &new_stats);

        // Update next proposal ID
        let mut updated_data = data;
        updated_data.next_proposal_id += 1;
        env.storage().instance().set(&DATA_KEY, &updated_data);

        // Transfer deposit from proposer to contract
        // Note: This would require token contract integration
        // For now, we'll assume it's handled externally

        Ok(data.next_proposal_id)
    }

    // Reveal a committed proposal (for commit-reveal mechanism)
    pub fn reveal_proposal(
        env: &Env,
        proposal_id: u64,
        reveal: BytesN<32>,
    ) -> Result<(), GovernanceError> {
        let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();

        if !data.commit_reveal_enabled {
            return Err(GovernanceError::InvalidCommitReveal);
        }

        let proposal_key = (PROPOSAL_KEY, proposal_id);
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .unwrap_or_panic_with(GovernanceError::ProposalNotFound);

        if proposal.status != ProposalStatus::Committed {
            return Err(GovernanceError::InvalidCommitReveal);
        }

        let current_time = env.ledger().timestamp();
        if current_time < proposal.voting_deadline {
            return Err(GovernanceError::VotingPeriodNotEnded);
        }

        if let Some(reveal_deadline) = proposal.reveal_deadline {
            if current_time > reveal_deadline {
                return Err(GovernanceError::RevealPeriodEnded);
            }
        }

        let commit_key = (COMMIT_REVEAL_KEY, proposal_id);
        let mut commit_reveal: CommitReveal = env
            .storage()
            .persistent()
            .get(&commit_key)
            .unwrap_or_panic_with(GovernanceError::InvalidCommitReveal);

        // Verify the reveal matches the commitment
        let expected_commit = Self::hash_bytes(&reveal);
        if expected_commit != commit_reveal.commitment {
            return Err(GovernanceError::InvalidCommitReveal);
        }

        // Update commit-reveal record
        commit_reveal.reveal = Some(reveal);
        commit_reveal.revealed_at = Some(current_time);
        env.storage().persistent().set(&commit_key, &commit_reveal);

        // Activate proposal
        proposal.status = ProposalStatus::Active;
        env.storage().persistent().set(&proposal_key, &proposal);

        Ok(())
    }

    // Vote on a proposal
    pub fn vote(
        env: &Env,
        voter: Address,
        proposal_id: u64,
        vote_weight: i128,
        is_yes: bool,
    ) -> Result<(), GovernanceError> {
        // Check if contract is paused
        if Self::is_paused(env) {
            return Err(GovernanceError::ContractAlreadyPaused);
        }

        let proposal_key = (PROPOSAL_KEY, proposal_id);
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .unwrap_or_panic_with(GovernanceError::ProposalNotFound);

        if proposal.status != ProposalStatus::Active {
            return Err(GovernanceError::VotingPeriodEnded);
        }

        let current_time = env.ledger().timestamp();
        if current_time > proposal.voting_deadline {
            proposal.status = ProposalStatus::Rejected;
            env.storage().persistent().set(&proposal_key, &proposal);
            return Err(GovernanceError::VotingPeriodEnded);
        }

        // Check if already voted
        let vote_key = (VOTE_KEY, proposal_id, voter.clone());
        if env.storage().persistent().has(&vote_key) {
            return Err(GovernanceError::AlreadyVoted);
        }

        // Record vote
        let vote = Vote {
            voter: voter.clone(),
            weight: vote_weight,
            is_yes,
            timestamp: current_time,
        };
        env.storage().persistent().set(&vote_key, &vote);

        // Update proposal vote counts
        if is_yes {
            proposal.yes_votes += vote_weight;
        } else {
            proposal.no_votes += vote_weight;
        }
        proposal.total_voting_power += vote_weight;
        env.storage().persistent().set(&proposal_key, &proposal);

        Ok(())
    }

    // Finalize a proposal
    pub fn finalize_proposal(env: &Env, proposal_id: u64) -> Result<(), GovernanceError> {
        let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();
        let proposal_key = (PROPOSAL_KEY, proposal_id);
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .unwrap_or_panic_with(GovernanceError::ProposalNotFound);

        if proposal.status == ProposalStatus::Executed {
            return Err(GovernanceError::ProposalNotFound);
        }

        let current_time = env.ledger().timestamp();
        if current_time <= proposal.voting_deadline {
            return Err(GovernanceError::VotingPeriodNotEnded);
        }

        // Check if this is a pause/unpause proposal by checking execution data
        let is_pause_proposal = Self::is_pause_proposal(&proposal.execution_data);
        let is_unpause_proposal = Self::is_unpause_proposal(&proposal.execution_data);

        // Use appropriate quorum for pause/unpause proposals
        let quorum_percentage = if is_pause_proposal || is_unpause_proposal {
            data.pause_quorum_percentage
        } else {
            data.min_quorum_percentage
        };

        // Check quorum
        let total_supply = 1000000i128; // This should be fetched from token contract
        let quorum_threshold = (total_supply * quorum_percentage as i128) / 100;
        if proposal.total_voting_power < quorum_threshold {
            proposal.status = ProposalStatus::Rejected;
            env.storage().persistent().set(&proposal_key, &proposal);
            return Err(GovernanceError::QuorumNotMet);
        }

        // Check threshold
        let yes_percentage = (proposal.yes_votes * 100) / proposal.total_voting_power;
        if yes_percentage >= proposal.threshold_percentage as i128 {
            proposal.status = ProposalStatus::Passed;
        } else {
            proposal.status = ProposalStatus::Rejected;
        }

        // Decrement active proposal count for proposer
        let proposer_key = (PROPOSER_STATS_KEY, proposal.proposer.clone());
        let mut proposer_stats: ProposerStats = env
            .storage()
            .persistent()
            .get(&proposer_key)
            .unwrap_or(ProposerStats {
                active_proposal_count: 1,
                total_proposal_count: 1,
                last_proposal_at: 0,
            });
        if proposer_stats.active_proposal_count > 0 {
            proposer_stats.active_proposal_count -= 1;
        }
        env.storage()
            .persistent()
            .set(&proposer_key, &proposer_stats);

        env.storage().persistent().set(&proposal_key, &proposal);
        Ok(())
    }

    // Execute a proposal
    pub fn execute_proposal(env: &Env, proposal_id: u64) -> Result<(), GovernanceError> {
        let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();
        let proposal_key = (PROPOSAL_KEY, proposal_id);
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .unwrap_or_panic_with(GovernanceError::ProposalNotFound);

        if proposal.status != ProposalStatus::Passed {
            return Err(GovernanceError::NotAuthorized);
        }

        // Check if contract is paused (but allow pause/unpause proposals to execute)
        let is_pause_proposal = Self::is_pause_proposal(&proposal.execution_data);
        let is_unpause_proposal = Self::is_unpause_proposal(&proposal.execution_data);

        if Self::is_paused(env) && !is_unpause_proposal {
            return Err(GovernanceError::ContractAlreadyPaused);
        }

        // Check time-lock if enabled
        if let Some(time_lock_expiry) = proposal.time_lock_expiry {
            let current_time = env.ledger().timestamp();
            if current_time < time_lock_expiry {
                return Err(GovernanceError::TimeLockNotExpired);
            }
        }

        // Execute the proposal (this would involve calling the execution_data)
        // For now, we'll just mark it as executed
        proposal.status = ProposalStatus::Executed;
        env.storage().persistent().set(&proposal_key, &proposal);

        // Return deposit to proposer
        // Note: This would require token contract integration

        Ok(())
    }

    // Helper function to generate proposal hash
    fn generate_proposal_hash(title: &str, execution_data: &BytesN<32>) -> BytesN<32> {
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        hasher.update(execution_data.as_slice());
        let result = hasher.finalize();

        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&result);
        BytesN::from_array(&hash_bytes)
    }

    // Helper function to hash bytes
    fn hash_bytes(data: &BytesN<32>) -> BytesN<32> {
        let mut hasher = Sha256::new();
        hasher.update(data.as_slice());
        let result = hasher.finalize();

        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&result);
        BytesN::from_array(&hash_bytes)
    }

    // Query functions
    pub fn get_proposal(env: &Env, proposal_id: u64) -> Result<Proposal, GovernanceError> {
        let proposal_key = (PROPOSAL_KEY, proposal_id);
        env.storage()
            .persistent()
            .get(&proposal_key)
            .ok_or(GovernanceError::ProposalNotFound)
    }

    pub fn get_proposer_stats(env: &Env, proposer: Address) -> ProposerStats {
        let proposer_key = (PROPOSER_STATS_KEY, proposer);
        env.storage()
            .persistent()
            .get(&proposer_key)
            .unwrap_or(ProposerStats {
                active_proposal_count: 0,
                total_proposal_count: 0,
                last_proposal_at: 0,
            })
    }

    pub fn get_active_proposals(env: &Env) -> Vec<u64> {
        let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();
        let mut active_proposals = Vec::new(env);

        for i in 1..data.next_proposal_id {
            if let Ok(proposal) = Self::get_proposal(env, i) {
                if proposal.status == ProposalStatus::Active {
                    active_proposals.push_back(i);
                }
            }
        }

        active_proposals
    }

    // Clean up expired proposals (admin only)
    pub fn cleanup_expired_proposals(env: &Env, admin: Address) -> Result<u64, GovernanceError> {
        let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();

        if admin != data.admin {
            return Err(GovernanceError::NotAuthorized);
        }

        let current_time = env.ledger().timestamp();
        let mut cleaned_count = 0;

        for i in 1..data.next_proposal_id {
            if let Ok(mut proposal) = Self::get_proposal(env, i) {
                let should_cleanup = match proposal.status {
                    ProposalStatus::Active => {
                        current_time > proposal.voting_deadline + (30 * 86400)
                    } // 30 days after voting deadline
                    ProposalStatus::Committed => {
                        current_time
                            > proposal.reveal_deadline.unwrap_or(proposal.voting_deadline)
                                + (30 * 86400)
                    }
                    ProposalStatus::Passed | ProposalStatus::Rejected => {
                        current_time > proposal.voting_deadline + (90 * 86400)
                    } // 90 days after voting deadline
                    ProposalStatus::Executed => false, // Keep executed proposals
                };

                if should_cleanup {
                    proposal.status = ProposalStatus::Rejected; // Mark as rejected for cleanup
                    let proposal_key = (PROPOSAL_KEY, i);
                    env.storage().persistent().set(&proposal_key, &proposal);
                    cleaned_count += 1;
                }
            }
        }

        Ok(cleaned_count)
    }

    // Get proposal by hash (for duplicate checking)
    pub fn get_proposal_by_hash(
        env: &Env,
        title: String,
        execution_data: BytesN<32>,
    ) -> Option<u64> {
        let proposal_hash = Self::generate_proposal_hash(&title, &execution_data);
        let hashes_key = (PROPOSAL_HASHES_KEY, proposal_hash);
        env.storage().persistent().get(&hashes_key)
    }

    // =====================================================================
    // EMERGENCY PAUSE/UNPAUSE GOVERNANCE FUNCTIONS
    // =====================================================================

    // Create a pause proposal (requires higher quorum and threshold)
    pub fn create_pause_proposal(
        env: &Env,
        proposer: Address,
        title: String,
        description: String,
        reason: String,
    ) -> Result<u64, GovernanceError> {
        // Check if multi-sig is enabled - if so, use multi-sig instead
        if Self::is_multi_sig_enabled(env) {
            // Create multi-sig pause proposal instead
            let operation_data = Map::from_array(env, &[(Symbol::short("reason"), reason.clone())]);
            return Self::create_multi_sig_proposal(
                env,
                proposer,
                OperationType::Pause,
                operation_data,
                None,
                604800, // 7 days expiry
            );
        }

        let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();

        // Check minimum deposit
        if data.min_proposal_deposit > 0 {
            // For pause proposals, we might require higher deposit
            let pause_deposit = data.min_proposal_deposit * 2; // Double deposit for emergency proposals
                                                               // Note: In a real implementation, you'd check the token balance here
        }

        // Create pause-specific execution data
        let execution_data = Self::generate_pause_execution_data(&reason);

        // Create proposal with pause-specific threshold
        Self::create_proposal(
            env,
            proposer,
            title,
            description,
            execution_data,
            data.pause_threshold_percentage,
            data.min_proposal_deposit * 2, // Higher deposit for emergency proposals
            None,                          // No commitment for pause proposals
        )
    }

    // Create an unpause proposal
    pub fn create_unpause_proposal(
        env: &Env,
        proposer: Address,
        title: String,
        description: String,
        reason: String,
    ) -> Result<u64, GovernanceError> {
        // Check if multi-sig is enabled - if so, use multi-sig instead
        if Self::is_multi_sig_enabled(env) {
            // Create multi-sig unpause proposal instead
            let operation_data = Map::from_array(env, &[(Symbol::short("reason"), reason.clone())]);
            return Self::create_multi_sig_proposal(
                env,
                proposer,
                OperationType::Unpause,
                operation_data,
                None,
                604800, // 7 days expiry
            );
        }

        let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();

        // Check minimum deposit
        if data.min_proposal_deposit > 0 {
            let pause_deposit = data.min_proposal_deposit * 2;
            // Note: In a real implementation, you'd check the token balance here
        }

        // Create unpause-specific execution data
        let execution_data = Self::generate_unpause_execution_data(&reason);

        // Create proposal with pause-specific threshold
        Self::create_proposal(
            env,
            proposer,
            title,
            description,
            execution_data,
            data.pause_threshold_percentage,
            data.min_proposal_deposit * 2,
            None,
        )
    }

    // Execute pause action (only callable through successful governance proposal)
    pub fn execute_pause_action(
        env: &Env,
        proposal_id: u64,
        reason: String,
    ) -> Result<(), GovernanceError> {
        // If multi-sig is enabled, this should be handled through multi-sig
        if Self::is_multi_sig_enabled(env) {
            return Err(GovernanceError::NotAuthorized);
        }

        let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();
        let proposal_key = (PROPOSAL_KEY, proposal_id);
        let proposal: Proposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .unwrap_or_panic_with(GovernanceError::ProposalNotFound);

        // Verify proposal is passed
        if proposal.status != ProposalStatus::Passed {
            return Err(GovernanceError::NotAuthorized);
        }

        // Check if already paused
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

        if pause_state.is_paused {
            return Err(GovernanceError::ContractAlreadyPaused);
        }

        // Execute pause
        let current_time = env.ledger().timestamp();
        let new_pause_state = PauseState {
            is_paused: true,
            paused_at: Some(current_time),
            paused_by: Some(proposal.proposer.clone()),
            pause_reason: Some(reason.clone()),
        };

        env.storage()
            .instance()
            .set(&PAUSE_STATE_KEY, &new_pause_state);

        // Add to pause history
        Self::add_pause_history_entry(
            env,
            PauseAction::Pause,
            proposal.proposer.clone(),
            Some(reason),
            Some(proposal_id),
        );

        // Mark proposal as executed
        let mut updated_proposal = proposal;
        updated_proposal.status = ProposalStatus::Executed;
        env.storage()
            .persistent()
            .set(&proposal_key, &updated_proposal);

        Ok(())
    }

    // Execute unpause action (only callable through successful governance proposal)
    pub fn execute_unpause_action(
        env: &Env,
        proposal_id: u64,
        reason: String,
    ) -> Result<(), GovernanceError> {
        // If multi-sig is enabled, this should be handled through multi-sig
        if Self::is_multi_sig_enabled(env) {
            return Err(GovernanceError::NotAuthorized);
        }

        let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap_or_panic();
        let proposal_key = (PROPOSAL_KEY, proposal_id);
        let proposal: Proposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .unwrap_or_panic_with(GovernanceError::ProposalNotFound);

        // Verify proposal is passed
        if proposal.status != ProposalStatus::Passed {
            return Err(GovernanceError::NotAuthorized);
        }

        // Check if not paused
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

        if !pause_state.is_paused {
            return Err(GovernanceError::ContractNotPaused);
        }

        // Execute unpause
        let new_pause_state = PauseState {
            is_paused: false,
            paused_at: None,
            paused_by: None,
            pause_reason: None,
        };

        env.storage()
            .instance()
            .set(&PAUSE_STATE_KEY, &new_pause_state);

        // Add to pause history
        Self::add_pause_history_entry(
            env,
            PauseAction::Unpause,
            proposal.proposer.clone(),
            Some(reason),
            Some(proposal_id),
        );

        // Mark proposal as executed
        let mut updated_proposal = proposal;
        updated_proposal.status = ProposalStatus::Executed;
        env.storage()
            .persistent()
            .set(&proposal_key, &updated_proposal);

        Ok(())
    }

    // Check if contract is paused (guard function)
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

    // Get current pause status
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

    // Get pause history
    pub fn get_pause_history(env: &Env) -> Vec<PauseHistoryEntry> {
        env.storage()
            .persistent()
            .get(&PAUSE_HISTORY_KEY)
            .unwrap_or(Vec::new(env))
    }

    // =====================================================================
    // INTERNAL HELPER FUNCTIONS
    // =====================================================================

    // Generate pause execution data
    fn generate_pause_execution_data(reason: &str) -> BytesN<32> {
        let mut hasher = Sha256::new();
        hasher.update(b"PAUSE:");
        hasher.update(reason.as_bytes());
        let result = hasher.finalize();

        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&result);
        BytesN::from_array(&hash_bytes)
    }

    // Generate unpause execution data
    fn generate_unpause_execution_data(reason: &str) -> BytesN<32> {
        let mut hasher = Sha256::new();
        hasher.update(b"UNPAUSE:");
        hasher.update(reason.as_bytes());
        let result = hasher.finalize();

        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&result);
        BytesN::from_array(&hash_bytes)
    }

    // Add entry to pause history
    fn add_pause_history_entry(
        env: &Env,
        action: PauseAction,
        actor: Address,
        reason: Option<String>,
        proposal_id: Option<u64>,
    ) {
        let mut history: Vec<PauseHistoryEntry> = env
            .storage()
            .persistent()
            .get(&PAUSE_HISTORY_KEY)
            .unwrap_or(Vec::new(env));

        let entry = PauseHistoryEntry {
            timestamp: env.ledger().timestamp(),
            action,
            actor,
            reason,
            proposal_id,
        };

        history.push_back(entry);
        env.storage().persistent().set(&PAUSE_HISTORY_KEY, &history);
    }

    // Check if execution data represents a pause proposal
    fn is_pause_proposal(execution_data: &BytesN<32>) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(b"PAUSE:");
        let prefix_hash = hasher.finalize();

        // Check if execution_data starts with PAUSE: prefix
        let execution_slice = execution_data.as_slice();
        if execution_slice.len() >= 32 {
            let mut test_hasher = Sha256::new();
            test_hasher.update(b"PAUSE:");
            let result = test_hasher.finalize();

            // Simple check: compare first 16 bytes to detect PAUSE prefix
            for i in 0..16 {
                if execution_slice[i] != result[i] {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    // Check if execution data represents an unpause proposal
    fn is_unpause_proposal(execution_data: &BytesN<32>) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(b"UNPAUSE:");
        let prefix_hash = hasher.finalize();

        // Check if execution_data starts with UNPAUSE: prefix
        let execution_slice = execution_data.as_slice();
        if execution_slice.len() >= 32 {
            let mut test_hasher = Sha256::new();
            test_hasher.update(b"UNPAUSE:");
            let result = test_hasher.finalize();

            // Simple check: compare first 16 bytes to detect UNPAUSE prefix
            for i in 0..16 {
                if execution_slice[i] != result[i] {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }
}
