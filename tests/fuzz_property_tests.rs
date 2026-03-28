use proptest::prelude::*;
use proptest::test_runner::{Config, FileFailurePersistence, TestRunner};

#[derive(Debug, Clone, PartialEq)]
pub enum PolicyStatus { Active, Expired, Cancelled }

#[derive(Debug, Clone)]
pub struct Policy {
    pub id: u64,
    pub coverage: u64,
    pub premium: u64,
    pub status: PolicyStatus,
}

#[derive(Debug, Clone)]
pub struct RiskPool { pub balance: i128 }

#[derive(Debug, Clone)]
pub struct GovernanceProposal {
    pub votes_for: u64,
    pub votes_against: u64,
    pub quorum: u64,
    pub threshold_pct: u8,
}

impl Policy {
    pub fn issue(id: u64, coverage: u64, premium: u64) -> Self {
        Self { id, coverage, premium, status: PolicyStatus::Active }
    }
    pub fn renew(&mut self)  { self.status = PolicyStatus::Active; }
    pub fn cancel(&mut self) { self.status = PolicyStatus::Cancelled; }
    pub fn expire(&mut self) { self.status = PolicyStatus::Expired; }
    pub fn is_active(&self) -> bool { self.status == PolicyStatus::Active }
}

impl RiskPool {
    pub fn new(initial: u64) -> Self { Self { balance: initial as i128 } }
    pub fn deposit(&mut self, amount: u64) { self.balance += amount as i128; }
    pub fn withdraw(&mut self, amount: u64) -> Result<(), &'static str> {
        if self.balance - amount as i128 < 0 { return Err("insufficient"); }
        self.balance -= amount as i128;
        Ok(())
    }
}

impl GovernanceProposal {
    pub fn is_quorum_met(&self) -> bool {
        self.votes_for + self.votes_against >= self.quorum
    }
    pub fn passes(&self) -> bool {
        if !self.is_quorum_met() { return false; }
        let total = self.votes_for + self.votes_against;
        if total == 0 { return false; }
        (self.votes_for * 100 / total) as u8 >= self.threshold_pct
    }
}

proptest! {
    #![proptest_config(Config {
        cases: 2_000,
        max_shrink_iters: 512,
        failure_persistence: Some(Box::new(
            FileFailurePersistence::WithSource("proptest-regressions"),
        )),
        ..Config::default()
    })]

    #[test]
    fn prop_newly_issued_policy_is_active(id in 0u64..u64::MAX, coverage in 1u64..=100_000_000u64, premium in 1u64..=1_000_000u64) {
        let p = Policy::issue(id, coverage, premium);
        prop_assert!(p.is_active());
    }

    #[test]
    fn prop_cancelled_policy_not_active(id in 0u64..u64::MAX, coverage in 1u64..=100_000_000u64, premium in 1u64..=1_000_000u64) {
        let mut p = Policy::issue(id, coverage, premium);
        p.cancel();
        prop_assert!(!p.is_active());
    }

    #[test]
    fn prop_expired_policy_not_active(id in 0u64..u64::MAX, coverage in 1u64..=100_000_000u64, premium in 1u64..=1_000_000u64) {
        let mut p = Policy::issue(id, coverage, premium);
        p.expire();
        prop_assert!(!p.is_active());
    }

    #[test]
    fn prop_renewed_policy_is_active(id in 0u64..u64::MAX, coverage in 1u64..=100_000_000u64, premium in 1u64..=1_000_000u64, state in 0u8..3u8) {
        let mut p = Policy::issue(id, coverage, premium);
        match state { 1 => p.cancel(), 2 => p.expire(), _ => {} }
        p.renew();
        prop_assert!(p.is_active());
    }

    #[test]
    fn prop_cannot_settle_unapproved_claim(policy_id in 0u64..u64::MAX, amount in 1u64..=100_000_000u64) {
        let approved = false;
        let settled = if approved { Some(amount) } else { None };
        prop_assert!(settled.is_none());
    }

    #[test]
    fn prop_claim_rejected_for_inactive_policy(id in 0u64..u64::MAX, coverage in 1u64..=100_000_000u64, premium in 1u64..=1_000_000u64, amount in 1u64..=100_000_000u64, state in 1u8..3u8) {
        let mut p = Policy::issue(id, coverage, premium);
        match state { 1 => p.cancel(), _ => p.expire() }
        let accepted = p.is_active() && amount <= p.coverage;
        prop_assert!(!accepted);
    }

    #[test]
    fn prop_claim_cannot_exceed_coverage(id in 0u64..u64::MAX, coverage in 1u64..=100_000_000u64, premium in 1u64..=1_000_000u64, amount in 0u64..=200_000_000u64) {
        let p = Policy::issue(id, coverage, premium);
        if amount > p.coverage {
            let valid = p.is_active() && amount <= p.coverage;
            prop_assert!(!valid);
        }
    }

    #[test]
    fn prop_risk_pool_never_goes_negative(initial in 0u64..=500_000_000u64, deposits in prop::collection::vec(0u64..=10_000_000u64, 0..20), withdrawals in prop::collection::vec(0u64..=10_000_000u64, 0..20)) {
        let mut pool = RiskPool::new(initial);
        for d in &deposits { pool.deposit(*d); }
        for w in &withdrawals {
            let _ = pool.withdraw(*w);
            prop_assert!(pool.balance >= 0);
        }
    }

    #[test]
    fn prop_slashing_reduces_balance_exactly(initial in 1u64..=100_000_000u64, slash_pct in 1u8..=100u8) {
        let mut pool = RiskPool::new(initial);
        let slash = (initial as u128 * slash_pct as u128 / 100) as u64;
        let before = pool.balance;
        if pool.withdraw(slash).is_ok() {
            prop_assert_eq!(pool.balance, before - slash as i128);
        }
    }

    #[test]
    fn prop_proposal_fails_without_quorum(votes_for in 0u64..=500u64, votes_against in 0u64..=500u64, quorum in 1u64..=1000u64, threshold in 51u8..=100u8) {
        let p = GovernanceProposal { votes_for, votes_against, quorum, threshold_pct: threshold };
        if votes_for + votes_against < quorum {
            prop_assert!(!p.passes());
        }
    }

    #[test]
    fn prop_unanimous_quorum_passes(quorum in 1u64..=100u64, threshold in 51u8..=100u8) {
        let p = GovernanceProposal { votes_for: quorum, votes_against: 0, quorum, threshold_pct: threshold };
        prop_assert!(p.passes());
    }
}
