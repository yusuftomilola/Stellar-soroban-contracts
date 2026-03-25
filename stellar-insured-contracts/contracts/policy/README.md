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
| `policy_type`     | `Symbol`  | Category (e.g., "Life", "Auto", "Property") |
| `timestamp`       | `u64`     | Ledger epoch timestamp of the action        |

---

### Event Topics & Types

Events are published using a multi-topic format: `(ContractAddress, EventCategory, Action)`.

| Action           | Enum Variant     | Topic 1 (Category) | Topic 2 (Action)     |
| :--------------- | :--------------- | :----------------- | :------------------- |
| **Issuance**     | `PolicyIssued`   | `Symbol("policy")` | `Symbol("issued")`   |
| **Renewal**      | `PolicyRenewed`  | `Symbol("policy")` | `Symbol("renewed")`  |
| **Cancellation** | `PolicyCanceled` | `Symbol("policy")` | `Symbol("canceled")` |
| **Expiration**   | `PolicyExpired`  | `Symbol("policy")` | `Symbol("expired")`  |

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
