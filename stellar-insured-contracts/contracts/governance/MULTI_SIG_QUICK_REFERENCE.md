# Multi-Signature Quick Reference Guide

## Quick Setup (2-of-3 Multi-Sig)

```rust
use propchain_governance::{SignerInfo, OperationType};

// 1. Initialize governance
GovernanceContract::initialize(
    &env, admin, token_contract,
    7, 51, 10, 100, 5, 86400,
    false, 1, 1, false, 3600,
    60, 67,
);

// 2. Configure multi-sig
let signers = Vec::from_array(&env, [
    SignerInfo { address: signer1, weight: 1, active: true },
    SignerInfo { address: signer2, weight: 1, active: true },
    SignerInfo { address: signer3, weight: 1, active: true },
]);

GovernanceContract::configure_multi_sig(&env, admin, signers, 2);
```

## Common Operations

### Emergency Pause

```rust
// Create proposal
let data = Map::from_array(&env, [
    (Symbol::short("reason"), String::from_str(&env, "Security issue")),
]);

let id = GovernanceContract::create_multi_sig_proposal(
    &env, signer1, OperationType::Pause, data, None, 604800
)?;

// Confirm (needs 2 confirmations)
GovernanceContract::confirm_multi_sig_proposal(&env, signer2, id)?;

// Execute
GovernanceContract::execute_multi_sig_proposal(&env, any_address, id)?;
```

### Update Parameter

```rust
// Update min_voting_percentage to 60%
let data = Map::from_array(&env, [
    (Symbol::short("value"), String::from_str(&env, "60")),
]);

let id = GovernanceContract::create_multi_sig_proposal(
    &env, signer1,
    OperationType::UpdateParameter(Symbol::short("min_voting_pct")),
    data, None, 604800
)?;

// Confirm and execute
GovernanceContract::confirm_multi_sig_proposal(&env, signer2, id)?;
GovernanceContract::execute_multi_sig_proposal(&env, signer1, id)?;
```

### Add New Signer

```rust
// Existing signer adds new signer
GovernanceContract::add_signer(
    &env, signer1, new_signer, 1
)?;
```

### Remove Signer

```rust
// Existing signer removes signer
GovernanceContract::remove_signer(
    &env, signer2, signer_to_remove
)?;
```

## Query Functions

```rust
// Get config
let config = GovernanceContract::get_multi_sig_config(&env).unwrap();

// Check if signer
let is_valid = GovernanceContract::is_signer(&env, &address, &config);

// Get signer weight
let weight = GovernanceContract::get_signer_weight(&env, &address, &config);

// Get proposal
let proposal = GovernanceContract::get_multi_sig_proposal(&env, id).unwrap();

// Get confirmations
let confirms = GovernanceContract::get_multi_sig_confirmations(&env, id);

// Check if enabled
let enabled = GovernanceContract::is_multi_sig_enabled(&env);
```

## Error Codes

| Error | Code | Description |
|-------|------|-------------|
| `MultiSigNotConfigured` | 17 | Multi-sig not set up |
| `MultiSigThresholdNotMet` | 18 | Not enough confirmations |
| `MultiSigAlreadyConfirmed` | 19 | Duplicate confirmation |
| `MultiSigNotConfirmed` | 20 | Signer hasn't confirmed |
| `MultiSigInvalidSigner` | 21 | Address not a signer |
| `MultiSigProposalAlreadyExecuted` | 22 | Already executed |

## Recommended Thresholds

| Security Level | Configuration | Use Case |
|---------------|---------------|----------|
| Basic | 2-of-3 | Small teams, low value |
| Balanced | 3-of-5 | Medium organizations |
| High | 5-of-7 | Large organizations, high value |
| Maximum | 7-of-9 | Critical infrastructure |

## Best Practices Checklist

- [ ] Choose diverse signers (different orgs/teams)
- [ ] Use hardware wallets for signers
- [ ] Set reasonable expiry times (7 days recommended)
- [ ] Monitor all proposals actively
- [ ] Have emergency procedures documented
- [ ] Regular security audits
- [ ] Test in staging environment first
- [ ] Maintain backup communication channels

## Integration Flow

```
1. Configure Multi-Sig
   ↓
2. Create Proposal (auto-confirms creator)
   ↓
3. Gather Confirmations (need threshold)
   ↓
4. Execute (anyone can execute once threshold met)
   ↓
5. Operation Completed
```

## Storage Keys

- `MSIG_CFG`: Multi-sig configuration
- `MSIG_PRP`: Multi-sig proposals
- `MSIG_CFM`: Multi-sig confirmations
- `MSIG_NEXT_ID`: Next proposal ID

## Gas Optimization Tips

1. Use appropriate threshold (don't over-engineer)
2. Set reasonable expiry times
3. Batch operations when possible
4. Clean up old proposals periodically

## Troubleshooting

**Issue**: Can't create proposal
- Check: Is multi-sig configured? (`get_multi_sig_config`)
- Check: Is caller a valid signer? (`is_signer`)

**Issue**: Can't execute proposal
- Check: Has threshold been met? (`get_multi_sig_proposal.confirmed_weight`)
- Check: Is proposal expired? (`get_multi_sig_proposal.expires_at`)
- Check: Already executed? (`get_multi_sig_proposal.executed`)

**Issue**: Can't add/remove signer
- Check: Is caller an existing signer?
- Check: Signer doesn't already exist/isn't removed?

## Testing Commands

```bash
# Run all tests
cargo test -p propchain-governance

# Run specific test
cargo test -p propchain-governance test_configure_multi_sig

# Run with output
cargo test -p propchain-governance -- --nocapture
```

## Contract Addresses (After Deployment)

Fill in after deployment:
- Governance Contract: `_________________`
- Token Contract: `_________________`
- Multi-Sig Signers:
  - Signer 1: `_________________`
  - Signer 2: `_________________`
  - Signer 3: `_________________`

## Emergency Contacts

Maintain secure off-chain communication:
- Signer 1 Contact: _________________
- Signer 2 Contact: _________________
- Signer 3 Contact: _________________

---

**Note**: This guide complements the full README documentation. Refer to `README.md` for complete details.
