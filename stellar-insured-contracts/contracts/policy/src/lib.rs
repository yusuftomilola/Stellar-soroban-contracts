use soroban_sdk::{contracttype, Symbol, Env, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyEvent {
    PolicyIssued(PolicyContext),
    PolicyRenewed(PolicyContext),
    PolicyCanceled(PolicyContext),
    PolicyExpired(PolicyContext),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyContext {
    pub policy_id: u64,
    pub holder: Address,
    pub coverage_amount: i128,
    pub premium_amount: i128,
    pub duration_days: u32,
    pub policy_type: Symbol,
    pub timestamp: u64,
}