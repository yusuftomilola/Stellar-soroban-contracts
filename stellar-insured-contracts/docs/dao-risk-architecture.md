# DAO Risk Architecture

This document defines a transparent, auditable risk framework for the insurance and Soroban governance stack. It is intended for DAO operators, external auditors, and dashboard builders who need consistent formulae, thresholds, and reference scenarios.

## Purpose

The framework answers four recurring questions:

1. How much insured risk is the pool carrying right now?
2. How much of the pool's capital is already committed to coverage?
3. When should governance or slashing controls escalate?
4. How close is the pool to a liquidity bottleneck?

The metrics below are designed to be simple enough for on-chain and off-chain reproduction while still being meaningful for operational oversight.

## Core Inputs

Use the following normalized inputs per pool and per reporting interval:

| Field | Meaning |
|------|---------|
| `total_capital` | Total principal supplied by liquidity providers |
| `available_capital` | Liquid capital available for claims and withdrawals |
| `reserved_for_claims` | Capital earmarked for approved but not yet finalized obligations |
| `active_coverage` | Sum of remaining coverage across active policies |
| `active_policies` | Count of active policies |
| `total_claims_paid` | Aggregate paid claims |
| `claims_pending_amount` | Sum of pending or under-review claim amounts |
| `claims_approved_amount` | Sum of approved claim amounts awaiting settlement |
| `utilization_cap_bps` | DAO-configured maximum safe utilization in basis points |
| `governance_breach_count` | Number of consecutive reporting windows in breach state |
| `oracle_confidence` | Confidence score in `[0, 1]` for external risk inputs |
| `policy_risk_score` | Weighted average underwriting risk score in `[0, 100]` |

## Formulae

### 1. Risk Exposure

`risk_exposure` measures how much net insured obligation the pool carries against usable capital.

Formula:

```text
effective_liquidity = max(available_capital - reserved_for_claims, 0)
risk_exposure = active_coverage / max(effective_liquidity, 1)
```

Interpretation:

- `< 0.80` means the pool is conservatively capitalized
- `0.80 - 1.00` means the pool is approaching its operating ceiling
- `> 1.00` means insured exposure exceeds liquid backing and governance action is required

### 2. Coverage Ratio

`coverage_ratio` is the solvency-style metric used for dashboards and audits.

Formula:

```text
coverage_ratio = available_capital / max(active_coverage, 1)
```

Interpretation:

- `> 1.25` is healthy
- `1.00 - 1.25` is watch
- `< 1.00` indicates under-collateralized live coverage

Note:

`coverage_ratio` and `risk_exposure` are inverse-style views, but both are useful because operators often think in "coverage backing" while risk committees think in "exposure pressure."

### 3. Slashing Score

`slashing_score` estimates how severe operator or governance underperformance is. It should not directly slash stake by itself; instead it should feed governance thresholds and incident review.

Recommended normalized components:

```text
exposure_component = clamp((risk_exposure - 1.00) / 0.50, 0, 1)
utilization_component = clamp(liquidity_utilization / 0.95, 0, 1)
pending_claims_component = clamp(claims_pending_amount / max(total_capital, 1), 0, 1)
governance_breach_component = clamp(governance_breach_count / 3, 0, 1)
oracle_component = 1 - oracle_confidence
underwriting_component = clamp(policy_risk_score / 100, 0, 1)

slashing_score =
  0.30 * exposure_component +
  0.20 * utilization_component +
  0.15 * pending_claims_component +
  0.15 * governance_breach_component +
  0.10 * oracle_component +
  0.10 * underwriting_component
```

Interpretation:

- `0.00 - 0.30` healthy
- `0.30 - 0.60` warning
- `0.60 - 0.80` severe
- `> 0.80` slash-eligible or emergency governance review

### 4. Liquidity Utilization

`liquidity_utilization` measures how much of total capital is already consumed by obligations or effectively locked.

Formula:

```text
capital_committed = reserved_for_claims + claims_approved_amount + (total_capital - available_capital)
liquidity_utilization = capital_committed / max(total_capital, 1)
```

Equivalent simplified form:

```text
liquidity_utilization =
  (reserved_for_claims + claims_approved_amount + total_capital - available_capital)
  / max(total_capital, 1)
```

Interpretation:

- `< 0.60` comfortable
- `0.60 - 0.80` elevated
- `0.80 - 0.90` stressed
- `> 0.90` critical

## Safe Operating Ranges

| Metric | Healthy | Watch | Critical |
|------|---------|-------|----------|
| `risk_exposure` | `< 0.80` | `0.80 - 1.00` | `> 1.00` |
| `coverage_ratio` | `> 1.25` | `1.00 - 1.25` | `< 1.00` |
| `slashing_score` | `< 0.30` | `0.30 - 0.60` | `> 0.80` |
| `liquidity_utilization` | `< 0.60` | `0.60 - 0.80` | `> 0.90` |

Recommended DAO actions:

- Healthy: permit normal policy issuance and liquidity withdrawals
- Watch: slow issuance, require enhanced review for large policies
- Severe: require governance acknowledgment before adding new exposure
- Critical: pause new policies, restrict withdrawals, escalate to emergency governance

## Pseudocode

```text
function clamp(value, min_value, max_value):
    if value < min_value:
        return min_value
    if value > max_value:
        return max_value
    return value

function compute_pool_risk(inputs):
    effective_liquidity = max(inputs.available_capital - inputs.reserved_for_claims, 0)
    risk_exposure = inputs.active_coverage / max(effective_liquidity, 1)
    coverage_ratio = inputs.available_capital / max(inputs.active_coverage, 1)

    capital_committed =
        inputs.reserved_for_claims +
        inputs.claims_approved_amount +
        (inputs.total_capital - inputs.available_capital)
    liquidity_utilization = capital_committed / max(inputs.total_capital, 1)

    exposure_component = clamp((risk_exposure - 1.00) / 0.50, 0, 1)
    utilization_component = clamp(liquidity_utilization / 0.95, 0, 1)
    pending_claims_component = clamp(inputs.claims_pending_amount / max(inputs.total_capital, 1), 0, 1)
    governance_breach_component = clamp(inputs.governance_breach_count / 3, 0, 1)
    oracle_component = 1 - inputs.oracle_confidence
    underwriting_component = clamp(inputs.policy_risk_score / 100, 0, 1)

    slashing_score =
        0.30 * exposure_component +
        0.20 * utilization_component +
        0.15 * pending_claims_component +
        0.15 * governance_breach_component +
        0.10 * oracle_component +
        0.10 * underwriting_component

    if risk_exposure > 1.00 or coverage_ratio < 1.00 or liquidity_utilization > 0.90:
        operating_state = "critical"
    else if slashing_score >= 0.60 or liquidity_utilization >= 0.80:
        operating_state = "severe"
    else if risk_exposure >= 0.80 or coverage_ratio <= 1.25:
        operating_state = "watch"
    else:
        operating_state = "healthy"

    return {
        risk_exposure,
        coverage_ratio,
        slashing_score,
        liquidity_utilization,
        operating_state
    }
```

## Worked Numeric Scenarios

### Scenario A: Safe Range

Inputs:

- `total_capital = 10,000,000`
- `available_capital = 8,500,000`
- `reserved_for_claims = 500,000`
- `active_coverage = 5,000,000`
- `claims_pending_amount = 300,000`
- `claims_approved_amount = 200,000`
- `governance_breach_count = 0`
- `oracle_confidence = 0.96`
- `policy_risk_score = 42`

Results:

```text
effective_liquidity = 8,500,000 - 500,000 = 8,000,000
risk_exposure = 5,000,000 / 8,000,000 = 0.625
coverage_ratio = 8,500,000 / 5,000,000 = 1.70
capital_committed = 500,000 + 200,000 + (10,000,000 - 8,500,000) = 2,200,000
liquidity_utilization = 2,200,000 / 10,000,000 = 0.22
slashing_score approx 0.10
```

Assessment:

- Healthy across all dimensions
- New policies may be issued normally
- LP withdrawals can remain open

### Scenario B: Watch Range

Inputs:

- `total_capital = 10,000,000`
- `available_capital = 6,800,000`
- `reserved_for_claims = 1,000,000`
- `active_coverage = 5,600,000`
- `claims_pending_amount = 900,000`
- `claims_approved_amount = 600,000`
- `governance_breach_count = 1`
- `oracle_confidence = 0.90`
- `policy_risk_score = 58`

Results:

```text
effective_liquidity = 6,800,000 - 1,000,000 = 5,800,000
risk_exposure = 5,600,000 / 5,800,000 = 0.97
coverage_ratio = 6,800,000 / 5,600,000 = 1.21
capital_committed = 1,000,000 + 600,000 + 3,200,000 = 4,800,000
liquidity_utilization = 4,800,000 / 10,000,000 = 0.48
slashing_score approx 0.28
```

Assessment:

- Still solvent, but entering watch range
- Governance should review issuance velocity and pricing
- Large claims should require enhanced approval scrutiny

### Scenario C: Critical Range

Inputs:

- `total_capital = 10,000,000`
- `available_capital = 3,600,000`
- `reserved_for_claims = 1,400,000`
- `active_coverage = 4,600,000`
- `claims_pending_amount = 1,100,000`
- `claims_approved_amount = 900,000`
- `governance_breach_count = 3`
- `oracle_confidence = 0.72`
- `policy_risk_score = 83`

Results:

```text
effective_liquidity = 3,600,000 - 1,400,000 = 2,200,000
risk_exposure = 4,600,000 / 2,200,000 = 2.09
coverage_ratio = 3,600,000 / 4,600,000 = 0.78
capital_committed = 1,400,000 + 900,000 + 6,400,000 = 8,700,000
liquidity_utilization = 8,700,000 / 10,000,000 = 0.87
slashing_score approx 0.79
```

Assessment:

- Exposure exceeds liquid support
- Coverage ratio is below parity
- DAO should pause new underwriting and route incident review to governance immediately

## Dashboard Dataset

A lightweight example dataset lives at:

[risk_dashboard_dataset.json](C:\Users\hp\Desktop\wave\Stellar-soroban-contracts\stellar-insured-contracts\docs\risk_dashboard_dataset.json)

Suggested update cadence:

- Per block for core pool balances if an indexer is available
- Every 5-15 minutes for DAO dashboard summaries
- End-of-day snapshots for external audit exports

## Recommended Dashboard Metrics

Primary tiles:

- `risk_exposure`
- `coverage_ratio`
- `slashing_score`
- `liquidity_utilization`
- `available_capital`
- `active_coverage`

Supporting metrics:

- `reserved_for_claims`
- `claims_pending_amount`
- `claims_approved_amount`
- `active_policies`
- `policy_risk_score`
- `oracle_confidence`
- `governance_breach_count`
- `new_policies_24h`
- `claims_submitted_24h`
- `claims_paid_24h`
- `withdrawal_requests_24h`

Recommended alert rules:

- Alert if `risk_exposure > 1.00`
- Alert if `coverage_ratio < 1.00`
- Alert if `liquidity_utilization > 0.90`
- Alert if `slashing_score >= 0.60` for two consecutive windows
- Alert if `oracle_confidence < 0.80`

## External Audit Notes

To keep this model audit-friendly:

- Keep units explicit in every export
- Publish the formula version used by the dashboard
- Store the raw inputs next to computed outputs
- Recompute metrics off-chain from emitted events where possible
- Record threshold changes by governance proposal id

## Versioning

Initial formula version: `dao-risk-v1`

If the DAO changes weights or thresholds, increment the formula version and publish a migration note so historic dashboards remain reproducible.
