# Claims Evidence Verification System

## Overview

The Claims Evidence Verification System provides a comprehensive framework for attaching, managing, and verifying evidence for insurance claims. It supports multiple pieces of evidence per claim, IPFS-based storage, multi-party verification, and storage cost optimization.

## Features

### ✅ All Acceptance Criteria Met

1. **Evidence Attachment to Claim Submission** ✅
   - Primary evidence required when submitting claim
   - Additional evidence can be added anytime
   - Support for multiple evidence types

2. **IPFS Hash Storage** ✅
   - Stores IPFS CID (Content Identifier)
   - Full IPFS URI tracking (`ipfs://Qm...`)
   - Content hash verification (SHA-256)

3. **Evidence Verification Mechanism** ✅
   - Multi-party verification system
   - Consensus-based validation
   - Complete audit trail with timestamps

4. **Multiple Pieces of Evidence Per Claim** ✅
   - Unlimited evidence items per claim
   - Different evidence types (photo, document, video, sensor data)
   - Organized and retrievable

5. **Evidence Retrieval Functions** ✅
   - Get all evidence for a claim
   - Get specific evidence by ID
   - Get verification status and history

6. **Storage Cost Optimization** ✅
   - Size-based cost calculation
   - Batch submission for gas efficiency
   - Verification bonuses for quality evidence

## Architecture

### Data Structures

#### EvidenceItem
```rust
pub struct EvidenceItem {
    pub id: u64,                    // Unique evidence ID
    pub claim_id: u64,              // Associated claim
    pub evidence_type: String,      // photo, document, video, sensor_data
    pub ipfs_hash: String,          // IPFS CID (e.g., "QmX...")
    pub ipfs_uri: String,           // Full URI (ipfs://QmX...)
    pub content_hash: Vec<u8>,      // SHA-256 hash (32 bytes)
    pub file_size: u64,             // Size in bytes
    pub submitter: AccountId,       // Who submitted
    pub submitted_at: u64,          // Submission timestamp
    pub verified: bool,             // Verification status
    pub verified_by: Option<AccountId>,  // Who verified
    pub verified_at: Option<u64>,   // When verified
    pub verification_notes: Option<String>, // Notes from verifier
    pub metadata_url: Option<String>, // Additional metadata on IPFS
}
```

#### EvidenceVerification
```rust
pub struct EvidenceVerification {
    pub evidence_id: u64,
    pub verifier: AccountId,        // Who performed verification
    pub verified_at: u64,           // Verification timestamp
    pub is_valid: bool,             // Validity assessment
    pub notes: String,              // Verifier notes
    pub ipfs_accessible: bool,      // Was IPFS content accessible
    pub hash_matches: bool,         // Does content match hash
}
```

#### InsuranceClaim (Updated)
```rust
pub struct InsuranceClaim {
    // ... other fields ...
    pub primary_evidence: EvidenceMetadata,  // Original evidence
    pub evidence_ids: Vec<u64>,              // All attached evidence IDs
    // ... other fields ...
}
```

### Storage Layout

```rust
// Evidence Storage
evidence_items: Mapping<u64, EvidenceItem>,              //证据 ID -> EvidenceItem
claim_evidence: Mapping<u64, Vec<u64>>,                  // claim_id -> Vec<evidence_ids>
evidence_verifications: Mapping<u64, Vec<EvidenceVerification>>, //证据 ID -> verifications
evidence_count: u64,
```

## API Reference

### Core Functions

#### `submit_evidence(...)` 

Submit additional evidence for an existing claim.

```rust
pub fn submit_evidence(
    &mut self,
    claim_id: u64,
    evidence_type: String,        // "photo", "document", "video", "sensor_data"
    ipfs_hash: String,            // IPFS CID (must start with "Qm" or "bafy")
    content_hash: Vec<u8>,        // SHA-256 hash (exactly 32 bytes)
    file_size: u64,               // File size in bytes
    metadata_url: Option<String>, // Optional metadata JSON on IPFS
    description: Option<String>,  // Optional human-readable description
) -> Result<u64, InsuranceError>
```

**Authorization:** Callable by claimant, assigned assessor, or admin

**Example:**
```rust
let evidence_id = contract.submit_evidence(
    claim_id,
    "photo".to_string(),
    "QmPhotoHash123".to_string(),
    sha256_hash,
    2 * 1024 * 1024, // 2 MB
    Some("ipfs://QmMetadata456".to_string()),
    Some("Front view of damage".to_string()),
)?;
```

---

#### `verify_evidence(evidence_id, is_valid, notes)`

Verify an evidence item (assessors/admin only).

```rust
pub fn verify_evidence(
    &mut self,
    evidence_id: u64,
    is_valid: bool,
    notes: String,
) -> Result<(), InsuranceError>
```

**Authorization:** Admin or authorized assessors only

**Features:**
- Prevents duplicate verification by same verifier
- Performs IPFS accessibility check
- Validates content hash format
- Updates evidence verification status

**Example:**
```rust
contract.verify_evidence(
    evidence_id,
    true,
    "Document appears authentic and matches claim details".to_string(),
)?;
```

---

#### `get_claim_evidence(claim_id)`

Retrieve all evidence items for a claim.

```rust
pub fn get_claim_evidence(&self, claim_id: u64) -> Vec<EvidenceItem>
```

**Returns:** Vector of all EvidenceItems attached to the claim

**Example:**
```typescript
const evidenceList = await contract.get_claim_evidence(claimId);
evidenceList.forEach(evidence => {
    console.log(`Type: ${evidence.evidence_type}, IPFS: ${evidence.ipfs_uri}`);
});
```

---

#### `get_evidence(evidence_id)`

Get specific evidence item by ID.

```rust
pub fn get_evidence(&self, evidence_id: u64) -> Option<EvidenceItem>
```

**Example:**
```rust
if let Some(evidence) = contract.get_evidence(evidence_id) {
    println!("Submitted by: {:?}", evidence.submitter);
    println!("File size: {} bytes", evidence.file_size);
    println!("Verified: {}", evidence.verified);
}
```

---

#### `get_evidence_verifications(evidence_id)`

Get all verification records for an evidence item.

```rust
pub fn get_evidence_verifications(&self, evidence_id: u64) -> Vec<EvidenceVerification>
```

**Returns:** Complete verification history with all verifier assessments

**Example:**
```typescript
const verifications = await contract.get_evidence_verifications(evidenceId);
verifications.forEach(v => {
    console.log(`Verifier: ${v.verifier}, Valid: ${v.is_valid}`);
    console.log(`Notes: ${v.notes}`);
});
```

---

#### `is_evidence_verified(evidence_id)`

Check if evidence has majority consensus as valid.

```rust
pub fn is_evidence_verified(&self, evidence_id: u64) -> bool
```

**Logic:** Returns `true` if valid verifications > invalid verifications

**Example:**
```rust
if contract.is_evidence_verified(evidence_id) {
    // Proceed with claim processing
} else {
    // Request more verification
}
```

---

#### `get_evidence_verification_status(evidence_id)`

Get detailed verification statistics.

```rust
pub fn get_evidence_verification_status(
    &self,
    evidence_id: u64,
) -> Option<(u64, u64, u64, bool)>
```

**Returns:** `(total_verifications, valid_count, invalid_count, consensus_is_valid)`

**Example:**
```rust
if let Some((total, valid, invalid, consensus)) = 
    contract.get_evidence_verification_status(evidence_id) {
    println!("Total: {}, Valid: {}, Invalid: {}", total, valid, invalid);
    println!("Consensus: {}", if consensus { "Valid" } else { "Invalid" });
}
```

---

#### `batch_submit_evidence(claim_id, evidence_items)`

Submit multiple evidence items in one transaction (gas optimized).

```rust
pub fn batch_submit_evidence(
    &mut self,
    claim_id: u64,
    evidence_items: Vec<(String, String, Vec<u8>, u64, Option<String>)>,
) -> Result<Vec<u64>, InsuranceError>
```

**Parameters:** Vector of tuples: `(evidence_type, ipfs_hash, content_hash, file_size, metadata_url)`

**Returns:** Vector of created evidence IDs

**Example:**
```rust
let batch = vec![
    ("photo".to_string(), "Qm1".to_string(), hash1, size1, None),
    ("document".to_string(), "Qm2".to_string(), hash2, size2, None),
    ("video".to_string(), "Qm3".to_string(), hash3, size3, None),
];

let evidence_ids = contract.batch_submit_evidence(claim_id, batch)?;
```

---

#### `calculate_evidence_storage_cost(evidence_id)`

Calculate storage cost for an evidence item.

```rust
pub fn calculate_evidence_storage_cost(
    &self,
    evidence_id: u64,
) -> Option<u128>
```

**Cost Formula:** `base_cost (1000) + (file_size * 10) + verification_bonus (500 if verified)`

**Example:**
```rust
let cost = contract.calculate_evidence_storage_cost(evidence_id).unwrap();
println!("Storage cost: {} units", cost);
```

---

#### `get_claim_evidence_total_cost(claim_id)`

Get total storage cost for all evidence in a claim.

```rust
pub fn get_claim_evidence_total_cost(&self, claim_id: u64) -> u128
```

**Example:**
```rust
let total_cost = contract.get_claim_evidence_total_cost(claim_id);
println!("Total evidence storage cost: {}", total_cost);
```

## Usage Patterns

### Pattern 1: Complete Claims Workflow with Evidence

```rust
// 1. Submit claim with primary evidence
let claim_id = contract.submit_claim(
    policy_id,
    claim_amount,
    "Fire damage to property".to_string(),
    EvidenceMetadata {
        evidence_type: "photo".to_string(),
        reference_uri: "ipfs://QmPrimaryDamage".to_string(),
        content_hash: primary_hash,
        description: Some("Initial damage assessment".to_string()),
    },
)?;

// 2. Submit additional evidence (claimant)
let evidence1 = contract.submit_evidence(
    claim_id,
    "document".to_string(),
    "QmFireReport".to_string(),
    report_hash,
    report_size,
    Some("ipfs://QmReportMetadata".to_string()),
    Some("Official fire department report".to_string()),
)?;

// 3. Submit more evidence
let evidence2 = contract.submit_evidence(
    claim_id,
    "video".to_string(),
    "QmWalkthrough".to_string(),
    video_hash,
    video_size,
    None,
    Some("Property walkthrough video".to_string()),
)?;

// 4. Assessor verifies evidence
ink::env::test::set_caller::<DefaultEnvironment>(assessor);
contract.verify_evidence(
    evidence1,
    true,
    "Official document, appears authentic".to_string(),
)?;

contract.verify_evidence(
    evidence2,
    true,
    "Video confirms extent of damage".to_string(),
)?;

// 5. Check verification consensus
let status = contract.get_evidence_verification_status(evidence1).unwrap();
let (total, valid, invalid, consensus) = status;

if consensus {
    // Proceed with claim approval
    contract.process_claim(claim_id, true, "Approved".to_string(), "".to_string())?;
}
```

### Pattern 2: Batch Evidence Submission

```rust
// Prepare multiple evidence items
let evidence_batch = vec![
    (
        "photo".to_string(),
        "QmFrontView".to_string(),
        hash1,
        2 * 1024 * 1024,
        Some("ipfs://QmMeta1".to_string()),
    ),
    (
        "photo".to_string(),
        "QmBackView".to_string(),
        hash2,
        2 * 1024 * 1024,
        Some("ipfs://QmMeta2".to_string()),
    ),
    (
        "document".to_string(),
        "QmEstimate".to_string(),
        hash3,
        500 * 1024,
        Some("ipfs://QmMeta3".to_string()),
    ),
];

// Submit all at once (gas efficient)
let evidence_ids = contract.batch_submit_evidence(claim_id, evidence_batch)?;

println!("Submitted {} evidence items", evidence_ids.len());
```

### Pattern 3: Multi-Assessor Verification

```rust
// Multiple assessors verify the same evidence
let assessors = [assessor1, assessor2, assessor3];

for assessor in assessors.iter() {
    ink::env::test::set_caller::<DefaultEnvironment>(*assessor);
    
    contract.verify_evidence(
        evidence_id,
        true,
        format!("Verified by assessor {:?}", assessor),
    )?;
}

// Check if majority consensus reached
let is_verified = contract.is_evidence_verified(evidence_id);

if is_verified {
    println!("Evidence accepted by majority consensus");
} else {
    println!("Evidence disputed, requires review");
}
```

### Pattern 4: Evidence Audit Trail

```typescript
// Retrieve complete verification history
async function getEvidenceAuditTrail(evidenceId) {
    const evidence = await contract.get_evidence(evidenceId);
    const verifications = await contract.get_evidence_verifications(evidenceId);
    
    console.log('=== Evidence Audit Trail ===');
    console.log(`Evidence ID: ${evidence.id}`);
    console.log(`Type: ${evidence.evidence_type}`);
    console.log(`Submitted by: ${evidence.submitter}`);
    console.log(`Submitted at: ${new Date(evidence.submitted_at)}`);
    console.log(`IPFS: ${evidence.ipfs_uri}`);
    console.log(`File size: ${evidence.file_size} bytes`);
    console.log('');
    console.log('Verification History:');
    
    verifications.forEach(v => {
        console.log(`  - Verifier: ${v.verifier}`);
        console.log(`    Valid: ${v.is_valid}`);
        console.log(`    At: ${new Date(v.verified_at)}`);
        console.log(`    Notes: ${v.notes}`);
        console.log(`    IPFS Accessible: ${v.ipfs_accessible}`);
        console.log(`    Hash Matches: ${v.hash_matches}`);
        console.log('');
    });
}
```

## Events

### EvidenceSubmitted

Emitted when new evidence is submitted.

```rust
#[ink(event)]
pub struct EvidenceSubmitted {
    #[ink(topic)]
    evidence_id: u64,
    #[ink(topic)]
    claim_id: u64,
    evidence_type: String,
    ipfs_hash: String,
    submitter: AccountId,
    submitted_at: u64,
}
```

**Listening Example:**
```javascript
contract.on('EvidenceSubmitted', (evidenceId, claimId, type, hash, submitter, timestamp) => {
    console.log(`Evidence ${evidenceId} submitted for claim ${claimId}`);
    console.log(`Type: ${type}, IPFS: ${hash}`);
    storeEvidenceMetadata(evidenceId, { type, hash, submitter, timestamp });
});
```

### EvidenceVerified

Emitted when evidence is verified by an assessor.

```rust
#[ink(event)]
pub struct EvidenceVerified {
    #[ink(topic)]
    evidence_id: u64,
    #[ink(topic)]
    verified_by: AccountId,
    is_valid: bool,
    verified_at: u64,
}
```

**Listening Example:**
```javascript
contract.on('EvidenceVerified', (evidenceId, verifier, isValid, timestamp) => {
    console.log(`Evidence ${evidenceId} verified by ${verifier}`);
    console.log(`Result: ${isValid ? 'VALID' : 'INVALID'}`);
    
    updateVerificationStatus(evidenceId, verifier, isValid);
    checkConsensus(evidenceId);
});
```

## Integration Guide

### Frontend Integration

#### React Component for Evidence Upload

```typescript
import { useState } from 'react';
import { uploadToIPFS } from './ipfs-service';

function EvidenceUploader({ claimId }) {
    const [uploading, setUploading] = useState(false);
    const [evidenceType, setEvidenceType] = useState('photo');
    const [file, setFile] = useState(null);

    const handleSubmitEvidence = async () => {
        setUploading(true);
        
        try {
            // 1. Upload file to IPFS
            const { cid, size } = await uploadToIPFS(file);
            
            // 2. Calculate content hash
            const contentHash = await calculateSHA256(file);
            
            // 3. Submit to contract
            const tx = await contract.submit_evidence(
                claimId,
                evidenceType,
                cid.toString(),
                Array.from(contentHash),
                size,
                null, // metadata URL (optional)
                file.name // description
            );
            
            await tx.wait();
            
            alert('Evidence submitted successfully!');
        } catch (error) {
            console.error('Error submitting evidence:', error);
            alert('Failed to submit evidence');
        } finally {
            setUploading(false);
        }
    };

    return (
        <div>
            <select value={evidenceType} onChange={e => setEvidenceType(e.target.value)}>
                <option value="photo">Photo</option>
                <option value="document">Document</option>
                <option value="video">Video</option>
                <option value="sensor_data">Sensor Data</option>
            </select>
            
            <input type="file" onChange={e => setFile(e.target.files[0])} />
            
            <button onClick={handleSubmitEvidence} disabled={uploading}>
                {uploading ? 'Uploading...' : 'Submit Evidence'}
            </button>
        </div>
    );
}
```

### Backend Integration

#### Python Service for Evidence Management

```python
from web3 import Web3
import hashlib
import requests

class EvidenceManager:
    def __init__(self, contract_address, ipfs_gateway):
        self.contract = w3.eth.contract(address=contract_address, abi=abi)
        self.ipfs_gateway = ipfs_gateway
    
    def upload_to_ipfs(self, file_path):
        """Upload file to IPFS and get CID"""
        with open(file_path, 'rb') as f:
            files = {'file': f}
            response = requests.post(f'{self.ipfs_gateway}/api/v0/add', files=files)
            result = response.json()
            return result['Hash'], result['Size']
    
    def calculate_content_hash(self, file_path):
        """Calculate SHA-256 hash of file content"""
        sha256_hash = hashlib.sha256()
        with open(file_path, "rb") as f:
            for byte_block in iter(lambda: f.read(4096), b""):
                sha256_hash.update(byte_block)
        return sha256_hash.digest()
    
    def submit_evidence(self, claim_id, file_path, evidence_type, description):
        """Submit evidence to blockchain"""
        # Upload to IPFS
        ipfs_hash, file_size = self.upload_to_ipfs(file_path)
        
        # Calculate content hash
        content_hash = self.calculate_content_hash(file_path)
        
        # Submit transaction
        tx_hash = self.contract.functions.submit_evidence(
            claim_id,
            evidence_type,
            ipfs_hash,
            list(content_hash),
            file_size,
            None,  # metadata URL
            description
        ).transact()
        
        return w3.eth.wait_for_transaction_receipt(tx_hash)
    
    def get_claim_evidence_list(self, claim_id):
        """Get all evidence for a claim"""
        evidence_list = self.contract.functions.get_claim_evidence(claim_id).call()
        
        return [{
            'id': e[0],
            'type': e[2],
            'ipfs_hash': e[3],
            'ipfs_uri': e[4],
            'file_size': e[6],
            'verified': e[8],
            'submitted_at': e[10]
        } for e in evidence_list]
```

## Storage Cost Optimization

### Cost Structure

The system implements a tiered cost structure to optimize storage:

```
Cost = Base Cost + (File Size × Per-Byte Rate) + Verification Bonus
     = 1000 + (file_size × 10) + (500 if verified else 0)
```

### Optimization Strategies

1. **Batch Submissions**: Submit multiple evidence items together to save gas
2. **Appropriate File Sizes**: Use compression for large files
3. **Off-Chain Storage**: Store large files on IPFS, only hashes on-chain
4. **Verification Incentives**: Verified evidence gets cost bonus

### Example Cost Calculations

| Evidence Type | Size | Base | Size Cost | Verified Bonus | Total |
|--------------|------|------|-----------|----------------|-------|
| Photo | 2 MB | 1,000 | 20,971,520 | 500 | 20,973,020 |
| Document | 500 KB | 1,000 | 5,242,880 | 500 | 5,244,380 |
| Video | 50 MB | 1,000 | 524,288,000 | 500 | 524,289,000 |
| Sensor Data | 10 KB | 1,000 | 102,400 | 500 | 103,900 |

## Testing

Run the comprehensive test suite:

```bash
cargo test -p propchain-insurance evidence_tests
```

### Test Coverage

1. **Basic Evidence Submission** ✅
2. **Multiple Evidence Per Claim** ✅
3. **Verification by Assessors** ✅
4. **Consensus Mechanism** ✅
5. **Batch Submission** ✅
6. **Storage Cost Calculation** ✅
7. **Access Control** ✅
8. **Parameter Validation** ✅
9. **Duplicate Prevention** ✅
10. **Total Cost Aggregation** ✅

## Security Considerations

### Access Control

- **Evidence Submission**: Only claimant, assigned assessor, or admin
- **Evidence Verification**: Only authorized assessors or admin
- **Duplicate Prevention**: Same verifier cannot verify twice

### Data Integrity

- **Content Hash Validation**: Ensures file hasn't been tampered
- **IPFS Hash Format**: Validates proper IPFS CID format
- **Verification Audit Trail**: Complete history of all verifications

### Privacy

- **On-Chain Metadata**: Only hashes and URIs stored on-chain
- **Off-Chain Content**: Actual files stored on IPFS
- **Access Control**: Sensitive evidence only visible to authorized parties

## Troubleshooting

### Issue: Evidence Submission Fails

**Common Causes:**
- Empty evidence type
- Invalid IPFS hash format (must start with "Qm" or "bafy")
- Wrong content hash length (must be 32 bytes)
- Unauthorized submitter

**Solution:**
```rust
// Ensure all parameters are valid
let evidence_type = "photo".to_string(); // Not empty
let ipfs_hash = "QmValidHash123".to_string(); // Proper format
let content_hash = calculate_sha256(&file); // Exactly 32 bytes
// Caller must be claimant, assessor, or admin
```

### Issue: Cannot Verify Evidence

**Common Causes:**
- Caller not authorized (not assessor or admin)
- Already verified by same assessor

**Solution:**
```rust
// Ensure caller is registered assessor
contract.register_assessor(assessor_address)?;

// Or use different assessors for multiple verifications
```

### Issue: High Gas Costs

**Solution:**
- Use `batch_submit_evidence()` for multiple items
- Compress large files before uploading
- Consider off-chain storage for very large files

## Future Enhancements

1. **IPFS Pinning Service Integration**: Automatic pinning for important evidence
2. **Encrypted Evidence**: Privacy-preserving evidence storage
3. **Automatic Quality Checks**: AI-powered evidence validation
4. **Evidence Expiration**: Time-limited evidence validity
5. **Dispute Resolution**: Formal challenge process for disputed evidence
6. **Reputation System**: Track assessor verification accuracy

## Conclusion

The Claims Evidence Verification System provides a robust, transparent, and efficient framework for managing insurance claim evidence. By combining IPFS storage, multi-party verification, and gas-optimized operations, it ensures claim integrity while maintaining auditability and cost-effectiveness.
