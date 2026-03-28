# Policy Contract - Stellar Soroban

This contract manages insurance policy lifecycles on the Stellar network. It utilizes structured, type-safe events to ensure deterministic off-chain monitoring and auditability.

## Event Schema

The contract implements a comprehensive event system using Rust `enums` and `structs`. This allows indexers to parse event data without ambiguity.

### Event Data Structure

All events share a common `PolicyContext` structure:

| Field             | Type      | Description                                 |
| :---------------- | :-------- | :------------------------------------------ |
| `policy_id`       | `u64`     | Unique identifier for the policy            |
| `holder`          | `Address` | Stellar address of the policy holder        |
| `coverage_amount` | `i128`    | Total coverage value (stroops)              |
| `premium_amount`  | `i128`    | Amount paid for the policy                  |
| `duration_days`   | `u32`     | Validity period in days                     |
| `policy_type`     | `Symbol`  | Category (e.g., "STD", "PRM", "ENT")        |
| `timestamp`       | `u64`     | Ledger epoch timestamp of the action        |
| `status`          | `PolicyStatus` | Current status: Active, Renewed, Cancelled, Expired |

---

### Event Topics & Types

Events are published using a multi-topic format: `(ContractAddress, EventCategory, Action)`.

#### 1. Policy Issued Event

**Enum Variant:** `PolicyIssued(PolicyContext)`

**Topics:** `["POLICY", "ISSUED"]`

**Description:** Emitted when a new insurance policy is created and activated.

**Data Fields:**
- `policy_id`: Unique policy identifier
- `holder`: Policy holder's Stellar address
- `coverage_amount`: Total coverage amount in stroops
- `premium_amount`: Premium paid for the policy
- `duration_days`: Policy duration in days
- `policy_type`: Type of policy (e.g., STD for Standard)
- `timestamp`: Block timestamp when issued
- `status`: Set to `Active`

#### 2. Policy Renewed Event

**Enum Variant:** `PolicyRenewed(PolicyContext)`

**Topics:** `["POLICY", "RENEWED"]`

**Description:** Emitted when an existing policy is renewed with updated terms.

**Data Fields:**
- `policy_id`: Unique policy identifier
- `holder`: Policy holder's Stellar address
- `coverage_amount`: Current coverage amount
- `premium_amount`: New premium amount
- `duration_days`: Updated duration in days
- `policy_type`: Policy type indicator
- `timestamp`: Block timestamp when renewed
- `status`: Set to `Renewed`

#### 3. Policy Canceled Event

**Enum Variant:** `PolicyCanceled(PolicyContext, Option<String>)`

**Topics:** `["POLICY", "CANCELED"]`

**Description:** Emitted when a policy is canceled before its expiration.

**Data Fields:**
- `policy_id`: Unique policy identifier
- `holder`: Policy holder's Stellar address
- `coverage_amount`: Coverage amount at cancellation
- `premium_amount`: Premium amount at cancellation
- `duration_days`: Original duration
- `policy_type`: Set to "CANCEL"
- `timestamp`: Block timestamp when canceled
- `status`: Set to `Cancelled`
- `cancellation_reason`: Optional reason for cancellation

#### 4. Policy Expired Event

**Enum Variant:** `PolicyExpired(PolicyContext)`

**Topics:** `["POLICY", "EXPIRED"]`

**Description:** Emitted when a policy reaches its natural expiration.

**Data Fields:**
- `policy_id`: Unique policy identifier
- `holder`: Policy holder's Stellar address
- `coverage_amount`: Final coverage amount
- `premium_amount`: Final premium amount
- `duration_days`: Original duration
- `policy_type`: Policy type indicator
- `timestamp`: Block timestamp when expired
- `status`: Set to `Expired`

---

## 🛠 Usage for Off-Chain Monitoring

Because the events use the `#[contracttype]` macro, they are ASIC-friendly. To parse these events from a Mercury or Horizon stream, look for the following signature:

**Example Event Signature (JSON):**

```json
{
  "type": "contract",
  "topics": ["policy", "issued"],
  "data": {
    "policy_id": 1001,
    "holder": "G...",
    "coverage_amount": "5000000000",
    "premium_amount": "100000000",
    "duration_days": 365,
    "policy_type": "Life",
    "timestamp": 1709123456
  }
}
```
