# Governance Contract with Spam Protection and Multi-Signature Support

A robust DAO governance contract for Stellar/Soroban that implements comprehensive spam protection, anti-front-running mechanisms, and multi-signature support for critical operations.

## Features

### 🛡️ Security Features
- **Multi-Signature Support**: Critical operations require multiple authorized signers
- **Configurable Thresholds**: Flexible weight-based approval system
- **Emergency Pause/Unpause**: Multi-sig protected emergency controls
- **Parameter Updates**: Governance parameters protected by multi-sig
- **Signer Management**: Add/remove signers through multi-sig approval
- **Minimum Deposit Requirement**: Proposals must meet a minimum deposit threshold to prevent spam
- **Proposal Cooldown**: Time-based cooldown between proposals from the same proposer
- **Maximum Active Proposals**: Limit on concurrent active proposals per proposer
- **Proposal Uniqueness**: Hash-based deduplication prevents duplicate proposals

### 🔒 Anti-Front-Running
- **Commit-Reveal Mechanism**: Optional two-step proposal submission to prevent front-running
- **Time-Lock Protection**: Configurable delay before proposal execution
- **Timestamp-Based Ordering**: Fair proposal ordering based on creation time

### 🔒 Multi-Signature Operations

The following critical operations now require multi-signature approval when multi-sig is enabled:

1. **Pause/Unpause**: Emergency contract pause and unpause operations
2. **Parameter Updates**: Changing voting thresholds, deposits, quorum requirements
3. **Emergency Shutdown**: Complete system shutdown in emergencies
4. **Contract Upgrades**: Upgrading contract code
5. **Treasury Withdrawals**: Moving funds from treasury
6. **Signer Management**: Adding or removing multi-sig signers
7. **Threshold Updates**: Changing multi-sig threshold requirements

#### Multi-Sig Configuration

```rust
// Configure multi-sig with 3 signers and threshold of 2
let signers = Vec::from_array(&env, [
    SignerInfo { address: signer1, weight: 1, active: true },
    SignerInfo { address: signer2, weight: 1, active: true },
    SignerInfo { address: signer3, weight: 1, active: true },
]);

GovernanceContract::configure_multi_sig(
    &env,
    admin,
    signers,
    2, // Threshold: 2 out of 3 signers required
);
```

#### Creating Multi-Sig Proposals

```rust
// Create a pause proposal
let operation_data = Map::from_array(&env, [
    (Symbol::short("reason"), String::from_str(&env, "Security concern")),
]);

let proposal_id = GovernanceContract::create_multi_sig_proposal(
    &env,
    signer1,
    OperationType::Pause,
    operation_data,
    None,
    604800, // 7 days expiry
)?;
```

#### Confirming Multi-Sig Proposals

```rust
// Second signer confirms the proposal
GovernanceContract::confirm_multi_sig_proposal(
    &env,
    signer2,
    proposal_id,
)?;
```

#### Executing Multi-Sig Proposals

```rust
// Once threshold is met, anyone can execute
GovernanceContract::execute_multi_sig_proposal(
    &env,
    executor,
    proposal_id,
)?;
```

### 📊 Governance Features
- **Voting System**: Weighted voting with configurable thresholds
- **Quorum Requirements**: Minimum participation requirements for proposal validity
- **Proposal Lifecycle**: Complete proposal state management (Active → Passed/Rejected → Executed)
- **Deposit Management**: Security deposits returned upon successful execution

### 🧹 Maintenance
- **Automatic Cleanup**: Admin function to clean up expired proposals
- **Statistics Tracking**: Per-proposer statistics for active and total proposals
- **Query Functions**: Rich set of query functions for governance data

## Contract Structure

### Core Components

#### `GovernanceData`
Configuration parameters for the governance system:
- Admin address
- Token contract reference
- Voting periods and thresholds
- Spam protection settings
- Feature toggles (commit-reveal, time-lock)

#### `Proposal`
Complete proposal information:
- Metadata (title, description, proposer)
- Execution data and voting parameters
- Timestamps and deadlines
- Status tracking and vote counts

#### `ProposerStats`
Per-proposer statistics:
- Active proposal count
- Total proposal count
- Last proposal timestamp

#### `Vote`
Individual vote records:
- Voter address and weight
- Vote direction (yes/no)
- Timestamp

### Error Types

#### Standard Governance Errors
- `InsufficientDeposit`: Proposal deposit below minimum
- `ProposalTooFrequent`: Cooldown period not met or max proposals exceeded
- `ProposalDuplicate`: Duplicate proposal detected
- `ProposalNotFound`: Proposal ID invalid
- `NotAuthorized`: Unauthorized action attempted
- `VotingPeriodNotEnded`: Voting still active
- `VotingPeriodEnded`: Voting period expired
- `AlreadyVoted`: Duplicate vote attempt
- `QuorumNotMet`: Minimum participation not reached
- `ThresholdNotMet`: Approval threshold not met
- `InvalidCommitReveal`: Commit-reveal validation failed
- `RevealPeriodNotEnded`: Reveal period still active
- `TimeLockNotExpired`: Time-lock period not expired
- `ContractAlreadyPaused`: Contract is already paused
- `ContractNotPaused`: Contract is not paused

#### Multi-Signature Errors
- `MultiSigNotConfigured`: Multi-sig not configured
- `MultiSigThresholdNotMet`: Required confirmation threshold not reached
- `MultiSigAlreadyConfirmed`: Signer already confirmed this proposal
- `MultiSigNotConfirmed`: Signer has not confirmed
- `MultiSigInvalidSigner`: Address is not a valid signer
- `MultiSigProposalAlreadyExecuted`: Proposal already executed

## Usage

### Initialization

```rust
GovernanceContract::initialize(
    &env,
    admin_address,
    token_contract_address,
    voting_period_days,           // 7
    min_voting_percentage,       // 51
    min_quorum_percentage,       // 10
    min_proposal_deposit,        // 100 tokens
    max_proposals_per_proposer,  // 5
    proposal_cooldown_seconds,   // 86400 (1 day)
    commit_reveal_enabled,       // true
    commit_period_days,          // 1
    reveal_period_days,          // 1
    time_lock_enabled,           // true
    time_lock_seconds,           // 3600 (1 hour)
    pause_quorum_percentage,     // 60
    pause_threshold_percentage,  // 67
);
```

### Multi-Signature Setup

#### Step 1: Configure Multi-Sig

```rust
use propchain_governance::{SignerInfo, MultiSigConfig};

// Define signers with weights
let signers = Vec::from_array(&env, [
    SignerInfo {
        address: signer1_address,
        weight: 2,  // Higher weight for trusted signer
        active: true,
    },
    SignerInfo {
        address: signer2_address,
        weight: 1,
        active: true,
    },
    SignerInfo {
        address: signer3_address,
        weight: 1,
        active: true,
    },
]);

// Configure with threshold of 3 (e.g., signer1 + any other signer)
GovernanceContract::configure_multi_sig(
    &env,
    admin,
    signers,
    3,  // Threshold weight required
)?;
```

#### Step 2: Verify Configuration

```rust
let config = GovernanceContract::get_multi_sig_config(&env).unwrap();
assert_eq!(config.signers.len(), 3);
assert_eq!(config.threshold_weight, 3);
assert_eq!(config.enabled, true);
```

### Multi-Sig Workflow

#### Scenario 1: Emergency Pause

```rust
// Signer 1 creates pause proposal
let operation_data = Map::from_array(&env, [
    (Symbol::short("reason"), String::from_str(&env, "Critical security vulnerability detected")),
]);

let proposal_id = GovernanceContract::create_multi_sig_proposal(
    &env,
    signer1,
    OperationType::Pause,
    operation_data,
    None,
    604800,  // 7 days to gather confirmations
)?;

// Signer 2 confirms
GovernanceContract::confirm_multi_sig_proposal(
    &env,
    signer2,
    proposal_id,
)?;

// Check if threshold is met
let proposal = GovernanceContract::get_multi_sig_proposal(&env, proposal_id).unwrap();
if proposal.confirmed_weight >= config.threshold_weight {
    // Execute the pause
    GovernanceContract::execute_multi_sig_proposal(
        &env,
        any_address,  // Anyone can execute once threshold met
        proposal_id,
    )?;
}
```

#### Scenario 2: Parameter Update

```rust
// Update minimum voting percentage from 51% to 60%
let operation_data = Map::from_array(&env, [
    (Symbol::short("value"), String::from_str(&env, "60")),
]);

let proposal_id = GovernanceContract::create_multi_sig_proposal(
    &env,
    signer1,
    OperationType::UpdateParameter(Symbol::short("min_voting_pct")),
    operation_data,
    None,
    604800,
)?;

// Gather required confirmations
GovernanceContract::confirm_multi_sig_proposal(&env, signer2, proposal_id)?;
GovernanceContract::confirm_multi_sig_proposal(&env, signer3, proposal_id)?;

// Execute parameter update
GovernanceContract::execute_multi_sig_proposal(&env, signer1, proposal_id)?;
```

#### Scenario 3: Add New Signer

```rust
// Existing signer adds new signer
let new_signer_address = Address::generate(&env);

GovernanceContract::add_signer(
    &env,
    existing_signer,  // Must be an existing signer
    new_signer_address,
    1,  // Weight for new signer
)?;
```

### Standard Governance Operations

```rust
GovernanceContract::initialize(
    &env,
    admin_address,
    token_contract_address,
    voting_period_days,           // 7
    min_voting_percentage,       // 51
    min_quorum_percentage,       // 10
    min_proposal_deposit,        // 100 tokens
    max_proposals_per_proposer,  // 5
    proposal_cooldown_seconds,   // 86400 (1 day)
    commit_reveal_enabled,       // true
    commit_period_days,          // 1
    reveal_period_days,          // 1
    time_lock_enabled,           // true
    time_lock_seconds,           // 3600 (1 hour)
);
```

### Creating Proposals

#### Standard Proposal
```rust
let proposal_id = GovernanceContract::create_proposal(
    &env,
    proposer_address,
    "Proposal Title".to_string(),
    "Proposal Description".to_string(),
    execution_data,
    threshold_percentage,  // 51
    deposit_amount,       // >= min_proposal_deposit
    None,                // No commitment
)?;
```

#### Commit-Reveal Proposal
```rust
// Step 1: Commit
let commitment = hash_proposal_data(&title, &execution_data);
let proposal_id = GovernanceContract::create_proposal(
    &env,
    proposer_address,
    title,
    description,
    execution_data,
    threshold_percentage,
    deposit_amount,
    Some(commitment),  // Commitment hash
)?;

// Step 2: Reveal (after voting period ends)
GovernanceContract::reveal_proposal(
    &env,
    proposal_id,
    actual_execution_data,
)?;
```

### Voting

```rust
GovernanceContract::vote(
    &env,
    voter_address,
    proposal_id,
    vote_weight,  // Based on token holdings
    is_yes,       // true for yes, false for no
)?;
```

### Finalization and Execution

```rust
// Finalize after voting period
GovernanceContract::finalize_proposal(&env, proposal_id)?;

// Execute after time-lock (if enabled)
GovernanceContract::execute_proposal(&env, proposal_id)?;
```

### Query Functions

```rust
// Get proposal details
let proposal = GovernanceContract::get_proposal(&env, proposal_id)?;

// Get proposer statistics
let stats = GovernanceContract::get_proposer_stats(&env, proposer_address);

// Get all active proposals
let active_proposals = GovernanceContract::get_active_proposals(&env);

// Check for duplicate proposals
let existing_id = GovernanceContract::get_proposal_by_hash(
    &env,
    title.to_string(),
    execution_data,
);
```

### Maintenance

```rust
// Clean up expired proposals (admin only)
let cleaned_count = GovernanceContract::cleanup_expired_proposals(
    &env,
    admin_address,
)?;
```

## Security Considerations

### Multi-Signature Security

#### Signer Selection
1. **Diverse Control**: Choose signers from different organizations/teams
2. **Geographic Distribution**: Avoid single point of failure geographically
3. **Security Practices**: Ensure all signers follow strong security protocols
4. **Regular Rotation**: Periodically review and update signer list
5. **Emergency Contacts**: Maintain backup communication channels

#### Threshold Configuration
- **2-of-3 Minimum**: Recommended minimum for basic security
- **3-of-5 Balanced**: Good balance between security and operational efficiency
- **5-of-7 High Security**: For high-value systems requiring broad consensus
- **Avoid 1-of-N**: Single signer defeats the purpose of multi-sig
- **Consider Weights**: Use weighted voting for differentiated trust levels

#### Operational Security
1. **Monitor Proposals**: Track all multi-sig proposals and confirmations
2. **Expiry Management**: Set reasonable proposal expiry times
3. **Key Management**: Secure private keys with hardware wallets or MPC
4. **Audit Trail**: All multi-sig actions are recorded on-chain
5. **Emergency Procedures**: Have clear procedures for key loss scenarios

### Spam Protection

### Spam Protection
1. **Deposit Requirements**: Minimum deposits prevent spam but should be balanced to avoid excluding legitimate proposers
2. **Cooldown Periods**: Time-based limits prevent rapid-fire proposals
3. **Proposal Limits**: Per-proposer limits ensure fair participation
4. **Hash Deduplication**: Prevents identical proposals from multiple proposers

### Front-Running Protection
1. **Commit-Reveal**: Two-step process hides proposal details until reveal
2. **Time-Locks**: Delays execution to allow community review
3. **Timestamp Ordering**: Fair ordering based on submission time

### Economic Security
1. **Deposit Slashing**: Consider implementing deposit slashing for malicious proposals
2. **Quorum Requirements**: Ensures sufficient participation
3. **Threshold Validation**: Prevents low-quality proposals from passing

## Configuration Recommendations

### Conservative Settings
- `min_proposal_deposit`: 1% of total supply
- `max_proposals_per_proposer`: 3
- `proposal_cooldown_seconds`: 7 days
- `voting_period_days`: 7 days
- `min_quorum_percentage`: 20%
- `min_voting_percentage`: 51%

### Progressive Settings
- Start with higher deposits and limits
- Gradually reduce as governance matures
- Monitor proposal quality and participation
- Adjust based on community feedback

## Testing

The contract includes comprehensive tests covering:
- Spam protection mechanisms
- Commit-reveal functionality
- Time-lock behavior
- Edge cases and error conditions
- Statistics tracking
- Cleanup operations
- **Multi-signature configuration**
- **Multi-sig proposal creation and confirmation**
- **Multi-sig threshold validation**
- **Multi-sig execution logic**
- **Integration with governance operations**

Run tests with:
```bash
cargo test -p propchain-governance
```

## Integration Notes

### Multi-Sig Integration
- Configure multi-sig before enabling critical operations
- Choose signers carefully based on security requirements
- Set appropriate thresholds for your use case
- Monitor multi-sig proposals and confirmations
- Implement alerts for critical multi-sig operations

### Token Requirements
- Contract needs access to token contract for deposit handling
- Voting weight should be based on token holdings
- Consider implementing token locking during voting

### Frontend Integration
- Display proposal status and deadlines clearly
- Show proposer statistics
- Implement commit-reveal UI if enabled
- Provide cleanup interface for admins
- **Show multi-sig configuration and signer list**
- **Display multi-sig proposal confirmation status**
- **Implement multi-sig workflow UI**

### Upgrade Path
- Store configuration in upgradeable storage
- Implement versioning for proposal structures
- Consider migration strategies for existing proposals
- Plan for multi-sig configuration upgrades

## License

MIT License - see LICENSE file for details.
