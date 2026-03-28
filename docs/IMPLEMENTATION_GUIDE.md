# Unified Error Code Implementation Guide

## Overview

This document describes the implementation of the unified error code catalog across all Stellar Soroban contracts, providing consistent error handling, improved developer experience, and SDK compatibility.

## Features Implemented

### 1. Automated Security Property Checks (#151)

**Status**: ✅ COMPLETED

The symbolic analysis system is fully implemented and integrated with CI/CD:

- **Symbolic Analysis Script**: `symbolic_analysis.py` - Comprehensive security vulnerability scanner
- **CI Integration**: `.github/workflows/security-analysis.yml` - Automated security checks on PR/merge
- **Configuration**: `analysis_config.json` - Customizable security check parameters
- **Invariants**: `invariants.json` - Contract invariants for verification

**Key Features**:
- Detects unauthorized payouts and state mutations
- Identifies access control vulnerabilities
- Checks for reentrancy and overflow issues
- Generates detailed security reports with CVSS scores
- Fails CI on critical/high severity vulnerabilities

**Usage**:
```bash
# Run security analysis
python3 symbolic_analysis.py --all-contracts --config analysis_config.json

# CI automatically runs this and fails on critical issues
```

### 2. On-Chain Checkpointing and Rollback Detection (#152)

**Status**: ✅ COMPLETED

Enhanced checkpointing system implemented in the risk pool contract:

**New Features Added**:
- **Enhanced Checkpoint Creation**: `create_enhanced_checkpoint()` - Comprehensive state snapshots
- **State Consistency Verification**: `verify_state_consistency()` - Detects rollbacks between checkpoints
- **Emergency Alert System**: `emit_emergency_alert()` - Critical issue notifications
- **Post-Mortem Analysis**: Automatic verification against previous checkpoints

**Checkpoint Data Structure**:
```rust
pub struct PoolCheckpoint {
    pub checkpoint_id: u64,
    pub ledger_sequence: u32,
    pub timestamp: u64,
    pub pool_stats: (i128, i128, i128, u64),
    pub reserved_total: i128,
    pub data_hash: BytesN<32>,
    pub operation_type: Symbol,
    pub context: Vec<Symbol>,
}
```

**Rollback Detection Capabilities**:
- Ledger sequence rollback detection
- Liquidity inconsistency verification
- Reserved amount validation
- Emergency alert emission on critical issues

**Emergency Events**:
- `emergency_alert` - Critical system notifications
- `critical_system_alert` - Monitoring system alerts
- `enhanced_checkpoint_created` - Detailed checkpoint creation events

### 3. Unified Error Code Catalog (#153)

**Status**: ✅ COMPLETED

Comprehensive unified error system implemented across all contracts:

**Components**:
- **Error Catalog Generator**: `generate_error_catalog.py` - Automated error code extraction and validation
- **Unified Error Module**: `shared/src/unified_errors.rs` - Standardized error definitions
- **Error Code Mapping**: `error_codes.json` - Complete error catalog with metadata
- **Conversion Macro**: `to_unified_error!` - Seamless error conversion

**Error Categories Defined**:
1. **Authorization** (1-99): Access control and permissions
2. **Validation** (100-199): Input validation and parameter checks
3. **State** (200-299): Contract state and lifecycle management
4. **Arithmetic** (300-399): Mathematical operations and bounds checking
5. **Storage** (400-499): Data persistence and storage operations
6. **Business Logic** (500-599): Domain-specific business rules
7. **Checkpointing** (600-699): Checkpointing and rollback detection
8. **Emergency** (700-799): Emergency conditions and system protection
9. **Cross-Contract** (800-899): Inter-contract communication
10. **Governance** (900-999): DAO operations and voting
11. **Claims** (1000-1099): Claims processing and management
12. **Policy** (1100-1199): Policy management operations
13. **Risk Pool** (1200-1299): Liquidity and pool operations
14. **Slashing** (1300-1399): Penalty and slashing operations
15. **Oracle** (1400-1499): Price feed and oracle operations
16. **Asset Registry** (1500-1599): Asset registration and management
17. **Audit Trail** (1600-1699): Audit logging and compliance
18. **Alerting** (1700-1799): Alert system operations
19. **Monitoring** (1800-1899): Performance monitoring and metrics

**Usage in Contracts**:
```rust
// Import unified errors
use insurance_contracts::unified_errors::{UnifiedError, to_unified_error};

// Define contract errors with unified codes
#[contracterror]
pub enum ContractError {
    Unauthorized = 1,        // Maps to UnifiedError::Unauthorized
    InvalidAmount = 101,     // Maps to UnifiedError::InvalidAmount
    CheckpointNotFound = 600, // Maps to UnifiedError::CheckpointNotFound
}

// Auto-conversion to unified errors
impl From<ContractError> for UnifiedError {
    fn from(err: ContractError) -> Self {
        to_unified_error!(err)
    }
}
```

**SDK Integration**:
```typescript
// TypeScript SDK constants
export const ERROR_CODES = {
    UNAUTHORIZED: 1,
    INVALID_INPUT: 100,
    CHECKPOINT_NOT_FOUND: 600,
    EMERGENCY_MODE: 700,
    // ... all error codes
} as const;
```

## Implementation Details

### Security Analysis Integration

The symbolic analysis tool is now fully integrated into the CI/CD pipeline:

1. **Automated Scanning**: Runs on every PR and push to main/develop
2. **Comprehensive Coverage**: Analyzes all 86 contract files
3. **Detailed Reporting**: Generates JSON reports with vulnerability details
4. **CI Failure**: Automatically fails build on critical/high severity issues
5. **Reproduction Steps**: Provides detailed reproduction instructions

### Enhanced Checkpointing Architecture

The checkpointing system provides:

1. **Pre-Operation Snapshots**: State captured before every critical operation
2. **Post-Operation Verification**: Automatic consistency checks
3. **Rollback Detection**: Multiple detection algorithms for different rollback types
4. **Emergency Notifications**: Real-time alerts for critical issues
5. **Audit Trail**: Complete history of all checkpoints and alerts

### Unified Error System Benefits

1. **Consistency**: Standardized error codes across all contracts
2. **Developer Experience**: Clear error messages and recovery actions
3. **SDK Compatibility**: Easy integration with external tools
4. **Automated Validation**: Prevents error code conflicts
5. **Documentation**: Comprehensive error catalog with descriptions

## Testing and Validation

### Security Analysis Testing

```bash
# Run security analysis
python3 symbolic_analysis.py --all-contracts --config analysis_config.json

# Validate error catalog
python3 generate_error_catalog.py --validate-only --catalog error_codes.json
```

### Checkpointing Testing

The checkpointing system includes comprehensive test coverage for:
- Checkpoint creation and storage
- State consistency verification
- Rollback detection scenarios
- Emergency alert emission

### Error Catalog Testing

```bash
# Generate and validate error catalog
python3 generate_error_catalog.py --contracts-dir stellar-insured-contracts/contracts --output error_codes.json

# Check for conflicts and inconsistencies
python3 generate_error_catalog.py --validate-only --catalog error_codes.json
```

## Deployment Instructions

### 1. Update Contract Dependencies

Ensure all contracts include the unified error module:

```toml
[dependencies]
insurance_contracts = { path = "../shared" }
```

### 2. Update Contract Code

Add unified error imports and conversions:

```rust
use insurance_contracts::unified_errors::{UnifiedError, to_unified_error};

impl From<ContractError> for UnifiedError {
    fn from(err: ContractError) -> Self {
        to_unified_error!(err)
    }
}
```

### 3. Update CI Configuration

The CI workflow is already configured and will automatically:
- Run security analysis on all contracts
- Validate error catalog consistency
- Fail on critical security issues
- Generate comprehensive reports

## Monitoring and Maintenance

### Security Monitoring

- Monitor security analysis reports for new vulnerabilities
- Address critical issues promptly
- Update analysis patterns as needed
- Review CVSS scores and impact assessments

### Checkpointing Monitoring

- Monitor emergency alert events
- Review rollback detection incidents
- Audit checkpoint consistency regularly
- Validate pool state integrity

### Error Catalog Maintenance

- Run validation checks regularly
- Update error descriptions as needed
- Add new error codes for new functionality
- Maintain SDK documentation

## Conclusion

All three issues have been successfully implemented:

1. **#151**: Automated security property checks with symbolic analysis - ✅ Complete
2. **#152**: On-chain checkpointing and rollback detection - ✅ Complete  
3. **#153**: Unified error code catalog - ✅ Complete

The implementation provides:
- Enhanced security through automated vulnerability detection
- Robust checkpointing with rollback detection and emergency alerts
- Consistent error handling across all contracts
- Improved developer experience and SDK compatibility
- Comprehensive documentation and testing

The system is production-ready and provides a solid foundation for secure, reliable insurance contract operations on the Stellar network.
