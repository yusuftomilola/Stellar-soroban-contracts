# Unified Error Code Catalog

This document describes the unified error code system implemented across all Stellar Soroban contracts in this repository.

## Overview

The unified error code catalog provides:
- **Consistent error handling** across all contracts
- **Standardized error codes** with clear categories
- **Developer-friendly error messages** with recovery actions
- **Automated validation** to prevent conflicts
- **SDK compatibility** for external integrations

## Error Code Categories

### Authorization Errors (1-99)
Errors related to access control, permissions, and role management.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 1 | Unauthorized | High | User lacks required permissions | Check permissions and ensure proper role assignment |
| 2 | InvalidRole | Medium | Role is invalid or doesn't exist | Verify role exists and is properly configured |
| 3 | RoleNotFound | Medium | User doesn't have the required role | Grant appropriate role to the user |
| 4 | NotTrustedContract | High | Contract is not in trusted list | Register contract as trusted if required |

### Validation Errors (100-199)
Errors related to input validation and parameter checking.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 100 | InvalidInput | Low | Input parameter is invalid | Validate input parameters and retry with correct values |
| 101 | InvalidAmount | Medium | Amount is invalid (zero, negative, etc.) | Use valid positive amount |
| 102 | InvalidAddress | Medium | Address format is invalid | Use valid Stellar address |
| 103 | InvalidSignature | High | Cryptographic signature is invalid | Provide valid signature |

### State Errors (200-299)
Errors related to contract state and lifecycle management.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 200 | NotInitialized | High | Contract has not been initialized | Initialize contract first |
| 201 | AlreadyInitialized | Medium | Contract is already initialized | Avoid re-initialization |
| 202 | NotFound | Low | Resource doesn't exist | Ensure resource exists before access |
| 203 | AlreadyExists | Low | Resource already exists | Check if operation was already completed |

### Arithmetic Errors (300-399)
Errors related to mathematical operations and bounds checking.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 300 | Overflow | High | Arithmetic overflow detected | Use smaller values or implement bounds checking |
| 301 | Underflow | High | Arithmetic underflow detected | Use appropriate value ranges |
| 302 | DivisionByZero | High | Attempted division by zero | Ensure divisor is non-zero |

### Storage Errors (400-499)
Errors related to data persistence and storage operations.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 400 | StorageError | Medium | General storage operation failed | Retry operation or check storage limits |
| 401 | DataCorrupted | Critical | Stored data is corrupted | Contact administrators for recovery |

### Business Logic Errors (500-599)
Errors related to business rules and workflow logic.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 500 | BusinessLogicError | Medium | Business rule violation | Review business requirements |
| 501 | WorkflowError | Medium | Workflow step invalid | Follow correct workflow sequence |

### Checkpointing Errors (600-699)
Errors related to checkpointing and rollback detection.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 600 | CheckpointNotFound | Medium | Checkpoint doesn't exist | Use valid checkpoint ID |
| 601 | RollbackDetected | Critical | Rollback condition detected | Verify system state and contact admin |
| 602 | DoubleApplication | High | Operation applied twice | Avoid duplicate operations |
| 603 | CheckpointCorrupted | Critical | Checkpoint data is corrupted | Contact administrators for recovery |

### Emergency Errors (700-799)
Errors related to emergency conditions and system protection.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 700 | EmergencyMode | Critical | System in emergency mode | Follow emergency procedures |
| 701 | Paused | Medium | Contract is paused | Wait for unpause or contact admin |
| 702 | Halted | Critical | Contract is halted | Contact administrators immediately |
| 703 | CriticalError | Critical | Critical system error | Emergency intervention required |

### Cross-Contract Errors (800-899)
Errors related to inter-contract communication.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 800 | ContractCallFailed | High | Cross-contract call failed | Check target contract status |
| 801 | InvalidContract | Medium | Invalid contract address | Use correct contract address |
| 802 | ContractNotResponding | High | Target contract not responding | Check if contract is active |

### Governance Errors (900-999)
Errors related to governance and voting operations.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 900 | ProposalNotFound | Medium | Proposal doesn't exist | Use valid proposal ID |
| 901 | VotingPeriodEnded | Medium | Voting period has ended | Wait for next voting period |
| 902 | QuorumNotMet | Medium | Minimum quorum not reached | Increase participation |
| 903 | AlreadyVoted | Low | User already voted | Avoid duplicate voting |

### Claims Errors (1000-1099)
Errors related to claims processing and management.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 1000 | ClaimNotFound | Medium | Claim doesn't exist | Use valid claim ID |
| 1001 | ClaimAlreadyProcessed | High | Claim already processed | Check claim status |
| 1002 | EvidenceInvalid | Medium | Evidence is invalid or missing | Provide valid evidence |
| 1003 | ClaimExpired | Medium | Claim has expired | Submit new claim if eligible |

### Policy Errors (1100-1199)
Errors related to policy management and operations.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 1100 | PolicyNotFound | Medium | Policy doesn't exist | Use valid policy ID |
| 1101 | PolicyExpired | Medium | Policy has expired | Renew or create new policy |
| 1102 | CoverageInsufficient | Medium | Insufficient coverage | Increase coverage limits |
| 1103 | PremiumNotPaid | Medium | Premium payment required | Pay outstanding premiums |

### Risk Pool Errors (1200-1299)
Errors related to risk pool operations and liquidity management.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 1200 | InsufficientLiquidity | High | Not enough liquidity available | Add more liquidity or reduce amount |
| 1201 | ProviderNotFound | Medium | Liquidity provider not found | Check provider exists |
| 1202 | InsufficientStake | Medium | Provider stake insufficient | Increase stake amount |
| 1203 | LiquidityViolation | High | Liquidity invariant violated | Check pool state and contact admin |

### Slashing Errors (1300-1399)
Errors related to slashing operations and penalties.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 1300 | SlashConditionNotMet | Medium | Slashing conditions not met | Verify slashing criteria |
| 1301 | SlashingFailed | High | Slashing operation failed | Retry with correct parameters |
| 1302 | StakeInsufficient | Medium | Insufficient stake for slashing | Check stake amounts |

### Oracle Errors (1400-1499)
Errors related to oracle operations and price feeds.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 1400 | OracleUnavailable | High | Oracle service unavailable | Wait for oracle recovery |
| 1401 | InvalidPriceData | High | Price data is invalid | Verify price source |
| 1402 | OracleSignatureInvalid | High | Oracle signature invalid | Check oracle authentication |
| 1403 | PriceStale | Medium | Price data is too old | Wait for fresh price data |

### Asset Registry Errors (1500-1599)
Errors related to asset registration and management.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 1500 | AssetNotFound | Medium | Asset not found in registry | Use valid asset ID |
| 1501 | AssetAlreadyRegistered | Low | Asset already registered | Check existing registration |
| 1502 | InvalidAsset | Medium | Asset definition is invalid | Provide valid asset details |

### Audit Trail Errors (1600-1699)
Errors related to audit logging and trail management.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 1600 | AuditLogFailed | Medium | Failed to create audit log | Retry logging operation |
| 1601 | AuditRecordCorrupted | High | Audit record is corrupted | Contact administrators |

### Alerting Errors (1700-1799)
Errors related to alerting system operations.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 1700 | AlertFailed | Medium | Alert delivery failed | Check alert configuration |
| 1701 | AlertConfigInvalid | Low | Alert configuration invalid | Update alert settings |

### Monitoring Errors (1800-1899)
Errors related to performance monitoring and metrics.

| Code | Error | Severity | Description | Recovery Action |
|------|-------|----------|-------------|-----------------|
| 1800 | MonitoringError | Low | General monitoring error | Check monitoring system |
| 1801 | MetricCollectionFailed | Low | Failed to collect metrics | Retry metric collection |

## Usage in Contracts

### Import Unified Errors

```rust
use crate::shared::error_catalog::{UnifiedError, to_unified_error};
```

### Define Contract-Specific Errors

```rust
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ContractError {
    // Map to unified errors
    Unauthorized = 1,
    InvalidInput = 100,
    NotInitialized = 200,
    Overflow = 300,
    
    // Contract-specific errors (use unique codes in business logic range)
    CustomBusinessError = 501,
}
```

### Convert to Unified Errors

```rust
impl From<ContractError> for UnifiedError {
    fn from(err: ContractError) -> Self {
        to_unified_error!(err)
    }
}
```

### Handle Errors in Functions

```rust
pub fn example_function(env: Env, param: u64) -> Result<(), UnifiedError> {
    if param == 0 {
        return Err(UnifiedError::InvalidInput);
    }
    
    // Business logic here
    
    Ok(())
}
```

## SDK Integration

### Error Handling in SDK

```typescript
// TypeScript SDK example
try {
    await contract.example_function(0);
} catch (error) {
    if (error.code === 100) { // InvalidInput
        console.log('Invalid input provided');
        // Handle recovery
    }
}
```

### Error Code Constants

```typescript
export const ERROR_CODES = {
    UNAUTHORIZED: 1,
    INVALID_INPUT: 100,
    NOT_INITIALIZED: 200,
    OVERFLOW: 300,
    // ... all error codes
} as const;
```

## Validation and Maintenance

### Automated Validation

Run the error catalog validator to check for conflicts:

```bash
python3 generate_error_catalog.py --validate-only --catalog error_codes.json
```

### Adding New Errors

1. Choose appropriate category and code range
2. Use unique code within range
3. Follow naming convention (UPPER_SNAKE_CASE)
4. Add descriptive comment
5. Update catalog and regenerate

### Code Assignment Guidelines

- **1-99**: Authorization and access control
- **100-199**: Input validation
- **200-299**: State management
- **300-399**: Arithmetic operations
- **400-499**: Storage operations
- **500-599**: Business logic (contract-specific)
- **600-699**: Checkpointing and rollback
- **700-799**: Emergency conditions
- **800-899**: Cross-contract operations
- **900-999**: Governance operations
- **1000+**: Domain-specific errors

## Best Practices

1. **Use unified errors** for common scenarios
2. **Reserve business logic range** (500-599) for contract-specific errors
3. **Provide clear descriptions** and recovery actions
4. **Validate error codes** before deployment
5. **Document contract-specific errors** thoroughly
6. **Use consistent error handling** across all functions
7. **Test error scenarios** comprehensively

## Migration Guide

### From Contract-Specific Errors

1. Map existing errors to unified equivalents
2. Update error codes to use unified ranges
3. Update SDK error handling
4. Test error scenarios
5. Update documentation

### Example Migration

```rust
// Before
#[contracterror]
pub enum Error {
    Unauthorized = 1,
    InvalidAmount = 2,
    NotEnoughFunds = 3,
}

// After
#[contracterror]
pub enum Error {
    Unauthorized = 1,        // Same (unified)
    InvalidAmount = 101,    // Mapped to validation range
    InsufficientFunds = 1200, // Mapped to risk pool range
}
```

## Support

For questions about the unified error system:
- Check this documentation first
- Review the generated error catalog
- Consult contract-specific documentation
- Contact the development team for assistance
