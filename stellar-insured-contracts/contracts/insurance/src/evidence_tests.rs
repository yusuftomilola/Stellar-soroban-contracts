#![cfg(test)]
use super::*;
use ink::env::DefaultEnvironment;

type AccountId = <DefaultEnvironment as ink::env::Environment>::AccountId;

#[ink::test]
fn test_submit_evidence_basic() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);

    // Setup: Create pool, assessment, and policy
    setup_test_environment(&mut contract, &policyholder);

    // Submit a claim first
    let evidence_metadata = EvidenceMetadata {
        evidence_type: "photo".to_string(),
        reference_uri: "ipfs://QmPrimary123".to_string(),
        content_hash: vec![1u8; 32],
        description: Some("Primary damage photo".to_string()),
    };

    let claim_id = contract.submit_claim(
        1, // property_id
        50_000_000_000, // claim_amount
        "Fire damage claim".to_string(),
        evidence_metadata,
    ).unwrap();

    // Submit additional evidence
    let ipfs_hash = "QmEvidence456".to_string();
    let content_hash = vec![2u8; 32];
    
    let evidence_id = contract.submit_evidence(
        claim_id,
        "document".to_string(),
        ipfs_hash.clone(),
        content_hash.clone(),
        1024 * 500, // 500 KB file
        Some("ipfs://QmMetadata789".to_string()),
        Some("Repair estimate document".to_string()),
    ).unwrap();

    assert_eq!(evidence_id, 1);

    // Verify evidence was stored correctly
    let evidence = contract.get_evidence(evidence_id).unwrap();
    assert_eq!(evidence.claim_id, claim_id);
    assert_eq!(evidence.evidence_type, "document");
    assert_eq!(evidence.ipfs_hash, ipfs_hash);
    assert_eq!(evidence.content_hash, content_hash);
    assert_eq!(evidence.file_size, 1024 * 500);
    assert!(!evidence.verified);
}

#[ink::test]
fn test_submit_multiple_evidence_per_claim() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);
    setup_test_environment(&mut contract, &policyholder);

    // Submit claim
    let claim_id = contract.submit_claim(
        1,
        50_000_000_000,
        "Damage claim".to_string(),
        EvidenceMetadata {
            evidence_type: "photo".to_string(),
            reference_uri: "ipfs://QmPrimary".to_string(),
            content_hash: vec![1u8; 32],
            description: None,
        },
    ).unwrap();

    // Submit multiple pieces of evidence
    let evidence1 = contract.submit_evidence(
        claim_id,
        "photo".to_string(),
        "QmPhoto1".to_string(),
        vec![1u8; 32],
        1024 * 1024 * 2, // 2 MB
        None,
        Some("Front view".to_string()),
    ).unwrap();

    let evidence2 = contract.submit_evidence(
        claim_id,
        "photo".to_string(),
        "QmPhoto2".to_string(),
        vec![2u8; 32],
        1024 * 1024 * 3, // 3 MB
        None,
        Some("Back view".to_string()),
    ).unwrap();

    let evidence3 = contract.submit_evidence(
        claim_id,
        "video".to_string(),
        "QmVideo123".to_string(),
        vec![3u8; 32],
        1024 * 1024 * 50, // 50 MB
        None,
        Some("Walkthrough video".to_string()),
    ).unwrap();

    // Verify all evidence attached to claim
    let all_evidence = contract.get_claim_evidence(claim_id);
    assert_eq!(all_evidence.len(), 3);
    assert_eq!(all_evidence[0].id, evidence1);
    assert_eq!(all_evidence[1].id, evidence2);
    assert_eq!(all_evidence[2].id, evidence3);
}

#[ink::test]
fn test_verify_evidence_by_assessor() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;
    let assessor = accounts.charlie;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);
    setup_test_environment(&mut contract, &policyholder);

    // Register assessor
    ink::env::test::set_caller::<DefaultEnvironment>(admin);
    contract.register_assessor(assessor).unwrap();

    // Submit claim and evidence
    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let claim_id = contract.submit_claim(
        1,
        50_000_000_000,
        "Claim".to_string(),
        EvidenceMetadata {
            evidence_type: "photo".to_string(),
            reference_uri: "ipfs://QmTest".to_string(),
            content_hash: vec![1u8; 32],
            description: None,
        },
    ).unwrap();

    let evidence_id = contract.submit_evidence(
        claim_id,
        "document".to_string(),
        "QmDoc123".to_string(),
        vec![1u8; 32],
        1024 * 100,
        None,
        None,
    ).unwrap();

    // Assessor verifies evidence
    ink::env::test::set_caller::<DefaultEnvironment>(assessor);
    contract.verify_evidence(
        evidence_id,
        true,
        "Document appears authentic".to_string(),
    ).unwrap();

    // Verify evidence status updated
    let evidence = contract.get_evidence(evidence_id).unwrap();
    assert!(evidence.verified);
    assert_eq!(evidence.verified_by, Some(assessor));

    // Check verification records
    let verifications = contract.get_evidence_verifications(evidence_id);
    assert_eq!(verifications.len(), 1);
    assert!(verifications[0].is_valid);
    assert_eq!(verifications[0].verifier, assessor);
}

#[ink::test]
fn test_evidence_verification_consensus() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;
    let assessor1 = accounts.charlie;
    let assessor2 = accounts.david;
    let assessor3 = accounts.eve;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);
    setup_test_environment(&mut contract, &policyholder);

    // Register multiple assessors
    ink::env::test::set_caller::<DefaultEnvironment>(admin);
    contract.register_assessor(assessor1).unwrap();
    contract.register_assessor(assessor2).unwrap();
    contract.register_assessor(assessor3).unwrap();

    // Submit claim and evidence
    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let claim_id = contract.submit_claim(
        1,
        50_000_000_000,
        "Claim".to_string(),
        EvidenceMetadata::default(),
    ).unwrap();

    let evidence_id = contract.submit_evidence(
        claim_id,
        "photo".to_string(),
        "QmPhoto".to_string(),
        vec![1u8; 32],
        1024 * 200,
        None,
        None,
    ).unwrap();

    // Multiple assessors verify with different opinions
    ink::env::test::set_caller::<DefaultEnvironment>(assessor1);
    contract.verify_evidence(evidence_id, true, "Valid".to_string()).unwrap();

    ink::env::test::set_caller::<DefaultEnvironment>(assessor2);
    contract.verify_evidence(evidence_id, false, "Suspicious".to_string()).unwrap();

    ink::env::test::set_caller::<DefaultEnvironment>(assessor3);
    contract.verify_evidence(evidence_id, true, "Authentic".to_string()).unwrap();

    // Check consensus (2 valid vs 1 invalid = valid consensus)
    let status = contract.get_evidence_verification_status(evidence_id).unwrap();
    let (total, valid, invalid, consensus) = status;
    
    assert_eq!(total, 3);
    assert_eq!(valid, 2);
    assert_eq!(invalid, 1);
    assert!(consensus); // Majority says valid

    // Check is_evidence_verified returns true
    assert!(contract.is_evidence_verified(evidence_id));
}

#[ink::test]
fn test_batch_submit_evidence() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);
    setup_test_environment(&mut contract, &policyholder);

    // Submit claim
    let claim_id = contract.submit_claim(
        1,
        50_000_000_000,
        "Claim".to_string(),
        EvidenceMetadata::default(),
    ).unwrap();

    // Batch submit 5 evidence items
    let batch = vec![
        ("photo".to_string(), "Qm1".to_string(), vec![1u8; 32], 1024u64 * 100, Some("meta1".to_string())),
        ("photo".to_string(), "Qm2".to_string(), vec![2u8; 32], 1024u64 * 150, Some("meta2".to_string())),
        ("document".to_string(), "Qm3".to_string(), vec![3u8; 32], 1024u64 * 200, Some("meta3".to_string())),
        ("video".to_string(), "Qm4".to_string(), vec![4u8; 32], 1024u64 * 500, Some("meta4".to_string())),
        ("sensor".to_string(), "Qm5".to_string(), vec![5u8; 32], 1024u64 * 50, Some("meta5".to_string())),
    ];

    let evidence_ids = contract.batch_submit_evidence(claim_id, batch).unwrap();
    assert_eq!(evidence_ids.len(), 5);

    // Verify all were stored
    let all_evidence = contract.get_claim_evidence(claim_id);
    assert_eq!(all_evidence.len(), 5);
}

#[ink::test]
fn test_storage_cost_calculation() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);
    setup_test_environment(&mut contract, &policyholder);

    // Submit claim and evidence
    let claim_id = contract.submit_claim(
        1,
        50_000_000_000,
        "Claim".to_string(),
        EvidenceMetadata::default(),
    ).unwrap();

    let evidence_id = contract.submit_evidence(
        claim_id,
        "document".to_string(),
        "QmDoc".to_string(),
        vec![1u8; 32],
        1024 * 1024, // 1 MB
        None,
        None,
    ).unwrap();

    // Calculate storage cost
    let cost = contract.calculate_evidence_storage_cost(evidence_id).unwrap();
    
    // Base cost (1000) + size cost (1MB * 10) = 1000 + 10485760 = 10486760
    assert!(cost > 10000000);

    // Verify evidence increases cost
    ink::env::test::set_caller::<DefaultEnvironment>(admin);
    contract.verify_evidence(evidence_id, true, "Verified".to_string()).unwrap();

    let cost_after_verification = contract.calculate_evidence_storage_cost(evidence_id).unwrap();
    assert!(cost_after_verification > cost); // Verification bonus added
}

#[ink::test]
fn test_unauthorized_evidence_submission() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;
    let random_user = accounts.charlie;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);
    setup_test_environment(&mut contract, &policyholder);

    // Submit claim
    let claim_id = contract.submit_claim(
        1,
        50_000_000_000,
        "Claim".to_string(),
        EvidenceMetadata::default(),
    ).unwrap();

    // Random user tries to submit evidence (should fail)
    ink::env::test::set_caller::<DefaultEnvironment>(random_user);
    let result = contract.submit_evidence(
        claim_id,
        "photo".to_string(),
        "QmFake".to_string(),
        vec![1u8; 32],
        1024,
        None,
        None,
    );

    assert_eq!(result, Err(InsuranceError::Unauthorized));
}

#[ink::test]
fn test_invalid_evidence_parameters() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);
    setup_test_environment(&mut contract, &policyholder);

    let claim_id = contract.submit_claim(
        1,
        50_000_000_000,
        "Claim".to_string(),
        EvidenceMetadata::default(),
    ).unwrap();

    // Test empty evidence type
    let result = contract.submit_evidence(
        claim_id,
        "".to_string(), // Empty type
        "QmValid".to_string(),
        vec![1u8; 32],
        1024,
        None,
        None,
    );
    assert_eq!(result, Err(InsuranceError::EvidenceNonceEmpty));

    // Test invalid IPFS hash format
    let result = contract.submit_evidence(
        claim_id,
        "photo".to_string(),
        "invalid_hash".to_string(), // Doesn't start with Qm or bafy
        vec![1u8; 32],
        1024,
        None,
        None,
    );
    assert_eq!(result, Err(InsuranceError::InvalidParameters));

    // Test wrong hash length
    let result = contract.submit_evidence(
        claim_id,
        "photo".to_string(),
        "QmValid".to_string(),
        vec![1u8; 16], // Wrong length (should be 32)
        1024,
        None,
        None,
    );
    assert_eq!(result, Err(InsuranceError::EvidenceInvalidHashLength));
}

#[ink::test]
fn test_duplicate_verification_prevention() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;
    let assessor = accounts.charlie;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);
    setup_test_environment(&mut contract, &policyholder);

    // Register assessor
    ink::env::test::set_caller::<DefaultEnvironment>(admin);
    contract.register_assessor(assessor).unwrap();

    // Submit claim and evidence
    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let claim_id = contract.submit_claim(
        1,
        50_000_000_000,
        "Claim".to_string(),
        EvidenceMetadata::default(),
    ).unwrap();

    let evidence_id = contract.submit_evidence(
        claim_id,
        "photo".to_string(),
        "QmPhoto".to_string(),
        vec![1u8; 32],
        1024,
        None,
        None,
    ).unwrap();

    // Assessor verifies once
    ink::env::test::set_caller::<DefaultEnvironment>(assessor);
    let result1 = contract.verify_evidence(evidence_id, true, "First verification".to_string());
    assert!(result1.is_ok());

    // Same assessor tries to verify again (should fail)
    let result2 = contract.verify_evidence(evidence_id, false, "Second attempt".to_string());
    assert_eq!(result2, Err(InsuranceError::DuplicateClaim));
}

#[ink::test]
fn test_total_claim_evidence_cost() {
    let accounts = ink::env::test::default_accounts::<DefaultEnvironment>();
    let admin = accounts.alice;
    let policyholder = accounts.bob;

    ink::env::test::set_caller::<DefaultEnvironment>(policyholder);
    let mut contract = PropertyInsurance::new(admin);
    setup_test_environment(&mut contract, &policyholder);

    let claim_id = contract.submit_claim(
        1,
        50_000_000_000,
        "Claim".to_string(),
        EvidenceMetadata::default(),
    ).unwrap();

    // Add multiple evidence with different sizes
    contract.submit_evidence(claim_id, "photo".to_string(), "Qm1".to_string(), vec![1u8; 32], 1024 * 100, None, None).unwrap();
    contract.submit_evidence(claim_id, "photo".to_string(), "Qm2".to_string(), vec![2u8; 32], 1024 * 200, None, None).unwrap();
    contract.submit_evidence(claim_id, "video".to_string(), "Qm3".to_string(), vec![3u8; 32], 1024 * 500, None, None).unwrap();

    // Get total cost
    let total_cost = contract.get_claim_evidence_total_cost(claim_id);
    assert!(total_cost > 0);

    // Cost should be sum of individual costs
    // Each has base cost (1000) + size-based cost
    assert!(total_cost > 3000); // At least 3 base costs
}

// Helper function to setup test environment
fn setup_test_environment(contract: &mut PropertyInsurance, policyholder: &AccountId) {
    // Create risk pool
    contract.create_risk_pool(
        CoverageType::Fire,
        1_000_000_000_000,
        8000,
        100_000_000_000,
        "Test Pool".to_string(),
    ).unwrap();

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

    // Create policy
    ink::env::test::set_value_transferred::<DefaultEnvironment>(1_000_000_000);
    contract.create_policy(
        1,
        CoverageType::Fire,
        100_000_000_000,
        1,
        86400,
        "ipfs://test".to_string(),
    ).unwrap();
}
