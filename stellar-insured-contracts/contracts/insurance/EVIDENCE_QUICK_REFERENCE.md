# Claims Evidence System - Quick Reference

## Quick Start

### Submit Evidence for a Claim

```rust
// Single evidence submission
let evidence_id = contract.submit_evidence(
    claim_id,
    "photo".to_string(),
    "QmYourIPFSHash".to_string(),
    content_hash_bytes,  // 32 bytes
    file_size_bytes,
    Some("ipfs://MetadataHash".to_string()),
    Some("Description".to_string()),
)?;
```

### Batch Submit Multiple Evidence

```rust
let batch = vec![
    ("photo".to_string(), "Qm1".to_string(), hash1, size1, None),
    ("document".to_string(), "Qm2".to_string(), hash2, size2, None),
];

let ids = contract.batch_submit_evidence(claim_id, batch)?;
```

### Verify Evidence

```rust
// Assessor or admin only
contract.verify_evidence(
    evidence_id,
    true,  // is_valid
    "Evidence appears authentic".to_string(),
)?;
```

## Function Cheat Sheet

| Function | Who Can Call | Description | Returns |
|----------|-------------|-------------|---------|
| `submit_evidence(...)` | Claimant/Assessor/Admin | Add evidence to claim | evidence_id |
| `batch_submit_evidence(...)` | Claimant/Assessor/Admin | Add multiple evidence | Vec<evidence_ids> |
| `verify_evidence(...)` | Assessor/Admin | Verify evidence validity | () |
| `get_claim_evidence(id)` | Anyone | Get all evidence for claim | Vec<EvidenceItem> |
| `get_evidence(id)` | Anyone | Get specific evidence | Option<EvidenceItem> |
| `get_evidence_verifications(id)` | Anyone | Get verification history | Vec<Verification> |
| `is_evidence_verified(id)` | Anyone | Check consensus validity | bool |
| `get_evidence_verification_status(id)` | Anyone | Get detailed stats | Option<(total,valid,invalid,consensus)> |
| `calculate_evidence_storage_cost(id)` | Anyone | Calculate storage cost | Option<u128> |
| `get_claim_evidence_total_cost(id)` | Anyone | Total cost for all evidence | u128 |

## Evidence Types

Use these standard types:
- `"photo"` - Photos/images
- `"document"` - PDFs, text documents
- `"video"` - Video recordings
- `"sensor_data"` - IoT sensor readings
- `"audio"` - Audio recordings
- `"report"` - Official reports

## IPFS Hash Requirements

✅ **Valid Formats:**
- `"QmX..."` (CID v0)
- `"bafy..."` (CID v1)

❌ **Invalid:**
- Empty string
- Doesn't start with "Qm" or "bafy"
- Local file paths

## Content Hash Requirements

- **Algorithm**: SHA-256
- **Length**: Exactly 32 bytes (256 bits)
- **Format**: `Vec<u8>` in Rust

```rust
// Example: Calculate SHA-256 hash
use sha2::{Sha256, Digest};
let mut hasher = Sha256::new();
hasher.update(&file_content);
let hash = hasher.finalize().to_vec(); // 32 bytes
```

## Verification Workflow

```
1. Evidence Submitted
   ↓
2. Assessor Reviews (checks IPFS, validates content)
   ↓
3. Assessor Verifies (valid/invalid + notes)
   ↓
4. Multiple Assessors (optional, for consensus)
   ↓
5. Consensus Reached (majority rules)
   ↓
6. Evidence Status Updated
```

## Common Patterns

### Pattern 1: Complete Evidence Flow

```rust
// 1. Submit claim
let claim_id = contract.submit_claim(...);

// 2. Submit primary evidence
let ev1 = contract.submit_evidence(claim_id, "photo", ...);

// 3. Submit additional evidence
let ev2 = contract.submit_evidence(claim_id, "document", ...);

// 4. Assessor verifies both
contract.verify_evidence(ev1, true, "Valid");
contract.verify_evidence(ev2, true, "Valid");

// 5. Check consensus before processing
if contract.is_evidence_verified(ev1) && 
   contract.is_evidence_verified(ev2) {
    contract.process_claim(claim_id, true, ...);
}
```

### Pattern 2: Check Evidence Status

```typescript
async function checkEvidenceStatus(evidenceId) {
    const evidence = await contract.get_evidence(evidenceId);
    const verifications = await contract.get_evidence_verifications(evidenceId);
    const status = await contract.get_evidence_verification_status(evidenceId);
    
    console.log('Evidence:', evidence);
    console.log('Verified:', evidence.verified);
    console.log('Verifications:', verifications.length);
    console.log('Consensus:', status ? status[3] : 'No verifications');
}
```

### Pattern 3: Batch Upload

```javascript
// Frontend: Upload multiple files
const files = [file1, file2, file3];
const batch = [];

for (const file of files) {
    const { cid, size } = await uploadToIPFS(file);
    const hash = await calculateSHA256(file);
    batch.push([
        getFileType(file.name),
        cid.toString(),
        Array.from(hash),
        size,
        null
    ]);
}

await contract.batch_submit_evidence(claim_id, batch);
```

## Storage Costs

**Formula:**
```
Cost = 1000 (base) + (file_size × 10) + (500 if verified)
```

**Examples:**
- 100 KB photo: ~1,001,000 units
- 1 MB document: ~10,001,000 units  
- 10 MB video: ~100,001,000 units

## Events to Monitor

### EvidenceSubmitted
```javascript
contract.on('EvidenceSubmitted', (evId, claimId, type, hash, submitter, time) => {
    console.log(`New evidence ${evId} for claim ${claimId}`);
});
```

### EvidenceVerified
```javascript
contract.on('EvidenceVerified', (evId, verifier, valid, time) => {
    console.log(`Evidence ${evId} verified: ${valid ? 'VALID' : 'INVALID'}`);
});
```

## Access Control Matrix

| Action | Claimant | Assessor | Admin |
|--------|----------|----------|-------|
| Submit Evidence | ✅ | ✅ | ✅ |
| Verify Evidence | ❌ | ✅ | ✅ |
| View Evidence | ✅ | ✅ | ✅ |
| Delete Evidence | ❌ | ❌ | ❌ |

## Testing Commands

```bash
# Run all evidence tests
cargo test -p propchain-insurance evidence_tests

# Run specific test
cargo test -p propchain-insurance test_submit_evidence_basic

# Run with output
cargo test -p propchain-insurance -- --nocapture
```

## Troubleshooting

| Error | Cause | Solution |
|-------|-------|----------|
| `EvidenceNonceEmpty` | Empty evidence_type | Use non-empty string |
| `EvidenceInvalidUriScheme` | Invalid IPFS URI | Must start with ipfs:// or https:// |
| `EvidenceInvalidHashLength` | Wrong hash length | Ensure exactly 32 bytes |
| `Unauthorized` | Wrong caller | Must be claimant/assessor/admin |
| `DuplicateClaim` | Duplicate verification | Use different assessor |
| `InvalidParameters` | Bad IPFS format | Use valid CID (Qm... or bafy...) |

## Best Practices

1. ✅ Always validate files before uploading to IPFS
2. ✅ Use descriptive evidence types
3. ✅ Include metadata when available
4. ✅ Get multiple verifications for important evidence
5. ✅ Monitor verification consensus
6. ✅ Compress large files before upload
7. ✅ Keep backup copies off-chain
8. ✅ Use batch submission for multiple files

## Integration Checklist

- [ ] Set up IPFS gateway access
- [ ] Implement file hashing (SHA-256)
- [ ] Create evidence upload UI
- [ ] Add assessor verification interface
- [ ] Set up event listeners
- [ ] Implement error handling
- [ ] Test with various file types
- [ ] Configure access control

---

**Full Documentation**: See [CLAIMS_EVIDENCE_VERIFICATION.md](../docs/CLAIMS_EVIDENCE_VERIFICATION.md)
