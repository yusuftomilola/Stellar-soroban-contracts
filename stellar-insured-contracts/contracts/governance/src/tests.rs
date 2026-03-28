use crate::{
    GovernanceContract, GovernanceData, GovernanceError, ProposalStatus, ProposerStats, DATA_KEY,
    PROPOSAL_KEY, PROPOSER_STATS_KEY,
};
use soroban_sdk::{Address, BytesN, Env, Symbol};

#[test]
fn test_initialize() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);

    GovernanceContract::initialize(
        &env,
        admin.clone(),
        token_contract,
        7,     // voting_period_days
        51,    // min_voting_percentage
        10,    // min_quorum_percentage
        100,   // min_proposal_deposit
        5,     // max_proposals_per_proposer
        86400, // proposal_cooldown_seconds
        true,  // commit_reveal_enabled
        1,     // commit_period_days
        1,     // reveal_period_days
        true,  // time_lock_enabled
        3600,  // time_lock_seconds
    );

    let data: GovernanceData = env.storage().instance().get(&DATA_KEY).unwrap();
    assert_eq!(data.admin, admin);
    assert_eq!(data.min_proposal_deposit, 100);
    assert_eq!(data.max_proposals_per_proposer, 5);
}

#[test]
fn test_create_proposal_minimum_deposit() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let proposer = Address::generate(&env);

    // Initialize
    GovernanceContract::initialize(
        &env,
        admin,
        token_contract,
        7,
        51,
        10,
        100,
        5,
        86400,
        false,
        1,
        1,
        false,
        3600,
    );

    // Try to create proposal with insufficient deposit
    let result = GovernanceContract::create_proposal(
        &env,
        proposer,
        "Test Proposal".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[0u8; 32]),
        51,   // threshold_percentage
        50,   // deposit_amount (insufficient)
        None, // commitment
    );

    assert_eq!(result, Err(GovernanceError::InsufficientDeposit));
}

#[test]
fn test_create_proposal_cooldown() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let proposer = Address::generate(&env);

    // Initialize with short cooldown for testing
    GovernanceContract::initialize(
        &env,
        admin,
        token_contract,
        7,
        51,
        10,
        100,
        5,
        10,
        false,
        1,
        1,
        false,
        3600,
    );

    // Create first proposal
    let result1 = GovernanceContract::create_proposal(
        &env,
        proposer.clone(),
        "Test Proposal 1".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[1u8; 32]),
        51,
        100,
        None,
    );
    assert!(result1.is_ok());

    // Try to create second proposal immediately (should fail due to cooldown)
    let result2 = GovernanceContract::create_proposal(
        &env,
        proposer.clone(),
        "Test Proposal 2".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[2u8; 32]),
        51,
        100,
        None,
    );
    assert_eq!(result2, Err(GovernanceError::ProposalTooFrequent));
}

#[test]
fn test_create_proposal_uniqueness() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let proposer1 = Address::generate(&env);
    let proposer2 = Address::generate(&env);

    // Initialize
    GovernanceContract::initialize(
        &env,
        admin,
        token_contract,
        7,
        51,
        10,
        100,
        5,
        0,
        false,
        1,
        1,
        false,
        3600,
    );

    // Create first proposal
    let result1 = GovernanceContract::create_proposal(
        &env,
        proposer1,
        "Test Proposal".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[1u8; 32]),
        51,
        100,
        None,
    );
    assert!(result1.is_ok());

    // Try to create duplicate proposal with different proposer
    let result2 = GovernanceContract::create_proposal(
        &env,
        proposer2,
        "Test Proposal".to_string(), // Same title
        "Test Description".to_string(),
        BytesN::from_array(&[1u8; 32]), // Same execution data
        51,
        100,
        None,
    );
    assert_eq!(result2, Err(GovernanceError::ProposalDuplicate));
}

#[test]
fn test_create_proposal_max_per_proposer() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let proposer = Address::generate(&env);

    // Initialize with max 2 proposals per proposer
    GovernanceContract::initialize(
        &env,
        admin,
        token_contract,
        7,
        51,
        10,
        100,
        2,
        0,
        false,
        1,
        1,
        false,
        3600,
    );

    // Create first proposal
    let result1 = GovernanceContract::create_proposal(
        &env,
        proposer.clone(),
        "Test Proposal 1".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[1u8; 32]),
        51,
        100,
        None,
    );
    assert!(result1.is_ok());

    // Create second proposal
    let result2 = GovernanceContract::create_proposal(
        &env,
        proposer.clone(),
        "Test Proposal 2".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[2u8; 32]),
        51,
        100,
        None,
    );
    assert!(result2.is_ok());

    // Try to create third proposal (should fail)
    let result3 = GovernanceContract::create_proposal(
        &env,
        proposer,
        "Test Proposal 3".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[3u8; 32]),
        51,
        100,
        None,
    );
    assert_eq!(result3, Err(GovernanceError::ProposalTooFrequent));
}

#[test]
fn test_commit_reveal_mechanism() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let proposer = Address::generate(&env);

    // Initialize with commit-reveal enabled
    GovernanceContract::initialize(
        &env,
        admin,
        token_contract,
        7,
        51,
        10,
        100,
        5,
        0,
        true,
        1,
        1,
        false,
        3600,
    );

    // Create proposal with commitment
    let commitment = BytesN::from_array(&[42u8; 32]);
    let result = GovernanceContract::create_proposal(
        &env,
        proposer.clone(),
        "Test Proposal".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[1u8; 32]),
        51,
        100,
        Some(commitment),
    );
    assert!(result.is_ok());
    let proposal_id = result.unwrap();

    // Check proposal is in committed status
    let proposal = GovernanceContract::get_proposal(&env, proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Committed);

    // Try to reveal with wrong data (should fail)
    let wrong_reveal = BytesN::from_array(&[99u8; 32]);
    let reveal_result = GovernanceContract::reveal_proposal(&env, proposal_id, wrong_reveal);
    assert_eq!(reveal_result, Err(GovernanceError::InvalidCommitReveal));

    // Reveal with correct data
    let correct_reveal = BytesN::from_array(&[1u8; 32]);
    let reveal_result = GovernanceContract::reveal_proposal(&env, proposal_id, correct_reveal);
    assert!(reveal_result.is_ok());

    // Check proposal is now active
    let proposal = GovernanceContract::get_proposal(&env, proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Active);
}

#[test]
fn test_time_lock_mechanism() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    // Initialize with time-lock enabled
    GovernanceContract::initialize(
        &env,
        admin,
        token_contract,
        7,
        51,
        10,
        100,
        5,
        0,
        false,
        1,
        1,
        true,
        3600, // 1 hour time-lock
    );

    // Create and pass a proposal
    let result = GovernanceContract::create_proposal(
        &env,
        proposer,
        "Test Proposal".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[1u8; 32]),
        51,
        100,
        None,
    );
    assert!(result.is_ok());
    let proposal_id = result.unwrap();

    // Vote and finalize
    let vote_result = GovernanceContract::vote(&env, voter, proposal_id, 1000, true);
    assert!(vote_result.is_ok());

    // Simulate time passing
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 8 * 86400);

    let finalize_result = GovernanceContract::finalize_proposal(&env, proposal_id);
    assert!(finalize_result.is_ok());

    // Try to execute immediately (should fail due to time-lock)
    let execute_result = GovernanceContract::execute_proposal(&env, proposal_id);
    assert_eq!(execute_result, Err(GovernanceError::TimeLockNotExpired));

    // Simulate time-lock passing
    env.ledger().set_timestamp(env.ledger().timestamp() + 3600);

    // Now execution should work
    let execute_result = GovernanceContract::execute_proposal(&env, proposal_id);
    assert!(execute_result.is_ok());
}

#[test]
fn test_voting_and_finalization() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);

    // Initialize
    GovernanceContract::initialize(
        &env,
        admin,
        token_contract,
        7,
        51,
        10,
        100,
        5,
        0,
        false,
        1,
        1,
        false,
        3600,
    );

    // Create proposal
    let result = GovernanceContract::create_proposal(
        &env,
        proposer,
        "Test Proposal".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[1u8; 32]),
        51,
        100,
        None,
    );
    assert!(result.is_ok());
    let proposal_id = result.unwrap();

    // Vote
    let vote_result1 = GovernanceContract::vote(&env, voter1.clone(), proposal_id, 600, true);
    assert!(vote_result1.is_ok());

    let vote_result2 = GovernanceContract::vote(&env, voter2, proposal_id, 400, false);
    assert!(vote_result2.is_ok());

    // Try to vote again (should fail)
    let duplicate_vote = GovernanceContract::vote(&env, voter1, proposal_id, 100, true);
    assert_eq!(duplicate_vote, Err(GovernanceError::AlreadyVoted));

    // Simulate voting period ending
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 8 * 86400);

    // Finalize
    let finalize_result = GovernanceContract::finalize_proposal(&env, proposal_id);
    assert!(finalize_result.is_ok());

    // Check proposal status
    let proposal = GovernanceContract::get_proposal(&env, proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Passed);
    assert_eq!(proposal.yes_votes, 600);
    assert_eq!(proposal.no_votes, 400);
}

#[test]
fn test_quorum_not_met() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    // Initialize with high quorum requirement
    GovernanceContract::initialize(
        &env,
        admin,
        token_contract,
        7,
        51,
        50,
        100,
        5,
        0,
        false,
        1,
        1,
        false,
        3600, // 50% quorum
    );

    // Create proposal
    let result = GovernanceContract::create_proposal(
        &env,
        proposer,
        "Test Proposal".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[1u8; 32]),
        51,
        100,
        None,
    );
    assert!(result.is_ok());
    let proposal_id = result.unwrap();

    // Vote with insufficient participation
    let vote_result = GovernanceContract::vote(&env, voter, proposal_id, 100, true);
    assert!(vote_result.is_ok());

    // Simulate voting period ending
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 8 * 86400);

    // Try to finalize (should fail due to quorum not met)
    let finalize_result = GovernanceContract::finalize_proposal(&env, proposal_id);
    assert_eq!(finalize_result, Err(GovernanceError::QuorumNotMet));
}

#[test]
fn test_proposer_stats() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let proposer = Address::generate(&env);

    // Initialize
    GovernanceContract::initialize(
        &env,
        admin,
        token_contract,
        7,
        51,
        10,
        100,
        5,
        0,
        false,
        1,
        1,
        false,
        3600,
    );

    // Check initial stats
    let stats = GovernanceContract::get_proposer_stats(&env, proposer.clone());
    assert_eq!(stats.active_proposal_count, 0);
    assert_eq!(stats.total_proposal_count, 0);
    assert_eq!(stats.last_proposal_at, 0);

    // Create proposal
    let result = GovernanceContract::create_proposal(
        &env,
        proposer.clone(),
        "Test Proposal".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[1u8; 32]),
        51,
        100,
        None,
    );
    assert!(result.is_ok());

    // Check updated stats
    let stats = GovernanceContract::get_proposer_stats(&env, proposer);
    assert_eq!(stats.active_proposal_count, 1);
    assert_eq!(stats.total_proposal_count, 1);
    assert!(stats.last_proposal_at > 0);
}

#[test]
fn test_get_active_proposals() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let proposer = Address::generate(&env);

    // Initialize
    GovernanceContract::initialize(
        &env,
        admin,
        token_contract,
        7,
        51,
        10,
        100,
        5,
        0,
        false,
        1,
        1,
        false,
        3600,
    );

    // Create multiple proposals
    let result1 = GovernanceContract::create_proposal(
        &env,
        proposer.clone(),
        "Test Proposal 1".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[1u8; 32]),
        51,
        100,
        None,
    );
    assert!(result1.is_ok());

    let result2 = GovernanceContract::create_proposal(
        &env,
        proposer,
        "Test Proposal 2".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[2u8; 32]),
        51,
        100,
        None,
    );
    assert!(result2.is_ok());

    // Get active proposals
    let active_proposals = GovernanceContract::get_active_proposals(&env);
    assert_eq!(active_proposals.len(), 2);
}

#[test]
fn test_proposal_cleanup() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let proposer = Address::generate(&env);

    // Initialize
    GovernanceContract::initialize(
        &env,
        admin.clone(),
        token_contract,
        7,
        51,
        10,
        100,
        5,
        0,
        false,
        1,
        1,
        false,
        3600,
    );

    // Create proposal
    let result = GovernanceContract::create_proposal(
        &env,
        proposer,
        "Test Proposal".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[1u8; 32]),
        51,
        100,
        None,
    );
    assert!(result.is_ok());

    // Simulate long time passing (more than cleanup threshold)
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 120 * 86400); // 120 days

    // Clean up expired proposals
    let cleanup_result = GovernanceContract::cleanup_expired_proposals(&env, admin);
    assert!(cleanup_result.is_ok());
    assert_eq!(cleanup_result.unwrap(), 1); // Should clean up 1 proposal
}

#[test]
fn test_cleanup_unauthorized() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let unauthorized = Address::generate(&env);

    // Initialize
    GovernanceContract::initialize(
        &env,
        admin,
        token_contract,
        7,
        51,
        10,
        100,
        5,
        0,
        false,
        1,
        1,
        false,
        3600,
    );

    // Try to clean up with unauthorized address
    let cleanup_result = GovernanceContract::cleanup_expired_proposals(&env, unauthorized);
    assert_eq!(cleanup_result, Err(GovernanceError::NotAuthorized));
}

#[test]
fn test_proposal_hash_lookup() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let proposer = Address::generate(&env);

    // Initialize
    GovernanceContract::initialize(
        &env,
        admin,
        token_contract,
        7,
        51,
        10,
        100,
        5,
        0,
        false,
        1,
        1,
        false,
        3600,
    );

    // Create proposal
    let result = GovernanceContract::create_proposal(
        &env,
        proposer,
        "Test Proposal".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[1u8; 32]),
        51,
        100,
        None,
    );
    assert!(result.is_ok());
    let proposal_id = result.unwrap();

    // Look up proposal by hash
    let found_id = GovernanceContract::get_proposal_by_hash(
        &env,
        "Test Proposal".to_string(),
        BytesN::from_array(&[1u8; 32]),
    );
    assert_eq!(found_id, Some(proposal_id));

    // Try non-existent proposal
    let not_found = GovernanceContract::get_proposal_by_hash(
        &env,
        "Non-existent".to_string(),
        BytesN::from_array(&[2u8; 32]),
    );
    assert_eq!(not_found, None);
}

#[test]
fn test_active_proposal_count_decrement() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    // Initialize
    GovernanceContract::initialize(
        &env,
        admin,
        token_contract,
        7,
        51,
        10,
        100,
        5,
        0,
        false,
        1,
        1,
        false,
        3600,
    );

    // Create proposal
    let result = GovernanceContract::create_proposal(
        &env,
        proposer.clone(),
        "Test Proposal".to_string(),
        "Test Description".to_string(),
        BytesN::from_array(&[1u8; 32]),
        51,
        100,
        None,
    );
    assert!(result.is_ok());
    let proposal_id = result.unwrap();

    // Check active count is 1
    let stats = GovernanceContract::get_proposer_stats(&env, proposer.clone());
    assert_eq!(stats.active_proposal_count, 1);

    // Vote and finalize
    let vote_result = GovernanceContract::vote(&env, voter, proposal_id, 1000, true);
    assert!(vote_result.is_ok());

    // Simulate voting period ending
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 8 * 86400);

    // Finalize (should decrement active count)
    let finalize_result = GovernanceContract::finalize_proposal(&env, proposal_id);
    assert!(finalize_result.is_ok());

    // Check active count is back to 0
    let stats = GovernanceContract::get_proposer_stats(&env, proposer);
    assert_eq!(stats.active_proposal_count, 0);
    assert_eq!(stats.total_proposal_count, 1); // Total should remain
}

// =====================================================================
// MULTI-SIGNATURE TESTS
// =====================================================================

#[test]
fn test_configure_multi_sig() {
    use crate::{MultiSigConfig, SignerInfo};

    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);

    // Initialize governance contract
    GovernanceContract::initialize(
        &env,
        admin.clone(),
        token_contract,
        7,
        51,
        10,
        100,
        5,
        86400,
        false,
        1,
        1,
        false,
        3600,
        60,
        67, // pause quorum and threshold
    );

    // Create signer list
    let signers = Vec::from_array(
        &env,
        [
            SignerInfo {
                address: signer1.clone(),
                weight: 1,
                active: true,
            },
            SignerInfo {
                address: signer2.clone(),
                weight: 1,
                active: true,
            },
            SignerInfo {
                address: signer3.clone(),
                weight: 1,
                active: true,
            },
        ],
    );

    // Configure multi-sig with threshold of 2 out of 3
    let result = GovernanceContract::configure_multi_sig(&env, admin.clone(), signers, 2);
    assert!(result.is_ok());

    // Verify configuration
    let config = GovernanceContract::get_multi_sig_config(&env).unwrap();
    assert_eq!(config.signers.len(), 3);
    assert_eq!(config.threshold_weight, 2);
    assert_eq!(config.enabled, true);
    assert_eq!(config.configured_by, admin);
}

#[test]
fn test_multi_sig_invalid_threshold() {
    use crate::SignerInfo;

    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);

    // Initialize governance contract
    GovernanceContract::initialize(
        &env,
        admin.clone(),
        token_contract,
        7,
        51,
        10,
        100,
        5,
        86400,
        false,
        1,
        1,
        false,
        3600,
        60,
        67,
    );

    // Create signer list with total weight of 2
    let signers = Vec::from_array(
        &env,
        [
            SignerInfo {
                address: signer1.clone(),
                weight: 1,
                active: true,
            },
            SignerInfo {
                address: signer2.clone(),
                weight: 1,
                active: true,
            },
        ],
    );

    // Try to configure with invalid threshold (greater than total weight)
    let result = std::panic::catch_unwind(|| {
        GovernanceContract::configure_multi_sig(&env, admin.clone(), signers, 5)
    });
    assert!(result.is_err()); // Should panic
}

#[test]
fn test_create_multi_sig_proposal() {
    use crate::{OperationType, SignerInfo};

    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);

    // Initialize and configure multi-sig
    GovernanceContract::initialize(
        &env,
        admin.clone(),
        token_contract,
        7,
        51,
        10,
        100,
        5,
        86400,
        false,
        1,
        1,
        false,
        3600,
        60,
        67,
    );

    let signers = Vec::from_array(
        &env,
        [
            SignerInfo {
                address: signer1.clone(),
                weight: 1,
                active: true,
            },
            SignerInfo {
                address: signer2.clone(),
                weight: 1,
                active: true,
            },
            SignerInfo {
                address: signer3.clone(),
                weight: 1,
                active: true,
            },
        ],
    );
    GovernanceContract::configure_multi_sig(&env, admin.clone(), signers, 2).unwrap();

    // Create multi-sig pause proposal
    let operation_data = Map::from_array(
        &env,
        [(
            Symbol::short("reason"),
            String::from_str(&env, "Security concern"),
        )],
    );

    let result = GovernanceContract::create_multi_sig_proposal(
        &env,
        signer1.clone(),
        OperationType::Pause,
        operation_data,
        None,
        604800, // 7 days expiry
    );
    assert!(result.is_ok());
    let proposal_id = result.unwrap();

    // Verify proposal was created
    let proposal = GovernanceContract::get_multi_sig_proposal(&env, proposal_id).unwrap();
    assert_eq!(proposal.operation_type, OperationType::Pause);
    assert_eq!(proposal.created_by, signer1);
    assert_eq!(proposal.confirmed_weight, 1); // Auto-confirmed by creator
}

#[test]
fn test_multi_sig_confirmation() {
    use crate::{OperationType, SignerInfo};

    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);

    // Initialize and configure multi-sig
    GovernanceContract::initialize(
        &env,
        admin.clone(),
        token_contract,
        7,
        51,
        10,
        100,
        5,
        86400,
        false,
        1,
        1,
        false,
        3600,
        60,
        67,
    );

    let signers = Vec::from_array(
        &env,
        [
            SignerInfo {
                address: signer1.clone(),
                weight: 1,
                active: true,
            },
            SignerInfo {
                address: signer2.clone(),
                weight: 1,
                active: true,
            },
            SignerInfo {
                address: signer3.clone(),
                weight: 1,
                active: true,
            },
        ],
    );
    GovernanceContract::configure_multi_sig(&env, admin.clone(), signers, 2).unwrap();

    // Create proposal
    let operation_data = Map::from_array(
        &env,
        [(
            Symbol::short("reason"),
            String::from_str(&env, "Security concern"),
        )],
    );
    let proposal_id = GovernanceContract::create_multi_sig_proposal(
        &env,
        signer1.clone(),
        OperationType::Pause,
        operation_data,
        None,
        604800,
    )
    .unwrap();

    // Second signer confirms
    let confirm_result =
        GovernanceContract::confirm_multi_sig_proposal(&env, signer2.clone(), proposal_id);
    assert!(confirm_result.is_ok());

    // Verify confirmation increased weight
    let proposal = GovernanceContract::get_multi_sig_proposal(&env, proposal_id).unwrap();
    assert_eq!(proposal.confirmed_weight, 2); // Now meets threshold

    // Try to confirm again (should fail)
    let duplicate_confirm =
        GovernanceContract::confirm_multi_sig_proposal(&env, signer2.clone(), proposal_id);
    assert_eq!(
        duplicate_confirm,
        Err(GovernanceError::MultiSigAlreadyConfirmed)
    );
}

#[test]
fn test_multi_sig_execution() {
    use crate::{OperationType, SignerInfo};

    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);

    // Initialize and configure multi-sig
    GovernanceContract::initialize(
        &env,
        admin.clone(),
        token_contract,
        7,
        51,
        10,
        100,
        5,
        86400,
        false,
        1,
        1,
        false,
        3600,
        60,
        67,
    );

    let signers = Vec::from_array(
        &env,
        [
            SignerInfo {
                address: signer1.clone(),
                weight: 1,
                active: true,
            },
            SignerInfo {
                address: signer2.clone(),
                weight: 1,
                active: true,
            },
            SignerInfo {
                address: signer3.clone(),
                weight: 1,
                active: true,
            },
        ],
    );
    GovernanceContract::configure_multi_sig(&env, admin.clone(), signers, 2).unwrap();

    // Create pause proposal
    let operation_data = Map::from_array(
        &env,
        [(
            Symbol::short("reason"),
            String::from_str(&env, "Security concern"),
        )],
    );
    let proposal_id = GovernanceContract::create_multi_sig_proposal(
        &env,
        signer1.clone(),
        OperationType::Pause,
        operation_data,
        None,
        604800,
    )
    .unwrap();

    // Second signer confirms
    GovernanceContract::confirm_multi_sig_proposal(&env, signer2.clone(), proposal_id).unwrap();

    // Execute the proposal
    let execute_result = GovernanceContract::execute_multi_sig_proposal(
        &env,
        signer3.clone(), // Any address can execute once threshold met
        proposal_id,
    );
    assert!(execute_result.is_ok());

    // Verify contract is paused
    assert!(GovernanceContract::is_paused(&env));

    // Try to execute again (should fail)
    let duplicate_execute =
        GovernanceContract::execute_multi_sig_proposal(&env, signer1, proposal_id);
    assert_eq!(
        duplicate_execute,
        Err(GovernanceError::MultiSigProposalAlreadyExecuted)
    );
}

#[test]
fn test_multi_sig_unpause() {
    use crate::{OperationType, SignerInfo};

    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);

    // Initialize and configure multi-sig
    GovernanceContract::initialize(
        &env,
        admin.clone(),
        token_contract,
        7,
        51,
        10,
        100,
        5,
        86400,
        false,
        1,
        1,
        false,
        3600,
        60,
        67,
    );

    let signers = Vec::from_array(
        &env,
        [
            SignerInfo {
                address: signer1.clone(),
                weight: 1,
                active: true,
            },
            SignerInfo {
                address: signer2.clone(),
                weight: 1,
                active: true,
            },
        ],
    );
    GovernanceContract::configure_multi_sig(&env, admin.clone(), signers, 2).unwrap();

    // First pause the contract
    let pause_data = Map::from_array(
        &env,
        [(
            Symbol::short("reason"),
            String::from_str(&env, "Security concern"),
        )],
    );
    let pause_proposal_id = GovernanceContract::create_multi_sig_proposal(
        &env,
        signer1.clone(),
        OperationType::Pause,
        pause_data,
        None,
        604800,
    )
    .unwrap();
    GovernanceContract::confirm_multi_sig_proposal(&env, signer2.clone(), pause_proposal_id)
        .unwrap();
    GovernanceContract::execute_multi_sig_proposal(&env, signer1.clone(), pause_proposal_id)
        .unwrap();

    // Verify contract is paused
    assert!(GovernanceContract::is_paused(&env));

    // Create unpause proposal
    let unpause_data = Map::from_array(
        &env,
        [(
            Symbol::short("reason"),
            String::from_str(&env, "Issue resolved"),
        )],
    );
    let unpause_proposal_id = GovernanceContract::create_multi_sig_proposal(
        &env,
        signer1.clone(),
        OperationType::Unpause,
        unpause_data,
        None,
        604800,
    )
    .unwrap();

    // Confirm and execute unpause
    GovernanceContract::confirm_multi_sig_proposal(&env, signer2.clone(), unpause_proposal_id)
        .unwrap();
    let execute_result =
        GovernanceContract::execute_multi_sig_proposal(&env, signer1.clone(), unpause_proposal_id);
    assert!(execute_result.is_ok());

    // Verify contract is unpaused
    assert!(!GovernanceContract::is_paused(&env));
}

#[test]
fn test_multi_sig_invalid_signer() {
    use crate::{OperationType, SignerInfo};

    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let non_signer = Address::generate(&env);

    // Initialize and configure multi-sig
    GovernanceContract::initialize(
        &env,
        admin.clone(),
        token_contract,
        7,
        51,
        10,
        100,
        5,
        86400,
        false,
        1,
        1,
        false,
        3600,
        60,
        67,
    );

    let signers = Vec::from_array(
        &env,
        [
            SignerInfo {
                address: signer1.clone(),
                weight: 1,
                active: true,
            },
            SignerInfo {
                address: signer2.clone(),
                weight: 1,
                active: true,
            },
        ],
    );
    GovernanceContract::configure_multi_sig(&env, admin.clone(), signers, 2).unwrap();

    // Try to create proposal from non-signer (should fail)
    let operation_data = Map::from_array(
        &env,
        [(Symbol::short("reason"), String::from_str(&env, "Test"))],
    );
    let result = GovernanceContract::create_multi_sig_proposal(
        &env,
        non_signer.clone(),
        OperationType::Pause,
        operation_data,
        None,
        604800,
    );
    assert_eq!(result, Err(GovernanceError::MultiSigInvalidSigner));

    // Try to confirm as non-signer (should fail)
    let proposal_id = GovernanceContract::create_multi_sig_proposal(
        &env,
        signer1.clone(),
        OperationType::Pause,
        operation_data,
        None,
        604800,
    )
    .unwrap();

    let confirm_result =
        GovernanceContract::confirm_multi_sig_proposal(&env, non_signer, proposal_id);
    assert_eq!(confirm_result, Err(GovernanceError::MultiSigInvalidSigner));
}

#[test]
fn test_multi_sig_add_remove_signer() {
    use crate::SignerInfo;

    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let new_signer = Address::generate(&env);

    // Initialize and configure multi-sig
    GovernanceContract::initialize(
        &env,
        admin.clone(),
        token_contract,
        7,
        51,
        10,
        100,
        5,
        86400,
        false,
        1,
        1,
        false,
        3600,
        60,
        67,
    );

    let signers = Vec::from_array(
        &env,
        [
            SignerInfo {
                address: signer1.clone(),
                weight: 1,
                active: true,
            },
            SignerInfo {
                address: signer2.clone(),
                weight: 1,
                active: true,
            },
        ],
    );
    GovernanceContract::configure_multi_sig(&env, admin.clone(), signers, 2).unwrap();

    // Add new signer (requires existing signer approval)
    let add_result = GovernanceContract::add_signer(&env, signer1.clone(), new_signer.clone(), 1);
    assert!(add_result.is_ok());

    // Verify signer was added
    let config = GovernanceContract::get_multi_sig_config(&env).unwrap();
    assert_eq!(config.signers.len(), 3);

    // Remove signer (requires existing signer approval)
    let remove_result =
        GovernanceContract::remove_signer(&env, signer2.clone(), new_signer.clone());
    assert!(remove_result.is_ok());

    // Verify signer was removed
    let config = GovernanceContract::get_multi_sig_config(&env).unwrap();
    assert_eq!(config.signers.len(), 2);
}

#[test]
fn test_is_signer_and_get_weight() {
    use crate::SignerInfo;

    let env = Env::default();
    let admin = Address::generate(&env);
    let token_contract = Address::generate(&env);
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);

    // Initialize and configure multi-sig
    GovernanceContract::initialize(
        &env,
        admin.clone(),
        token_contract,
        7,
        51,
        10,
        100,
        5,
        86400,
        false,
        1,
        1,
        false,
        3600,
        60,
        67,
    );

    let signers = Vec::from_array(
        &env,
        [
            SignerInfo {
                address: signer1.clone(),
                weight: 2,
                active: true,
            },
            SignerInfo {
                address: signer2.clone(),
                weight: 1,
                active: true,
            },
        ],
    );
    GovernanceContract::configure_multi_sig(&env, admin.clone(), signers, 2).unwrap();

    let config = GovernanceContract::get_multi_sig_config(&env).unwrap();

    // Test is_signer
    assert!(GovernanceContract::is_signer(&env, &signer1, &config));
    assert!(GovernanceContract::is_signer(&env, &signer2, &config));

    let non_signer = Address::generate(&env);
    assert!(!GovernanceContract::is_signer(&env, &non_signer, &config));

    // Test get_signer_weight
    assert_eq!(
        GovernanceContract::get_signer_weight(&env, &signer1, &config),
        2
    );
    assert_eq!(
        GovernanceContract::get_signer_weight(&env, &signer2, &config),
        1
    );
    assert_eq!(
        GovernanceContract::get_signer_weight(&env, &non_signer, &config),
        0
    );
}
