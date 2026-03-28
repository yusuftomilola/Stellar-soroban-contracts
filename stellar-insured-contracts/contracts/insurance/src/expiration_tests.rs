#![cfg(test)]
use super::*;
use ink::env::DefaultEnvironment;

type AccountId = <DefaultEnvironment as ink::env::Environment>::AccountId;

#[ink::test]
fn test_check_and_expire_policies_basic() {
    // Setup test environment
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;

    // Set caller to policyholder
    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);

    // Create contract instance
    let mut contract = PropertyInsurance::new(admin);

    // Create a risk pool first
    let pool_id = 1;
    let coverage_type = CoverageType::Fire;
    let _ = contract.create_risk_pool(
        coverage_type.clone(),
        1_000_000_000_000, // 1M capital
        8000,              // 80% max coverage
        100_000_000_000,   // Reinsurance threshold
        "Test Pool".to_string(),
    );

    // Create a risk assessment
    let property_id = 1;
    let assessment = RiskAssessment {
        property_id,
        location_risk_score: 30,
        construction_risk_score: 20,
        age_risk_score: 25,
        claims_history_score: 10,
        overall_risk_score: 21,
        risk_level: RiskLevel::Low,
        assessed_at: contract.env().block_timestamp(),
        valid_until: contract.env().block_timestamp() + 365 * 24 * 60 * 60, // 1 year
    };
    contract.risk_assessments.insert(&property_id, &assessment);

    // Create a policy with short duration (1 hour for testing)
    let duration = 3600; // 1 hour
    let coverage_amount = 100_000_000_000; // 100k
    let premium = 1_000_000_000; // 1k premium

    ink::env::test::set_value_transferred::<DefaultEnvironment>(premium);
    
    let policy_id = contract.create_policy(
        property_id,
        CoverageType::Fire,
        coverage_amount,
        pool_id,
        duration,
        "ipfs://test".to_string(),
    ).expect("Should create policy");

    // Verify policy is active
    let policy = contract.get_policy(policy_id).unwrap();
    assert_eq!(policy.status, PolicyStatus::Active);

    // Check active policies count
    assert_eq!(contract.get_active_policies_count(), 1);

    // Try to expire immediately (should not expire yet)
    let expired = contract.check_and_expire_policies(10).unwrap();
    assert_eq!(expired, 0);

    // Advance time beyond policy end time
    ink::env::test::advance_block_time::<DefaultEnvironment>(duration + 100);

    // Now check and expire
    let expired = contract.check_and_expire_policies(10).unwrap();
    assert_eq!(expired, 1);

    // Verify policy is expired
    let policy = contract.get_policy(policy_id).unwrap();
    assert_eq!(policy.status, PolicyStatus::Expired);

    // Check active policies count should be 0
    assert_eq!(contract.get_active_policies_count(), 0);
}

#[ink::test]
fn test_pagination_large_policy_set() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);

    // Create pool
    let pool_id = 1;
    let _ = contract.create_risk_pool(
        CoverageType::Fire,
        10_000_000_000_000,
        8000,
        1_000_000_000_000,
        "Large Test Pool".to_string(),
    );

    // Create risk assessment
    let assessment = RiskAssessment {
        property_id: 1,
        location_risk_score: 30,
        construction_risk_score: 20,
        age_risk_score: 25,
        claims_history_score: 10,
        overall_risk_score: 21,
        risk_level: RiskLevel::Low,
        assessed_at: contract.env().block_timestamp(),
        valid_until: contract.env().block_timestamp() + 365 * 24 * 60 * 60,
    };
    contract.risk_assessments.insert(&1, &assessment);

    // Create 50 policies with different durations
    let num_policies = 50;
    let batch_size = 10;
    let mut policy_ids = Vec::new();

    for i in 0..num_policies {
        let property_id = i + 1;
        contract.risk_assessments.insert(&property_id, &assessment);

        let duration = if i < 25 { 3600 } else { 7200 }; // First 25 expire soon, rest later
        let premium = 1_000_000_000;

        ink::env::test::set_value_transferred::<DefaultEnvironment>(premium);
        
        let policy_id = contract.create_policy(
            property_id,
            CoverageType::Fire,
            100_000_000_000,
            pool_id,
            duration,
            format!("ipfs://test{}", i),
        ).expect("Should create policy");

        policy_ids.push(policy_id);
    }

    // Verify all policies are active
    assert_eq!(contract.get_active_policies_count(), num_policies as u64);

    // Advance time to expire first batch
    ink::env::test::advance_block_time::<DefaultEnvironment>(3700);

    // Process in batches
    let mut total_expired = 0u64;
    let mut iterations = 0;
    
    loop {
        let expired = contract.check_and_expire_policies(batch_size).unwrap();
        total_expired += expired;
        iterations += 1;

        if expired == 0 || iterations > 10 {
            break;
        }
    }

    // Should have expired 25 policies
    assert_eq!(total_expired, 25);

    // Remaining active policies should be 25
    assert_eq!(contract.get_active_policies_count(), 25);
}

#[ink::test]
fn test_get_active_policies_pagination() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);

    // Create pool and assessment
    let _ = contract.create_risk_pool(
        CoverageType::Fire,
        10_000_000_000_000,
        8000,
        1_000_000_000_000,
        "Test Pool".to_string(),
    );

    let assessment = RiskAssessment {
        property_id: 1,
        location_risk_score: 30,
        construction_risk_score: 20,
        age_risk_score: 25,
        claims_history_score: 10,
        overall_risk_score: 21,
        risk_level: RiskLevel::Low,
        assessed_at: contract.env().block_timestamp(),
        valid_until: contract.env().block_timestamp() + 365 * 24 * 60 * 60,
    };

    // Create 20 policies
    for i in 0..20 {
        let property_id = i + 1;
        contract.risk_assessments.insert(&property_id, &assessment);

        ink::env::test::set_value_transferred::<DefaultEnvironment>(1_000_000_000);
        
        contract.create_policy(
            property_id,
            CoverageType::Fire,
            100_000_000_000,
            1,
            86400, // 1 day
            format!("ipfs://test{}", i),
        ).unwrap();
    }

    // Test pagination - get first 10
    let page1 = contract.get_active_policies(0, 10);
    assert_eq!(page1.len(), 10);

    // Get next 10
    let page2 = contract.get_active_policies(10, 10);
    assert_eq!(page2.len(), 10);

    // Ensure no overlap
    for id1 in &page1 {
        for id2 in &page2 {
            assert_ne!(id1, id2);
        }
    }

    // Get all at once
    let all = contract.get_active_policies(0, 100);
    assert_eq!(all.len(), 20);
}

#[ink::test]
fn test_get_expiring_soon_policies() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);

    // Setup
    let _ = contract.create_risk_pool(
        CoverageType::Fire,
        10_000_000_000_000,
        8000,
        1_000_000_000_000,
        "Test Pool".to_string(),
    );

    let assessment = RiskAssessment {
        property_id: 1,
        location_risk_score: 30,
        construction_risk_score: 20,
        age_risk_score: 25,
        claims_history_score: 10,
        overall_risk_score: 21,
        risk_level: RiskLevel::Low,
        assessed_at: contract.env().block_timestamp(),
        valid_until: contract.env().block_timestamp() + 365 * 24 * 60 * 60,
    };

    // Create policies with different expiration times
    for i in 0..10 {
        let property_id = i + 1;
        contract.risk_assessments.insert(&property_id, &assessment);

        let duration = if i < 5 { 3600 } else { 86400 }; // 5 expiring soon, 5 later

        ink::env::test::set_value_transferred::<DefaultEnvironment>(1_000_000_000);
        
        contract.create_policy(
            property_id,
            CoverageType::Fire,
            100_000_000_000,
            1,
            duration,
            format!("ipfs://test{}", i),
        ).unwrap();
    }

    // Get policies expiring within next 2 hours
    let expiring_soon = contract.get_expiring_soon_policies(7200, 0, 100);
    assert_eq!(expiring_soon.len(), 5);

    // Advance 1 hour - first batch should now be expired
    ink::env::test::advance_block_time::<DefaultEnvironment>(3700);

    // Expire them
    contract.check_and_expire_policies(100).unwrap();

    // Now get policies expiring within next 24 hours
    let expiring_soon = contract.get_expiring_soon_policies(86400, 0, 100);
    assert_eq!(expiring_soon.len(), 5); // Only the second batch remains
}

#[ink::test]
fn test_get_policy_expiration_info() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);

    // Setup
    let _ = contract.create_risk_pool(
        CoverageType::Fire,
        1_000_000_000_000,
        8000,
        100_000_000_000,
        "Test Pool".to_string(),
    );

    let assessment = RiskAssessment {
        property_id: 1,
        location_risk_score: 30,
        construction_risk_score: 20,
        age_risk_score: 25,
        claims_history_score: 10,
        overall_risk_score: 21,
        risk_level: RiskLevel::Low,
        assessed_at: contract.env().block_timestamp(),
        valid_until: contract.env().block_timestamp() + 365 * 24 * 60 * 60,
    };
    contract.risk_assessments.insert(&1, &assessment);

    let duration = 86400; // 1 day
    ink::env::test::set_value_transferred::<DefaultEnvironment>(1_000_000_000);
    
    let policy_id = contract.create_policy(
        1,
        CoverageType::Fire,
        100_000_000_000,
        1,
        duration,
        "ipfs://test".to_string(),
    ).unwrap();

    // Get expiration info
    let info = contract.get_policy_expiration_info(policy_id).unwrap();
    let (start_time, end_time, time_remaining, is_expired) = info;

    assert_eq!(start_time, contract.env().block_timestamp());
    assert_eq!(end_time, start_time + duration);
    assert!(time_remaining <= duration);
    assert!(!is_expired);

    // Advance time beyond expiration
    ink::env::test::advance_block_time::<DefaultEnvironment>(duration + 100);

    // Check again
    let info = contract.get_policy_expiration_info(policy_id).unwrap();
    let (_, _, time_remaining, is_expired) = info;

    assert_eq!(time_remaining, 0);
    assert!(is_expired);
}

#[ink::test]
fn test_manually_expire_policy() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);

    // Setup
    let _ = contract.create_risk_pool(
        CoverageType::Fire,
        1_000_000_000_000,
        8000,
        100_000_000_000,
        "Test Pool".to_string(),
    );

    let assessment = RiskAssessment {
        property_id: 1,
        location_risk_score: 30,
        construction_risk_score: 20,
        age_risk_score: 25,
        claims_history_score: 10,
        overall_risk_score: 21,
        risk_level: RiskLevel::Low,
        assessed_at: contract.env().block_timestamp(),
        valid_until: contract.env().block_timestamp() + 365 * 24 * 60 * 60,
    };
    contract.risk_assessments.insert(&1, &assessment);

    ink::env::test::set_value_transferred::<DefaultEnvironment>(1_000_000_000);
    
    let policy_id = contract.create_policy(
        1,
        CoverageType::Fire,
        100_000_000_000,
        1,
        86400,
        "ipfs://test".to_string(),
    ).unwrap();

    // Verify policy is active
    let policy = contract.get_policy(policy_id).unwrap();
    assert_eq!(policy.status, PolicyStatus::Active);

    // Switch to admin and manually expire
    ink::env::test::set_caller::<DefaultEnvironment>(admin);
    contract.manually_expire_policy(policy_id).unwrap();

    // Verify policy is expired
    let policy = contract.get_policy(policy_id).unwrap();
    assert_eq!(policy.status, PolicyStatus::Expired);
    assert_eq!(contract.get_active_policies_count(), 0);
}

#[ink::test]
fn test_manual_expire_unauthorized() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let unauthorized = accounts.bob;

    ink::env::test::set_caller::<DefaultEnvironment>(unauthorized);
    let mut contract = PropertyInsurance::new(admin);

    // Try to manually expire as non-admin (should fail)
    let result = contract.manually_expire_policy(1);
    assert_eq!(result, Err(InsuranceError::Unauthorized));
}

#[ink::test]
fn test_gas_optimization_batch_processing() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);

    // Setup
    let _ = contract.create_risk_pool(
        CoverageType::Fire,
        100_000_000_000_000,
        8000,
        10_000_000_000_000,
        "Gas Test Pool".to_string(),
    );

    let assessment = RiskAssessment {
        property_id: 1,
        location_risk_score: 30,
        construction_risk_score: 20,
        age_risk_score: 25,
        claims_history_score: 10,
        overall_risk_score: 21,
        risk_level: RiskLevel::Low,
        assessed_at: contract.env().block_timestamp(),
        valid_until: contract.env().block_timestamp() + 365 * 24 * 60 * 60,
    };

    // Create 100 policies
    for i in 0..100 {
        let property_id = i + 1;
        contract.risk_assessments.insert(&property_id, &assessment);

        ink::env::test::set_value_transferred::<DefaultEnvironment>(1_000_000_000);
        
        contract.create_policy(
            property_id,
            CoverageType::Fire,
            100_000_000_000,
            1,
            3600, // All expire in 1 hour
            format!("ipfs://test{}", i),
        ).unwrap();
    }

    // Advance time
    ink::env::test::advance_block_time::<DefaultEnvironment>(3700);

    // Process with different batch sizes and measure iterations
    let batch_sizes = [10, 20, 50];
    
    for batch_size in batch_sizes {
        // Reset contract state by creating new instance would be ideal,
        // but for this test we'll just verify the batch processing works
        let expired = contract.check_and_expire_policies(batch_size).unwrap();
        
        // Verify batch size is respected
        assert!(expired <= batch_size);
    }
}
