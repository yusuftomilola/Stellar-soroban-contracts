# Policy Expiration Checker - Quick Reference

## Quick Start

### 1. Check and Expire Policies (Anyone Can Call)

```rust
// Process up to 50 policies in one transaction
let expired = contract.check_and_expire_policies(50)?;
println!("Expired {} policies", expired);
```

### 2. Get Active Policies

```rust
// Get first 20 active policies
let policies = contract.get_active_policies(0, 20);
```

### 3. Check Policy Expiration Status

```rust
if let Some((start, end, remaining, expired)) = 
    contract.get_policy_expiration_info(policy_id) {
    println!("Expired: {}", expired);
    println!("Time remaining: {} seconds", remaining);
}
```

## Function Cheat Sheet

| Function | Who Can Call | Description | Returns |
|----------|-------------|-------------|---------|
| `check_and_expire_policies(n)` | Anyone | Expires up to n policies | Count expired |
| `get_active_policies(start, limit)` | Anyone | List active policy IDs | Vec<policy_id> |
| `get_active_policies_count()` | Anyone | Total active policies | Count |
| `get_expiring_soon_policies(sec, start, limit)` | Anyone | Policies expiring soon | Vec<policy_id> |
| `get_policy_expiration_info(id)` | Anyone | Detailed expiration info | (start, end, remaining, expired) |
| `manually_expire_policy(id)` | Admin only | Force expire policy | () |

## Common Patterns

### Run Periodically (Bot/Script)

```rust
// Run every hour
fn run_expiration_check() {
    loop {
        let expired = contract.check_and_expire_policies(100).unwrap();
        if expired == 0 { break; } // All done
    }
}
```

### Find Policies Expiring Soon

```rust
// Get policies expiring in next 7 days
let seven_days = 7 * 24 * 3600;
let expiring = contract.get_expiring_soon_policies(seven_days, 0, 100);
```

### Paginate Through Large Sets

```rust
let page_size = 50;
let mut page = 0;

loop {
    let policies = contract.get_active_policies(page * page_size, page_size);
    if policies.is_empty() { break; }
    
    // Process page...
    page += 1;
}
```

## Events to Monitor

### PolicyExpired
```typescript
contract.on('PolicyExpired', (policyId, holder, expiredAt, endTime) => {
  console.log(`Policy ${policyId} expired`);
});
```

### PoliciesExpirationChecked
```typescript
contract.on('PoliciesExpirationChecked', (checked, expired, nextIdx, ts) => {
  console.log(`Processed ${checked}, expired ${expired}, resume at ${nextIdx}`);
});
```

## Recommended Batch Sizes

| Network | Batch Size | Frequency |
|---------|-----------|-----------|
| Ethereum L1 | 10-20 | Daily |
| Arbitrum/Optimism | 50-100 | Hourly |
| High-throughput | 100-500 | Every 30 min |

## Testing Commands

```bash
# Run all expiration tests
cargo test -p propchain-insurance expiration_tests

# Run specific test
cargo test -p propchain-insurance test_check_and_expire_policies_basic

# Run with output
cargo test -p propchain-insurance -- --nocapture
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Not all policies expired | Call function again or increase batch_size |
| Gas too high | Reduce batch_size, run more frequently |
| Policy not found | Check if already expired or cancelled |
| Can't manually expire | Must be admin |

## Integration Checklist

- [ ] Add event listeners for `PolicyExpired`
- [ ] Set up automated bot to call `check_and_expire_policies()`
- [ ] Update UI to show expiration status
- [ ] Configure monitoring/alerts for large batches
- [ ] Test with small batch sizes first

## Storage Layout

```rust
active_policy_indexes: Vec<u64>,              // All active policy IDs
last_expiration_check_index: u64,             // Pagination state
```

## Best Practices

1. ✅ Run expiration checks regularly (hourly/daily)
2. ✅ Use appropriate batch sizes for your network
3. ✅ Monitor events to track progress
4. ✅ Implement retry logic for failed transactions
5. ✅ Alert on unusually large batches (may indicate issue)

## Security Notes

- 🔓 **Permissionless**: Anyone can call expiration functions
- 🔒 **Safe**: Only changes Active → Expired status
- ⚠️ **Irreversible**: Can't un-expire a policy
- 💰 **No Incentive**: No economic value in manipulating expirations

---

**Full Documentation**: See [POLICY_EXPIRATION_CHECKER.md](../docs/POLICY_EXPIRATION_CHECKER.md)
