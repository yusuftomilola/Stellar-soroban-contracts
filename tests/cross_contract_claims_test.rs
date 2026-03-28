use soroban_sdk::{testutils::Address as _, Address, Env};

mod policy { include!("../contracts/policy/src/lib.rs"); }
mod claims { include!("../contracts/claims/src/lib.rs"); }
mod risk_pool { include!("../contracts/risk_pool/src/lib.rs"); }

use policy::{PolicyContract, PolicyContractClient};
use claims::{ClaimsContract, ClaimsContractClient, ClaimError};
use risk_pool::{RiskPoolContract, RiskPoolContractClient};

fn setup(env: &Env) -> (Address, Address, Address, PolicyContractClient, ClaimsContractClient, RiskPoolContractClient) {
    let admin = Address::generate(env);
    let guardian = Address::generate(env);
    let manager = Address::generate(env);

    let pol_addr = env.register_contract(None, PolicyContract);
    let pol = PolicyContractClient::new(env, &pol_addr);
    pol.initialize(&admin, &guardian);

    let clm_addr = env.register_contract(None, ClaimsContract);
    let clm = ClaimsContractClient::new(env, &clm_addr);
    clm.initialize(&admin, &guardian);

    let pool_addr = env.register_contract(None, RiskPoolContract);
    let pool = RiskPoolContractClient::new(env, &pool_addr);
    pool.initialize(&admin, &guardian);

    (pol_addr, admin, guardian, pol, clm, pool)
}

#[test]
fn test_claim_lifecycle_e2e() {
    let env = Env::default();
    env.mock_all_auths();
    let (pol_addr, admin, _guardian, pol, clm, pool) = setup(&env);
    let holder = Address::generate(&env);

    // 1. Issue Policy
    pol.issue_policy(&holder, &1u64, &10_000_000i128, &100_000i128).unwrap();
    assert!(pol.is_policy_active(&1u64));

    // 2. Submit Claim
    clm.submit_claim(&pol_addr, &1u64, &1u64, &5_000_000i128).unwrap();

    // 3. Approve Claim
    clm.approve_claim(&1u64).unwrap();

    // 4. Settle Claim
    clm.settle_claim(&1u64).unwrap();

    // 5. Verify Risk Pool Withdrawal (Simple check if pool logic is there)
    pool.deposit(&holder, &10_000_000i128).unwrap();
    assert_eq!(pool.get_balance(), 10_000_000i128);
    pool.withdraw(&holder, &5_000_000i128).unwrap();
    assert_eq!(pool.get_balance(), 5_000_000i128);
}

#[test]
fn test_emergency_pause_prevents_claims() {
    let env = Env::default();
    env.mock_all_auths();
    let (pol_addr, admin, _guardian, pol, clm, _pool) = setup(&env);
    let holder = Address::generate(&env);

    pol.issue_policy(&holder, &1u64, &10_000_000i128, &100_000i128).unwrap();

    // Pause claims contract
    clm.set_pause_state(&admin, &true, &None).unwrap();
    assert!(clm.is_paused());

    // Try to submit claim
    let result = clm.submit_claim(&pol_addr, &1u64, &1u64, &5_000_000i128);
    assert_eq!(result.unwrap_err(), ClaimError::ContractPaused);

    // Unpause
    clm.set_pause_state(&admin, &false, &None).unwrap();
    assert!(!clm.is_paused());
    assert!(clm.submit_claim(&pol_addr, &1u64, &1u64, &5_000_000i128).is_ok());
}
