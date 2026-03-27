#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unexpected_cfgs)]

use ink::prelude::string::String;
use ink::storage::Mapping;
use propchain_traits::*;
#[cfg(not(feature = "std"))]
use scale_info::prelude::vec::Vec;

#[ink::contract]
mod property_token {
    use super::*;

    /// Error types for the property token contract
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        // Standard ERC errors
        TokenNotFound,
        Unauthorized,
        // Property-specific errors
        PropertyNotFound,
        InvalidMetadata,
        DocumentNotFound,
        ComplianceFailed,
        // Cross-chain bridge errors
        BridgeNotSupported,
        InvalidChain,
        BridgeLocked,
        InsufficientSignatures,
        RequestExpired,
        InvalidRequest,
        BridgePaused,
        GasLimitExceeded,
        MetadataCorruption,
        InvalidBridgeOperator,
        DuplicateBridgeRequest,
        BridgeTimeout,
        AlreadySigned,
        InsufficientBalance,
        InvalidAmount,
        ProposalNotFound,
        ProposalClosed,
        AskNotFound,
        // Slashing-specific errors
        SlashCooldownActive,
        SlashBlacklisted,
        UnauthorizedSlashingRole,
        SlashingACLRequired,
    }

    /// Property Token contract that maintains compatibility with ERC-721 and ERC-1155
    /// while adding real estate-specific features and cross-chain support
    #[ink(storage)]
    pub struct PropertyToken {
        // ERC-721 standard mappings
        token_owner: Mapping<TokenId, AccountId>,
        owner_token_count: Mapping<AccountId, u32>,
        token_approvals: Mapping<TokenId, AccountId>,
        operator_approvals: Mapping<(AccountId, AccountId), bool>,

        // ERC-1155 batch operation support
        balances: Mapping<(AccountId, TokenId), u128>,
        operators: Mapping<(AccountId, AccountId), bool>,

        // Property-specific mappings
        token_properties: Mapping<TokenId, PropertyInfo>,
        property_tokens: Mapping<u64, TokenId>, // property_id to token_id mapping
        ownership_history_count: Mapping<TokenId, u32>,
        ownership_history_items: Mapping<(TokenId, u32), OwnershipTransfer>,
        compliance_flags: Mapping<TokenId, ComplianceInfo>,
        legal_documents_count: Mapping<TokenId, u32>,
        legal_documents_items: Mapping<(TokenId, u32), DocumentInfo>,

        // Cross-chain bridge mappings
        bridged_tokens: Mapping<(ChainId, TokenId), BridgedTokenInfo>,
        bridge_operators: Vec<AccountId>,
        bridge_requests: Mapping<u64, MultisigBridgeRequest>,
        bridge_transactions: Mapping<AccountId, Vec<BridgeTransaction>>,
        bridge_config: BridgeConfig,
        verified_bridge_hashes: Mapping<Hash, bool>,
        bridge_request_counter: u64,

        // Standard counters
        total_supply: u64,
        token_counter: u64,
        admin: AccountId,

        // Error logging and monitoring
        error_counts: Mapping<(AccountId, String), u64>,
        error_rates: Mapping<String, (u64, u64)>, // (count, window_start)
        recent_errors: Mapping<u64, ErrorLogEntry>,
        error_log_counter: u64,

        total_shares: Mapping<TokenId, u128>,
        dividends_per_share: Mapping<TokenId, u128>,
        dividend_credit: Mapping<(AccountId, TokenId), u128>,
        dividend_balance: Mapping<(AccountId, TokenId), u128>,
        proposal_counter: Mapping<TokenId, u64>,
        proposals: Mapping<(TokenId, u64), Proposal>,
        votes_cast: Mapping<(TokenId, u64, AccountId), bool>,
        asks: Mapping<(TokenId, AccountId), Ask>,
        escrowed_shares: Mapping<(TokenId, AccountId), u128>,
        last_trade_price: Mapping<TokenId, u128>,
        compliance_registry: Option<AccountId>,
        tax_records: Mapping<(AccountId, TokenId), TaxRecord>,

        // History tracking mappings for enterprise-grade APIs
        proposal_history_count: Mapping<TokenId, u32>,
        proposal_history_items: Mapping<(TokenId, u32), ProposalHistoryEntry>,
        vote_history_count: Mapping<(TokenId, u64), u32>,
        vote_history_items: Mapping<(TokenId, u64, u32), VoteHistoryEntry>,
        execution_history_count: u32,
        execution_history_items: Mapping<u32, ExecutionHistoryEntry>,
        slashing_history_count: u32,
        slashing_history_items: Mapping<u32, SlashingHistoryEntry>,

        // Slashing ACL: Role -> whether it can slash specific roles
        // slashing_acl: (SlashingRole, Role) -> bool
        slashing_acl: Mapping<(u8, u8), bool>,
        
        // Slashing cooldown: (target, role) -> last slash timestamp
        slashing_cooldowns: Mapping<(AccountId, u8), u64>,
        slashing_cooldown_period: u64, // Default cooldown period in seconds
        
        // Slashing blacklist: account -> whether blacklisted from slashing
        slashing_blacklist: Mapping<AccountId, bool>,

        // Multi-role identity management
        role_assignments: Mapping<AccountId, Vec<Role>>, // Account -> roles assigned
        role_info: Mapping<(AccountId, Role), RoleInfo>, // (Account, Role) -> RoleInfo
        role_transfer_requests: Mapping<u64, RoleTransferRequest>,
        role_transfer_counter: u64,
        annual_review_logs: Mapping<u64, AnnualReviewLog>,
        annual_review_counter: u64,
        role_timelock_seconds: u64, // Timelock period for role transfers
    }
    }

    /// Token ID type alias
    pub type TokenId = u64;

    /// Chain ID type alias
    pub type ChainId = u64;

    /// Ownership transfer record
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct OwnershipTransfer {
        pub from: AccountId,
        pub to: AccountId,
        pub timestamp: u64,
        pub transaction_hash: Hash,
    }

    /// Compliance information
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ComplianceInfo {
        pub verified: bool,
        pub verification_date: u64,
        pub verifier: AccountId,
        pub compliance_type: String,
    }

    /// Role enumeration for multi-role identity model
    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Role {
        /// Admin role - full administrative privileges
        Admin,
        /// Auditor role - can trigger claim reviews and audit operations
        Auditor,
        /// Liquidity manager role - can adjust pool parameters
        LiquidityManager,
        /// Governance operator role - can execute governance proposals
        GovernanceOperator,
    }

    /// Role information with metadata
    #[derive(
        Debug,
        Clone,
        PartialEq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct RoleInfo {
        pub role: Role,
        pub granted_at: u64,
        pub granted_by: AccountId,
        pub expires_at: Option<u64>, // None means no expiration
        pub is_active: bool,
    }

    /// Role transfer request with timelock
    #[derive(
        Debug,
        Clone,
        PartialEq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct RoleTransferRequest {
        pub from_role: Role,
        pub from_account: AccountId,
        pub to_account: AccountId,
        pub requested_at: u64,
        pub executable_at: u64,
        pub is_executed: bool,
    }

    /// Annual review log entry
    #[derive(
        Debug,
        Clone,
        PartialEq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct AnnualReviewLog {
        pub account: AccountId,
        pub role: Role,
        pub reviewed_at: u64,
        pub reviewer: AccountId,
        pub performance_score: u32, // 0-100
        pub notes: String,
        pub is_renewed: bool,
    }

    /// Legal document information
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct DocumentInfo {
        pub document_hash: Hash,
        pub document_type: String,
        pub upload_date: u64,
        pub uploader: AccountId,
    }

    /// Bridged token information
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct BridgedTokenInfo {
        pub original_chain: ChainId,
        pub original_token_id: TokenId,
        pub destination_chain: ChainId,
        pub destination_token_id: TokenId,
        pub bridged_at: u64,
        pub status: BridgingStatus,
    }

    /// Bridging status enum
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum BridgingStatus {
        Locked,
        Pending,
        InTransit,
        Completed,
        Failed,
        Recovering,
        Expired,
    }

    /// Error log entry for monitoring and debugging
    #[derive(
        Debug, Clone, PartialEq, scale::Encode, scale::Decode, ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ErrorLogEntry {
        pub error_code: String,
        pub message: String,
        pub account: AccountId,
        pub timestamp: u64,
        pub context: Vec<(String, String)>,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Proposal {
        pub id: u64,
        pub token_id: TokenId,
        pub description_hash: Hash,
        pub quorum: u128,
        pub for_votes: u128,
        pub against_votes: u128,
        pub status: ProposalStatus,
        pub created_at: u64,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ProposalStatus {
        Open,
        Executed,
        Rejected,
        Closed,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Ask {
        pub token_id: TokenId,
        pub seller: AccountId,
        pub price_per_share: u128,
        pub amount: u128,
        pub created_at: u64,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct TaxRecord {
        pub dividends_received: u128,
        pub shares_sold: u128,
        pub proceeds: u128,
    }

    /// Pagination parameters for history queries
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PaginationParams {
        pub offset: u32,
        pub limit: u32,
        pub sort_ascending: bool,
    }

    /// Pagination metadata for response
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PaginationInfo {
        pub total_count: u32,
        pub returned_count: u32,
        pub offset: u32,
        pub limit: u32,
        pub has_more: bool,
    }

    /// Proposal history entry
    #[derive(
        Debug,
        Clone,
        PartialEq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ProposalHistoryEntry {
        pub proposal_id: u64,
        pub token_id: TokenId,
        pub description_hash: Hash,
        pub quorum: u128,
        pub for_votes: u128,
        pub against_votes: u128,
        pub status: ProposalStatus,
        pub created_at: u64,
        pub executed_at: Option<u64>,
        pub creator: AccountId,
    }

    /// Vote history entry
    #[derive(
        Debug,
        Clone,
        PartialEq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct VoteHistoryEntry {
        pub proposal_id: u64,
        pub token_id: TokenId,
        pub voter: AccountId,
        pub support: bool,
        pub vote_weight: u128,
        pub voted_at: u64,
    }

    /// Execution history entry
    #[derive(
        Debug,
        Clone,
        PartialEq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ExecutionHistoryEntry {
        pub proposal_id: u64,
        pub token_id: TokenId,
        pub executed_at: u64,
        pub passed: bool,
        pub executor: AccountId,
        pub transaction_hash: Hash,
    }

    /// Slashing reason enum
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum SlashingReason {
        OracleManipulation,
        GovernanceAttack,
        DoubleSigning,
        ComplianceViolation,
        MaliciousBehavior,
        Negligence,
        Custom(String),
    }

    /// Slashing role enum
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum SlashingRole {
        OracleProvider,
        GovernanceParticipant,
        RiskPoolProvider,
        ClaimSubmitter,
        BridgeOperator,
        Other(String),
    }

    /// Slashing history entry
    #[derive(
        Debug,
        Clone,
        PartialEq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct SlashingHistoryEntry {
        pub target: AccountId,
        pub role: SlashingRole,
        pub reason: SlashingReason,
        pub slashed_amount: u128,
        pub slashed_at: u64,
        pub transaction_hash: Hash,
        pub authority: AccountId,
        pub repeat_offense_count: u32,
    }

    /// History query response with pagination
    #[derive(
        Debug,
        Clone,
        PartialEq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PaginatedProposalHistory {
        pub entries: Vec<ProposalHistoryEntry>,
        pub pagination: PaginationInfo,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PaginatedVoteHistory {
        pub entries: Vec<VoteHistoryEntry>,
        pub pagination: PaginationInfo,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PaginatedExecutionHistory {
        pub entries: Vec<ExecutionHistoryEntry>,
        pub pagination: PaginationInfo,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PaginatedSlashingHistory {
        pub entries: Vec<SlashingHistoryEntry>,
        pub pagination: PaginationInfo,
    }

    /// Slashing eligibility information
    #[derive(
        Debug,
        Clone,
        PartialEq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct SlashingEligibility {
        pub target: AccountId,
        pub role: SlashingRole,
        pub is_blacklisted: bool,
        pub cooldown_remaining: u64,
        pub cooldown_period: u64,
        pub last_slash_timestamp: u64,
        pub can_be_slashed: bool,
    }

    // Events for tracking property token operations
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        pub from: Option<AccountId>,
        #[ink(topic)]
        pub to: Option<AccountId>,
        #[ink(topic)]
        pub id: TokenId,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        pub owner: AccountId,
        #[ink(topic)]
        pub spender: AccountId,
        #[ink(topic)]
        pub id: TokenId,
    }

    #[ink(event)]
    pub struct ApprovalForAll {
        #[ink(topic)]
        pub owner: AccountId,
        #[ink(topic)]
        pub operator: AccountId,
        pub approved: bool,
    }

    #[ink(event)]
    pub struct PropertyTokenMinted {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub property_id: u64,
        #[ink(topic)]
        pub owner: AccountId,
    }

    #[ink(event)]
    pub struct LegalDocumentAttached {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub document_hash: Hash,
        #[ink(topic)]
        pub document_type: String,
    }

    #[ink(event)]
    pub struct ComplianceVerified {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub verified: bool,
        #[ink(topic)]
        pub verifier: AccountId,
    }

    #[ink(event)]
    pub struct TokenBridged {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub destination_chain: ChainId,
        #[ink(topic)]
        pub recipient: AccountId,
        pub bridge_request_id: u64,
    }

    #[ink(event)]
    pub struct BridgeRequestCreated {
        #[ink(topic)]
        pub request_id: u64,
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub source_chain: ChainId,
        #[ink(topic)]
        pub destination_chain: ChainId,
        #[ink(topic)]
        pub requester: AccountId,
    }

    #[ink(event)]
    pub struct BridgeRequestSigned {
        #[ink(topic)]
        pub request_id: u64,
        #[ink(topic)]
        pub signer: AccountId,
        pub signatures_collected: u8,
        pub signatures_required: u8,
    }

    #[ink(event)]
    pub struct BridgeExecuted {
        #[ink(topic)]
        pub request_id: u64,
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub transaction_hash: Hash,
    }

    #[ink(event)]
    pub struct BridgeFailed {
        #[ink(topic)]
        pub request_id: u64,
        #[ink(topic)]
        pub token_id: TokenId,
        pub error: String,
    }

    #[ink(event)]
    pub struct BridgeRecovered {
        #[ink(topic)]
        pub request_id: u64,
        #[ink(topic)]
        pub recovery_action: RecoveryAction,
    }

    #[ink(event)]
    pub struct SharesIssued {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub to: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct SharesRedeemed {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub from: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct DividendsDeposited {
        #[ink(topic)]
        pub token_id: TokenId,
        pub amount: u128,
        pub per_share: u128,
    }

    #[ink(event)]
    pub struct DividendsWithdrawn {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub account: AccountId,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct ProposalCreated {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub proposal_id: u64,
        pub quorum: u128,
    }

    #[ink(event)]
    pub struct Voted {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub proposal_id: u64,
        #[ink(topic)]
        pub voter: AccountId,
        pub support: bool,
        pub weight: u128,
    }

    #[ink(event)]
    pub struct ProposalExecuted {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub proposal_id: u64,
        pub passed: bool,
    }

    #[ink(event)]
    pub struct AskPlaced {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub seller: AccountId,
        pub price_per_share: u128,
        pub amount: u128,
    }

    #[ink(event)]
    pub struct AskCancelled {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub seller: AccountId,
    }

    #[ink(event)]
    pub struct SharesPurchased {
        #[ink(topic)]
        pub token_id: TokenId,
        #[ink(topic)]
        pub seller: AccountId,
        #[ink(topic)]
        pub buyer: AccountId,
        pub amount: u128,
        pub price_per_share: u128,
    }

    impl PropertyToken {
        /// Creates a new PropertyToken contract
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();

            // Initialize default bridge configuration
            let bridge_config = BridgeConfig {
                supported_chains: vec![1, 2, 3], // Default supported chains
                min_signatures_required: 2,
                max_signatures_required: 5,
                default_timeout_blocks: 100,
                gas_limit_per_bridge: 500000,
                emergency_pause: false,
                metadata_preservation: true,
            };

            Self {
                // ERC-721 standard mappings
                token_owner: Mapping::default(),
                owner_token_count: Mapping::default(),
                token_approvals: Mapping::default(),
                operator_approvals: Mapping::default(),

                // ERC-1155 batch operation support
                balances: Mapping::default(),
                operators: Mapping::default(),

                // Property-specific mappings
                token_properties: Mapping::default(),
                property_tokens: Mapping::default(),
                ownership_history_count: Mapping::default(),
                ownership_history_items: Mapping::default(),
                compliance_flags: Mapping::default(),
                legal_documents_count: Mapping::default(),
                legal_documents_items: Mapping::default(),

                // Cross-chain bridge mappings
                bridged_tokens: Mapping::default(),
                bridge_operators: vec![caller],
                bridge_requests: Mapping::default(),
                bridge_transactions: Mapping::default(),
                bridge_config,
                verified_bridge_hashes: Mapping::default(),
                bridge_request_counter: 0,

                // Standard counters
                total_supply: 0,
                token_counter: 0,
                admin: caller,

                // Error logging and monitoring
                error_counts: Mapping::default(),
                error_rates: Mapping::default(),
                recent_errors: Mapping::default(),
                error_log_counter: 0,

                total_shares: Mapping::default(),
                dividends_per_share: Mapping::default(),
                dividend_credit: Mapping::default(),
                dividend_balance: Mapping::default(),
                proposal_counter: Mapping::default(),
                proposals: Mapping::default(),
                votes_cast: Mapping::default(),
                asks: Mapping::default(),
                escrowed_shares: Mapping::default(),
                last_trade_price: Mapping::default(),
                compliance_registry: None,
                tax_records: Mapping::default(),

                // History tracking mappings initialized to empty
                proposal_history_count: Mapping::default(),
                proposal_history_items: Mapping::default(),
                vote_history_count: Mapping::default(),
                vote_history_items: Mapping::default(),
                execution_history_count: 0,
                execution_history_items: Mapping::default(),
                slashing_history_count: 0,
                slashing_history_items: Mapping::default(),

                // Slashing ACL, cooldown, and blacklist initialized
                slashing_acl: Mapping::default(),
                slashing_cooldowns: Mapping::default(),
                slashing_cooldown_period: 86400, // 24 hours default cooldown
                slashing_blacklist: Mapping::default(),

                // Multi-role identity management initialized
                role_assignments: Mapping::default(),
                role_info: Mapping::default(),
                role_transfer_requests: Mapping::default(),
                role_transfer_counter: 0,
                annual_review_logs: Mapping::default(),
                annual_review_counter: 0,
                role_timelock_seconds: 604800, // 7 days default timelock
            }
        }

        // =============================================================================
        // Helper methods for role and slashing operations
        // =============================================================================

        /// Convert SlashingRole enum to u8 for storage key
        fn role_to_id(&self, role: &SlashingRole) -> u8 {
            match role {
                SlashingRole::OracleProvider => 0,
                SlashingRole::GovernanceParticipant => 1,
                SlashingRole::RiskPoolProvider => 2,
                SlashingRole::ClaimSubmitter => 3,
                SlashingRole::BridgeOperator => 4,
                SlashingRole::Other(_) => 255,
            }
        }

        /// Convert Role enum to u8 for storage key
        fn role_to_id_internal(&self, role: &Role) -> u8 {
            match role {
                Role::Admin => 0,
                Role::Auditor => 1,
                Role::LiquidityManager => 2,
                Role::GovernanceOperator => 3,
            }
        }

        /// ERC-721: Returns the balance of tokens owned by an account
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u32 {
            self.owner_token_count.get(owner).unwrap_or(0)
        }

        /// ERC-721: Returns the owner of a token
        #[ink(message)]
        pub fn owner_of(&self, token_id: TokenId) -> Option<AccountId> {
            self.token_owner.get(token_id)
        }

        /// ERC-721: Transfers a token from one account to another
        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            token_id: TokenId,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            // Check if caller is authorized to transfer
            let token_owner = self.token_owner.get(token_id).ok_or_else(|| {
                let caller = self.env().caller();
                self.log_error(
                    caller,
                    "TOKEN_NOT_FOUND".to_string(),
                    format!("Token ID {} does not exist", token_id),
                    vec![
                        ("token_id".to_string(), token_id.to_string()),
                        ("operation".to_string(), "transfer_from".to_string()),
                    ],
                );
                Error::TokenNotFound
            })?;
            if token_owner != from {
                let caller = self.env().caller();
                self.log_error(
                    caller,
                    "UNAUTHORIZED".to_string(),
                    format!("Caller is not authorized to transfer token {}", token_id),
                    vec![
                        ("token_id".to_string(), token_id.to_string()),
                        ("caller".to_string(), format!("{:?}", caller)),
                        ("owner".to_string(), format!("{:?}", token_owner)),
                    ],
                );
                return Err(Error::Unauthorized);
            }

            if caller != from
                && Some(caller) != self.token_approvals.get(token_id)
                && !self.is_approved_for_all(from, caller)
            {
                return Err(Error::Unauthorized);
            }

            // Perform the transfer
            self.remove_token_from_owner(from, token_id)?;
            self.add_token_to_owner(to, token_id)?;

            // Clear approvals
            self.token_approvals.remove(token_id);

            // Update ownership history
            self.update_ownership_history(token_id, from, to)?;

            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                id: token_id,
            });

            Ok(())
        }

        /// ERC-721: Approves an account to transfer a specific token
        #[ink(message)]
        pub fn approve(&mut self, to: AccountId, token_id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            let token_owner = self.token_owner.get(token_id).ok_or_else(|| {
                self.log_error(
                    caller,
                    "TOKEN_NOT_FOUND".to_string(),
                    format!("Token ID {} does not exist", token_id),
                    vec![
                        ("token_id".to_string(), token_id.to_string()),
                        ("operation".to_string(), "approve".to_string()),
                    ],
                );
                Error::TokenNotFound
            })?;

            if token_owner != caller && !self.is_approved_for_all(token_owner, caller) {
                self.log_error(
                    caller,
                    "UNAUTHORIZED".to_string(),
                    format!("Caller is not authorized to approve token {}", token_id),
                    vec![
                        ("token_id".to_string(), token_id.to_string()),
                        ("caller".to_string(), format!("{:?}", caller)),
                        ("owner".to_string(), format!("{:?}", token_owner)),
                    ],
                );
                return Err(Error::Unauthorized);
            }

            self.token_approvals.insert(token_id, &to);

            self.env().emit_event(Approval {
                owner: token_owner,
                spender: to,
                id: token_id,
            });

            Ok(())
        }

        /// ERC-721: Sets or unsets an operator for an owner
        #[ink(message)]
        pub fn set_approval_for_all(
            &mut self,
            operator: AccountId,
            approved: bool,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            self.operator_approvals
                .insert((&caller, &operator), &approved);

            self.env().emit_event(ApprovalForAll {
                owner: caller,
                operator,
                approved,
            });

            Ok(())
        }

        /// ERC-721: Gets the approved account for a token
        #[ink(message)]
        pub fn get_approved(&self, token_id: TokenId) -> Option<AccountId> {
            self.token_approvals.get(token_id)
        }

        /// ERC-721: Checks if an operator is approved for an owner
        #[ink(message)]
        pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.operator_approvals
                .get((&owner, &operator))
                .unwrap_or(false)
        }

        /// ERC-1155: Returns the balance of tokens for an account
        #[ink(message)]
        pub fn balance_of_batch(&self, accounts: Vec<AccountId>, ids: Vec<TokenId>) -> Vec<u128> {
            let mut balances = Vec::new();
            for i in 0..accounts.len() {
                if i < ids.len() {
                    let balance = self.balances.get((&accounts[i], &ids[i])).unwrap_or(0);
                    balances.push(balance);
                } else {
                    balances.push(0);
                }
            }
            balances
        }

        /// ERC-1155: Safely transfers tokens from one account to another
        #[ink(message)]
        pub fn safe_batch_transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            ids: Vec<TokenId>,
            amounts: Vec<u128>,
            _data: Vec<u8>,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            if from != caller && !self.is_approved_for_all(from, caller) {
                return Err(Error::Unauthorized);
            }

            // Verify lengths match
            if ids.len() != amounts.len() {
                return Err(Error::Unauthorized); // Using this as a general error for mismatched arrays
            }

            // Transfer each token
            for i in 0..ids.len() {
                let token_id = ids[i];
                let amount = amounts[i];

                // Check balance
                let from_balance = self.balances.get((&from, &token_id)).unwrap_or(0);
                if from_balance < amount {
                    return Err(Error::Unauthorized);
                }

                // Update balances
                self.balances
                    .insert((&from, &token_id), &(from_balance - amount));
                let to_balance = self.balances.get((&to, &token_id)).unwrap_or(0);
                self.balances
                    .insert((&to, &token_id), &(to_balance + amount));
            }

            // Emit transfer events for each token
            for id in &ids {
                self.env().emit_event(Transfer {
                    from: Some(from),
                    to: Some(to),
                    id: *id,
                });
            }

            Ok(())
        }

        /// ERC-1155: Returns the URI for a token
        #[ink(message)]
        pub fn uri(&self, token_id: TokenId) -> Option<String> {
            // Return a standard URI format for the token metadata
            let _property_info = self.token_properties.get(token_id)?;
            Some(format!(
                "ipfs://property/{:?}/{}/metadata.json",
                self.env().account_id(),
                token_id
            ))
        }

        #[ink(message)]
        pub fn set_compliance_registry(&mut self, registry: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            self.compliance_registry = Some(registry);
            Ok(())
        }

        #[ink(message)]
        pub fn total_shares(&self, token_id: TokenId) -> u128 {
            self.total_shares.get(token_id).unwrap_or(0)
        }

        #[ink(message)]
        pub fn share_balance_of(&self, owner: AccountId, token_id: TokenId) -> u128 {
            self.balances.get((owner, token_id)).unwrap_or(0)
        }

        #[ink(message)]
        pub fn issue_shares(
            &mut self,
            token_id: TokenId,
            to: AccountId,
            amount: u128,
        ) -> Result<(), Error> {
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }
            let caller = self.env().caller();
            let owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;
            if caller != self.admin && caller != owner {
                return Err(Error::Unauthorized);
            }
            let bal = self.balances.get((to, token_id)).unwrap_or(0);
            self.balances
                .insert((to, token_id), &(bal.saturating_add(amount)));
            let ts = self.total_shares.get(token_id).unwrap_or(0);
            self.total_shares
                .insert(token_id, &(ts.saturating_add(amount)));
            self.update_dividend_credit_on_change(to, token_id)?;
            self.env().emit_event(SharesIssued {
                token_id,
                to,
                amount,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn redeem_shares(
            &mut self,
            token_id: TokenId,
            from: AccountId,
            amount: u128,
        ) -> Result<(), Error> {
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }
            let caller = self.env().caller();
            if caller != from && !self.is_approved_for_all(from, caller) {
                return Err(Error::Unauthorized);
            }
            let bal = self.balances.get((from, token_id)).unwrap_or(0);
            if bal < amount {
                return Err(Error::InsufficientBalance);
            }
            self.balances
                .insert((from, token_id), &(bal.saturating_sub(amount)));
            let ts = self.total_shares.get(token_id).unwrap_or(0);
            self.total_shares
                .insert(token_id, &(ts.saturating_sub(amount)));
            self.update_dividend_credit_on_change(from, token_id)?;
            self.env().emit_event(SharesRedeemed {
                token_id,
                from,
                amount,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn transfer_shares(
            &mut self,
            from: AccountId,
            to: AccountId,
            token_id: TokenId,
            amount: u128,
        ) -> Result<(), Error> {
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }
            let caller = self.env().caller();
            if caller != from && !self.is_approved_for_all(from, caller) {
                return Err(Error::Unauthorized);
            }
            if !self.pass_compliance(from)? || !self.pass_compliance(to)? {
                return Err(Error::ComplianceFailed);
            }
            let from_balance = self.balances.get((from, token_id)).unwrap_or(0);
            if from_balance < amount {
                return Err(Error::InsufficientBalance);
            }
            self.update_dividend_credit_on_change(from, token_id)?;
            self.update_dividend_credit_on_change(to, token_id)?;
            self.balances
                .insert((from, token_id), &(from_balance.saturating_sub(amount)));
            let to_balance = self.balances.get((to, token_id)).unwrap_or(0);
            self.balances
                .insert((to, token_id), &(to_balance.saturating_add(amount)));
            Ok(())
        }

        #[ink(message, payable)]
        pub fn deposit_dividends(&mut self, token_id: TokenId) -> Result<(), Error> {
            let value = self.env().transferred_value();
            if value == 0 {
                return Err(Error::InvalidAmount);
            }
            let ts = self.total_shares.get(token_id).unwrap_or(0);
            if ts == 0 {
                return Err(Error::InvalidRequest);
            }
            let scaling: u128 = 1_000_000_000_000;
            let add = value.saturating_mul(scaling) / ts;
            let cur = self.dividends_per_share.get(token_id).unwrap_or(0);
            let new = cur.saturating_add(add);
            self.dividends_per_share.insert(token_id, &new);
            self.env().emit_event(DividendsDeposited {
                token_id,
                amount: value,
                per_share: add,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn withdraw_dividends(&mut self, token_id: TokenId) -> Result<u128, Error> {
            let caller = self.env().caller();
            self.update_dividend_credit_on_change(caller, token_id)?;
            let owed = self.dividend_balance.get((caller, token_id)).unwrap_or(0);
            if owed == 0 {
                return Ok(0);
            }
            self.dividend_balance.insert((caller, token_id), &0u128);
            match self.env().transfer(caller, owed) {
                Ok(_) => {
                    let mut rec = self
                        .tax_records
                        .get((caller, token_id))
                        .unwrap_or(TaxRecord {
                            dividends_received: 0,
                            shares_sold: 0,
                            proceeds: 0,
                        });
                    rec.dividends_received = rec.dividends_received.saturating_add(owed);
                    self.tax_records.insert((caller, token_id), &rec);
                    self.env().emit_event(DividendsWithdrawn {
                        token_id,
                        account: caller,
                        amount: owed,
                    });
                    Ok(owed)
                }
                Err(_) => Err(Error::InvalidRequest),
            }
        }

        #[ink(message)]
        pub fn create_proposal(
            &mut self,
            token_id: TokenId,
            quorum: u128,
            description_hash: Hash,
        ) -> Result<u64, Error> {
            let owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;
            let caller = self.env().caller();
            if caller != self.admin && caller != owner {
                return Err(Error::Unauthorized);
            }
            let counter = self.proposal_counter.get(token_id).unwrap_or(0) + 1;
            self.proposal_counter.insert(token_id, &counter);
            let proposal = Proposal {
                id: counter,
                token_id,
                description_hash,
                quorum,
                for_votes: 0,
                against_votes: 0,
                status: ProposalStatus::Open,
                created_at: self.env().block_timestamp(),
            };
            self.proposals.insert((token_id, counter), &proposal);
            
            // Add to proposal history
            let history_count = self.proposal_history_count.get(token_id).unwrap_or(0);
            let history_entry = ProposalHistoryEntry {
                proposal_id: counter,
                token_id,
                description_hash,
                quorum,
                for_votes: 0,
                against_votes: 0,
                status: ProposalStatus::Open,
                created_at: self.env().block_timestamp(),
                executed_at: None,
                creator: caller.clone(),
            };
            self.proposal_history_items.insert((token_id, history_count), &history_entry);
            self.proposal_history_count.insert(token_id, &(history_count + 1));
            
            self.env().emit_event(ProposalCreated {
                token_id,
                proposal_id: counter,
                quorum,
            });
            Ok(counter)
        }

        #[ink(message)]
        pub fn vote(
            &mut self,
            token_id: TokenId,
            proposal_id: u64,
            support: bool,
        ) -> Result<(), Error> {
            let mut proposal = self
                .proposals
                .get((token_id, proposal_id))
                .ok_or(Error::ProposalNotFound)?;
            if proposal.status != ProposalStatus::Open {
                return Err(Error::ProposalClosed);
            }
            let voter = self.env().caller();
            if self
                .votes_cast
                .get((token_id, proposal_id, voter))
                .unwrap_or(false)
            {
                return Err(Error::Unauthorized);
            }
            let weight = self.balances.get((voter, token_id)).unwrap_or(0);
            if support {
                proposal.for_votes = proposal.for_votes.saturating_add(weight);
            } else {
                proposal.against_votes = proposal.against_votes.saturating_add(weight);
            }
            self.proposals.insert((token_id, proposal_id), &proposal);
            self.votes_cast
                .insert((token_id, proposal_id, voter), &true);
            
            // Add to vote history
            let vote_history_count = self.vote_history_count.get((token_id, proposal_id)).unwrap_or(0);
            let vote_entry = VoteHistoryEntry {
                proposal_id,
                token_id,
                voter: voter.clone(),
                support,
                vote_weight: weight,
                voted_at: self.env().block_timestamp(),
            };
            self.vote_history_items.insert((token_id, proposal_id, vote_history_count), &vote_entry);
            self.vote_history_count.insert((token_id, proposal_id), &(vote_history_count + 1));
            
            self.env().emit_event(Voted {
                token_id,
                proposal_id,
                voter,
                support,
                weight,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn execute_proposal(
            &mut self,
            token_id: TokenId,
            proposal_id: u64,
        ) -> Result<bool, Error> {
            let mut proposal = self
                .proposals
                .get((token_id, proposal_id))
                .ok_or(Error::ProposalNotFound)?;
            if proposal.status != ProposalStatus::Open {
                return Err(Error::ProposalClosed);
            }
            let passed = proposal.for_votes >= proposal.quorum
                && proposal.for_votes > proposal.against_votes;
            let execution_timestamp = self.env().block_timestamp();
            proposal.status = if passed {
                ProposalStatus::Executed
            } else {
                ProposalStatus::Rejected
            };
            self.proposals.insert((token_id, proposal_id), &proposal);
            
            // Update proposal history with execution info
            if let Some(mut history_entry) = self.proposal_history_items.get((token_id, proposal_id)) {
                history_entry.status = proposal.status.clone();
                history_entry.executed_at = Some(execution_timestamp);
                history_entry.for_votes = proposal.for_votes;
                history_entry.against_votes = proposal.against_votes;
                self.proposal_history_items.insert((token_id, proposal_id), &history_entry);
            }
            
            // Add to execution history
            let exec_count = self.execution_history_count;
            let executor = self.env().caller();
            let tx_hash = Hash::from([0u8; 32]); // In real implementation, get actual tx hash
            let exec_entry = ExecutionHistoryEntry {
                proposal_id,
                token_id,
                executed_at: execution_timestamp,
                passed,
                executor,
                transaction_hash: tx_hash,
            };
            self.execution_history_items.insert(&exec_count, &exec_entry);
            self.execution_history_count = exec_count + 1;
            
            self.env().emit_event(ProposalExecuted {
                token_id,
                proposal_id,
                passed,
            });
            Ok(passed)
        }

        #[ink(message)]
        pub fn place_ask(
            &mut self,
            token_id: TokenId,
            price_per_share: u128,
            amount: u128,
        ) -> Result<(), Error> {
            if price_per_share == 0 || amount == 0 {
                return Err(Error::InvalidAmount);
            }
            let seller = self.env().caller();
            let bal = self.balances.get((seller, token_id)).unwrap_or(0);
            if bal < amount {
                return Err(Error::InsufficientBalance);
            }
            let esc = self.escrowed_shares.get((token_id, seller)).unwrap_or(0);
            self.escrowed_shares
                .insert((token_id, seller), &(esc.saturating_add(amount)));
            self.balances
                .insert((seller, token_id), &(bal.saturating_sub(amount)));
            let ask = Ask {
                token_id,
                seller,
                price_per_share,
                amount,
                created_at: self.env().block_timestamp(),
            };
            self.asks.insert((token_id, seller), &ask);
            self.env().emit_event(AskPlaced {
                token_id,
                seller,
                price_per_share,
                amount,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn cancel_ask(&mut self, token_id: TokenId) -> Result<(), Error> {
            let seller = self.env().caller();
            let _ask = self
                .asks
                .get((token_id, seller))
                .ok_or(Error::AskNotFound)?;
            let esc = self.escrowed_shares.get((token_id, seller)).unwrap_or(0);
            let bal = self.balances.get((seller, token_id)).unwrap_or(0);
            self.balances
                .insert((seller, token_id), &(bal.saturating_add(esc)));
            self.escrowed_shares.insert((token_id, seller), &0u128);
            self.asks.remove((token_id, seller));
            self.env().emit_event(AskCancelled { token_id, seller });
            Ok(())
        }

        #[ink(message, payable)]
        pub fn buy_shares(
            &mut self,
            token_id: TokenId,
            seller: AccountId,
            amount: u128,
        ) -> Result<(), Error> {
            if amount == 0 {
                return Err(Error::InvalidAmount);
            }
            let ask = self
                .asks
                .get((token_id, seller))
                .ok_or(Error::AskNotFound)?;
            if ask.amount < amount {
                return Err(Error::InvalidAmount);
            }
            let cost = ask.price_per_share.saturating_mul(amount);
            let paid = self.env().transferred_value();
            if paid != cost {
                return Err(Error::InvalidAmount);
            }
            let buyer = self.env().caller();
            if !self.pass_compliance(buyer)? || !self.pass_compliance(seller)? {
                return Err(Error::ComplianceFailed);
            }
            let esc = self.escrowed_shares.get((token_id, seller)).unwrap_or(0);
            if esc < amount {
                return Err(Error::AskNotFound);
            }
            let to_balance = self.balances.get((buyer, token_id)).unwrap_or(0);
            self.balances
                .insert((buyer, token_id), &(to_balance.saturating_add(amount)));
            self.escrowed_shares
                .insert((token_id, seller), &(esc.saturating_sub(amount)));
            match self.env().transfer(seller, cost) {
                Ok(_) => {
                    let mut rec = self
                        .tax_records
                        .get((seller, token_id))
                        .unwrap_or(TaxRecord {
                            dividends_received: 0,
                            shares_sold: 0,
                            proceeds: 0,
                        });
                    rec.shares_sold = rec.shares_sold.saturating_add(amount);
                    rec.proceeds = rec.proceeds.saturating_add(cost);
                    self.tax_records.insert((seller, token_id), &rec);
                }
                Err(_) => return Err(Error::InvalidRequest),
            }
            self.last_trade_price.insert(token_id, &ask.price_per_share);
            if ask.amount == amount {
                self.asks.remove((token_id, seller));
            } else {
                let mut new_ask = ask.clone();
                new_ask.amount = ask.amount.saturating_sub(amount);
                self.asks.insert((token_id, seller), &new_ask);
            }
            self.env().emit_event(SharesPurchased {
                token_id,
                seller,
                buyer,
                amount,
                price_per_share: ask.price_per_share,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn get_last_trade_price(&self, token_id: TokenId) -> Option<u128> {
            self.last_trade_price.get(token_id)
        }

        #[ink(message)]
        pub fn get_portfolio(
            &self,
            owner: AccountId,
            token_ids: Vec<TokenId>,
        ) -> Vec<(TokenId, u128, u128)> {
            let mut out = Vec::new();
            for t in token_ids.iter() {
                let bal = self.balances.get((owner, *t)).unwrap_or(0);
                let price = self.last_trade_price.get(*t).unwrap_or(0);
                out.push((*t, bal, price));
            }
            out
        }

        #[ink(message)]
        pub fn get_tax_record(&self, owner: AccountId, token_id: TokenId) -> TaxRecord {
            self.tax_records
                .get((owner, token_id))
                .unwrap_or(TaxRecord {
                    dividends_received: 0,
                    shares_sold: 0,
                    proceeds: 0,
                })
        }

        fn pass_compliance(&self, account: AccountId) -> Result<bool, Error> {
            if let Some(registry) = self.compliance_registry {
                use ink::env::call::FromAccountId;
                let checker: ink::contract_ref!(propchain_traits::ComplianceChecker) =
                    FromAccountId::from_account_id(registry);
                Ok(checker.is_compliant(account))
            } else {
                Ok(true)
            }
        }

        fn update_dividend_credit_on_change(
            &mut self,
            account: AccountId,
            token_id: TokenId,
        ) -> Result<(), Error> {
            let scaling: u128 = 1_000_000_000_000;
            let dps = self.dividends_per_share.get(token_id).unwrap_or(0);
            let credited = self.dividend_credit.get((account, token_id)).unwrap_or(0);
            if dps > credited {
                let bal = self.balances.get((account, token_id)).unwrap_or(0);
                let mut owed = self.dividend_balance.get((account, token_id)).unwrap_or(0);
                let delta = dps.saturating_sub(credited);
                let add = bal.saturating_mul(delta) / scaling;
                owed = owed.saturating_add(add);
                self.dividend_balance.insert((account, token_id), &owed);
                self.dividend_credit.insert((account, token_id), &dps);
            } else if credited == 0 && dps > 0 {
                self.dividend_credit.insert((account, token_id), &dps);
            }
            Ok(())
        }

        /// Property-specific: Registers a property and mints a token
        #[ink(message)]
        pub fn register_property_with_token(
            &mut self,
            metadata: PropertyMetadata,
        ) -> Result<TokenId, Error> {
            let caller = self.env().caller();

            // Register property in the property registry (simulated here)
            // In a real implementation, this might call an external contract

            // Mint a new token
            self.token_counter += 1;
            let token_id = self.token_counter;

            // Store property information
            let property_info = PropertyInfo {
                id: token_id, // Using token_id as property id for this implementation
                owner: caller,
                metadata: metadata.clone(),
                registered_at: self.env().block_timestamp(),
            };

            self.token_owner.insert(token_id, &caller);
            self.add_token_to_owner(caller, token_id)?;

            // Initialize balances
            self.balances.insert((&caller, &token_id), &1u128);

            // Store property-specific information
            self.token_properties.insert(token_id, &property_info);
            self.property_tokens.insert(token_id, &token_id); // property_id maps to token_id

            // Initialize ownership history
            let initial_transfer = OwnershipTransfer {
                from: AccountId::from([0u8; 32]), // Zero address for minting
                to: caller,
                timestamp: self.env().block_timestamp(),
                transaction_hash: {
                    use scale::Encode;
                    let data = (&caller, token_id);
                    let encoded = data.encode();
                    let mut hash_bytes = [0u8; 32];
                    let len = encoded.len().min(32);
                    hash_bytes[..len].copy_from_slice(&encoded[..len]);
                    Hash::from(hash_bytes)
                },
            };

            self.ownership_history_count.insert(token_id, &1u32);
            self.ownership_history_items
                .insert((token_id, 0), &initial_transfer);

            // Initialize compliance as unverified
            let compliance_info = ComplianceInfo {
                verified: false,
                verification_date: 0,
                verifier: AccountId::from([0u8; 32]),
                compliance_type: String::from("KYC"),
            };
            self.compliance_flags.insert(token_id, &compliance_info);

            // Initialize legal documents count
            self.legal_documents_count.insert(token_id, &0u32);

            self.total_supply += 1;

            self.env().emit_event(PropertyTokenMinted {
                token_id,
                property_id: token_id,
                owner: caller,
            });

            Ok(token_id)
        }

        /// Property-specific: Batch registers properties in a single gas-efficient transaction
        #[ink(message)]
        pub fn batch_register_properties(
            &mut self,
            metadata_list: Vec<PropertyMetadata>,
        ) -> Result<Vec<TokenId>, Error> {
            let caller = self.env().caller();
            let mut issued_tokens = Vec::new();
            let current_time = self.env().block_timestamp();

            for metadata in metadata_list {
                self.token_counter += 1;
                let token_id = self.token_counter;

                let property_info = PropertyInfo {
                    id: token_id,
                    owner: caller,
                    metadata: metadata.clone(),
                    registered_at: current_time,
                };

                self.token_owner.insert(token_id, &caller);
                let balance = self.owner_token_count.get(caller).unwrap_or(0);
                self.owner_token_count.insert(caller, &(balance + 1));

                self.balances.insert((&caller, &token_id), &1u128);
                self.token_properties.insert(token_id, &property_info);
                self.property_tokens.insert(token_id, &token_id);

                let initial_transfer = OwnershipTransfer {
                    from: AccountId::from([0u8; 32]),
                    to: caller,
                    timestamp: current_time,
                    transaction_hash: Hash::default(),
                };

                self.ownership_history_count.insert(token_id, &1u32);
                self.ownership_history_items
                    .insert((token_id, 0), &initial_transfer);

                let compliance_info = ComplianceInfo {
                    verified: false,
                    verification_date: 0,
                    verifier: AccountId::from([0u8; 32]),
                    compliance_type: String::from("KYC"),
                };
                self.compliance_flags.insert(token_id, &compliance_info);
                self.legal_documents_count.insert(token_id, &0u32);

                self.env().emit_event(PropertyTokenMinted {
                    token_id,
                    property_id: token_id,
                    owner: caller,
                });

                issued_tokens.push(token_id);
            }

            self.total_supply += issued_tokens.len() as u64;

            Ok(issued_tokens)
        }

        /// Property-specific: Attaches a legal document to a token
        #[ink(message)]
        pub fn attach_legal_document(
            &mut self,
            token_id: TokenId,
            document_hash: Hash,
            document_type: String,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let token_owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;

            if token_owner != caller {
                return Err(Error::Unauthorized);
            }

            // Get existing documents count
            let document_count = self.legal_documents_count.get(token_id).unwrap_or(0);

            // Add new document
            let document_info = DocumentInfo {
                document_hash,
                document_type: document_type.clone(),
                upload_date: self.env().block_timestamp(),
                uploader: caller,
            };

            // Save updated documents
            self.legal_documents_items
                .insert((token_id, document_count), &document_info);
            self.legal_documents_count
                .insert(token_id, &(document_count + 1));

            self.env().emit_event(LegalDocumentAttached {
                token_id,
                document_hash,
                document_type,
            });

            Ok(())
        }

        /// Property-specific: Verifies compliance for a token
        #[ink(message)]
        pub fn verify_compliance(
            &mut self,
            token_id: TokenId,
            verification_status: bool,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            // Only admin or bridge operators can verify compliance
            if caller != self.admin && !self.bridge_operators.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            let mut compliance_info = self
                .compliance_flags
                .get(token_id)
                .ok_or(Error::TokenNotFound)?;
            compliance_info.verified = verification_status;
            compliance_info.verification_date = self.env().block_timestamp();
            compliance_info.verifier = caller;

            self.compliance_flags.insert(token_id, &compliance_info);

            self.env().emit_event(ComplianceVerified {
                token_id,
                verified: verification_status,
                verifier: caller,
            });

            Ok(())
        }

        /// Property-specific: Gets ownership history for a token
        #[ink(message)]
        pub fn get_ownership_history(&self, token_id: TokenId) -> Option<Vec<OwnershipTransfer>> {
            let count = self.ownership_history_count.get(token_id).unwrap_or(0);
            if count == 0 {
                return None;
            }
            let mut result = Vec::new();
            for i in 0..count {
                if let Some(item) = self.ownership_history_items.get((token_id, i)) {
                    result.push(item);
                }
            }
            Some(result)
        }

        /// Cross-chain: Initiates token bridging to another chain with multi-signature
        #[ink(message)]
        pub fn initiate_bridge_multisig(
            &mut self,
            token_id: TokenId,
            destination_chain: ChainId,
            recipient: AccountId,
            required_signatures: u8,
            timeout_blocks: Option<u64>,
        ) -> Result<u64, Error> {
            let caller = self.env().caller();
            let token_owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;

            // Check authorization
            if token_owner != caller {
                return Err(Error::Unauthorized);
            }

            // Check if bridge is paused
            if self.bridge_config.emergency_pause {
                return Err(Error::BridgePaused);
            }

            // Validate destination chain
            if !self
                .bridge_config
                .supported_chains
                .contains(&destination_chain)
            {
                return Err(Error::InvalidChain);
            }

            // Check compliance before bridging
            let compliance_info = self
                .compliance_flags
                .get(token_id)
                .ok_or(Error::ComplianceFailed)?;
            if !compliance_info.verified {
                return Err(Error::ComplianceFailed);
            }

            // Validate signature requirements
            if required_signatures < self.bridge_config.min_signatures_required
                || required_signatures > self.bridge_config.max_signatures_required
            {
                return Err(Error::InsufficientSignatures);
            }

            // Check for duplicate requests
            if self.has_pending_bridge_request(token_id) {
                return Err(Error::DuplicateBridgeRequest);
            }

            // Create bridge request
            self.bridge_request_counter += 1;
            let request_id = self.bridge_request_counter;
            let current_block = self.env().block_number();
            let _expires_at = timeout_blocks.map(|blocks| u64::from(current_block) + blocks);

            let property_info = self
                .token_properties
                .get(token_id)
                .ok_or(Error::PropertyNotFound)?;

            let request = MultisigBridgeRequest {
                request_id,
                token_id,
                source_chain: 1, // Current chain ID
                destination_chain,
                sender: caller,
                recipient,
                required_signatures,
                signatures: Vec::new(),
                created_at: u64::from(current_block),
                expires_at: timeout_blocks.map(|blocks| u64::from(current_block) + blocks),
                status: BridgeOperationStatus::Pending,
                metadata: property_info.metadata.clone(),
            };

            self.bridge_requests.insert(request_id, &request);

            self.env().emit_event(BridgeRequestCreated {
                request_id,
                token_id,
                source_chain: request.source_chain,
                destination_chain,
                requester: caller,
            });

            Ok(request_id)
        }

        /// Cross-chain: Signs a bridge request
        #[ink(message)]
        pub fn sign_bridge_request(&mut self, request_id: u64, approve: bool) -> Result<(), Error> {
            let caller = self.env().caller();

            // Check if caller is a bridge operator
            if !self.bridge_operators.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            let mut request = self
                .bridge_requests
                .get(request_id)
                .ok_or(Error::InvalidRequest)?;

            // Check if request has expired
            if let Some(expires_at) = request.expires_at {
                if u64::from(self.env().block_number()) > expires_at {
                    request.status = BridgeOperationStatus::Expired;
                    self.bridge_requests.insert(request_id, &request);
                    return Err(Error::RequestExpired);
                }
            }

            // Check if already signed
            if request.signatures.contains(&caller) {
                return Err(Error::AlreadySigned);
            }

            // Add signature
            request.signatures.push(caller);

            // Update status based on approval and signatures collected
            if !approve {
                request.status = BridgeOperationStatus::Failed;
                self.env().emit_event(BridgeFailed {
                    request_id,
                    token_id: request.token_id,
                    error: String::from("Request rejected by operator"),
                });
            } else if request.signatures.len() >= request.required_signatures as usize {
                request.status = BridgeOperationStatus::Locked;

                // Lock the token for bridging
                let token_owner = self
                    .token_owner
                    .get(request.token_id)
                    .ok_or(Error::TokenNotFound)?;
                self.balances
                    .insert((&token_owner, &request.token_id), &0u128);
                self.token_owner
                    .insert(request.token_id, &AccountId::from([0u8; 32])); // Lock to zero address
            }

            self.bridge_requests.insert(request_id, &request);

            self.env().emit_event(BridgeRequestSigned {
                request_id,
                signer: caller,
                signatures_collected: request.signatures.len() as u8,
                signatures_required: request.required_signatures,
            });

            Ok(())
        }

        /// Cross-chain: Executes a bridge request after collecting required signatures
        #[ink(message)]
        pub fn execute_bridge(&mut self, request_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();

            // Check if caller is a bridge operator
            if !self.bridge_operators.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            let mut request = self
                .bridge_requests
                .get(request_id)
                .ok_or(Error::InvalidRequest)?;

            // Check if request is ready for execution
            if request.status != BridgeOperationStatus::Locked {
                return Err(Error::InvalidRequest);
            }

            // Check if enough signatures are collected
            if request.signatures.len() < request.required_signatures as usize {
                return Err(Error::InsufficientSignatures);
            }

            // Generate transaction hash
            let transaction_hash = self.generate_bridge_transaction_hash(&request);

            // Create bridge transaction record
            let transaction = BridgeTransaction {
                transaction_id: self.bridge_request_counter,
                token_id: request.token_id,
                source_chain: request.source_chain,
                destination_chain: request.destination_chain,
                sender: request.sender,
                recipient: request.recipient,
                transaction_hash,
                timestamp: self.env().block_timestamp(),
                gas_used: self.estimate_bridge_gas_usage(&request),
                status: BridgeOperationStatus::InTransit,
                metadata: request.metadata.clone(),
            };

            // Update request status
            request.status = BridgeOperationStatus::Completed;
            self.bridge_requests.insert(request_id, &request);

            // Store transaction verification
            self.verified_bridge_hashes.insert(transaction_hash, &true);

            // Add to bridge history
            let mut history = self
                .bridge_transactions
                .get(request.sender)
                .unwrap_or_default();
            history.push(transaction.clone());
            self.bridge_transactions.insert(request.sender, &history);

            // Update bridged token info
            let bridged_info = BridgedTokenInfo {
                original_chain: request.source_chain,
                original_token_id: request.token_id,
                destination_chain: request.destination_chain,
                destination_token_id: request.token_id, // Will be updated on destination
                bridged_at: self.env().block_timestamp(),
                status: BridgingStatus::InTransit,
            };

            self.bridged_tokens.insert(
                (&request.destination_chain, &request.token_id),
                &bridged_info,
            );

            self.env().emit_event(BridgeExecuted {
                request_id,
                token_id: request.token_id,
                transaction_hash,
            });

            Ok(())
        }

        /// Cross-chain: Receives a bridged token from another chain
        #[ink(message)]
        pub fn receive_bridged_token(
            &mut self,
            source_chain: ChainId,
            original_token_id: TokenId,
            recipient: AccountId,
            metadata: PropertyMetadata,
            transaction_hash: Hash,
        ) -> Result<TokenId, Error> {
            // Only bridge operators can receive bridged tokens
            let caller = self.env().caller();
            if !self.bridge_operators.contains(&caller) {
                return Err(Error::Unauthorized);
            }

            // Verify transaction hash
            if !self
                .verified_bridge_hashes
                .get(transaction_hash)
                .unwrap_or(false)
            {
                return Err(Error::InvalidRequest);
            }

            // Create a new token for the recipient
            self.token_counter += 1;
            let new_token_id = self.token_counter;

            // Store property information
            let property_info = PropertyInfo {
                id: new_token_id,
                owner: recipient,
                metadata,
                registered_at: self.env().block_timestamp(),
            };

            self.token_properties.insert(new_token_id, &property_info);
            self.token_owner.insert(new_token_id, &recipient);
            self.add_token_to_owner(recipient, new_token_id)?;
            self.balances.insert((&recipient, &new_token_id), &1u128);

            // Initialize ownership history for the new token
            let initial_transfer = OwnershipTransfer {
                from: AccountId::from([0u8; 32]), // Zero address for minting
                to: recipient,
                timestamp: self.env().block_timestamp(),
                transaction_hash: {
                    use scale::Encode;
                    let data = (&recipient, new_token_id);
                    let encoded = data.encode();
                    let mut hash_bytes = [0u8; 32];
                    let len = encoded.len().min(32);
                    hash_bytes[..len].copy_from_slice(&encoded[..len]);
                    Hash::from(hash_bytes)
                },
            };

            self.ownership_history_count.insert(new_token_id, &1u32);
            self.ownership_history_items
                .insert((new_token_id, 0), &initial_transfer);

            // Initialize compliance as verified for bridged tokens
            let compliance_info = ComplianceInfo {
                verified: true,
                verification_date: self.env().block_timestamp(),
                verifier: caller,
                compliance_type: String::from("Bridge"),
            };
            self.compliance_flags.insert(new_token_id, &compliance_info);

            // Initialize legal documents count
            self.legal_documents_count.insert(new_token_id, &0u32);

            self.total_supply += 1;

            // Update the bridged token status
            if let Some(mut bridged_info) =
                self.bridged_tokens.get((&source_chain, &original_token_id))
            {
                bridged_info.status = BridgingStatus::Completed;
                bridged_info.destination_token_id = new_token_id;
                self.bridged_tokens
                    .insert((&source_chain, &original_token_id), &bridged_info);
            }

            self.env().emit_event(Transfer {
                from: None, // None indicates minting
                to: Some(recipient),
                id: new_token_id,
            });

            Ok(new_token_id)
        }

        /// Cross-chain: Burns a bridged token when returning to original chain
        #[ink(message)]
        pub fn burn_bridged_token(
            &mut self,
            token_id: TokenId,
            destination_chain: ChainId,
            _recipient: AccountId,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let token_owner = self.token_owner.get(token_id).ok_or(Error::TokenNotFound)?;

            // Check authorization
            if token_owner != caller {
                return Err(Error::Unauthorized);
            }

            // Check if token is bridged
            let bridged_info = self
                .bridged_tokens
                .get((&destination_chain, &token_id))
                .ok_or(Error::BridgeNotSupported)?;

            if bridged_info.status != BridgingStatus::Completed {
                return Err(Error::InvalidRequest);
            }

            // Burn the token
            self.remove_token_from_owner(caller, token_id)?;
            self.token_owner.remove(token_id);
            self.balances.insert((&caller, &token_id), &0u128);
            self.total_supply -= 1;

            // Update bridged token status
            let mut updated_info = bridged_info;
            updated_info.status = BridgingStatus::Locked;
            self.bridged_tokens
                .insert((&destination_chain, &token_id), &updated_info);

            self.env().emit_event(Transfer {
                from: Some(caller),
                to: None, // None indicates burning
                id: token_id,
            });

            Ok(())
        }

        /// Cross-chain: Recovers from a failed bridge operation
        #[ink(message)]
        pub fn recover_failed_bridge(
            &mut self,
            request_id: u64,
            recovery_action: RecoveryAction,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            // Only admin can recover failed bridges
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            let mut request = self
                .bridge_requests
                .get(request_id)
                .ok_or(Error::InvalidRequest)?;

            // Check if request is in a failed state
            if !matches!(
                request.status,
                BridgeOperationStatus::Failed | BridgeOperationStatus::Expired
            ) {
                return Err(Error::InvalidRequest);
            }

            // Execute recovery action
            match recovery_action {
                RecoveryAction::UnlockToken => {
                    // Unlock the token
                    if let Some(token_owner) = self.token_owner.get(request.token_id) {
                        if token_owner == AccountId::from([0u8; 32]) {
                            // Token is locked, restore ownership to original sender
                            self.token_owner.insert(request.token_id, &request.sender);
                            self.balances
                                .insert((&request.sender, &request.token_id), &1u128);
                            self.add_token_to_owner(request.sender, request.token_id)?;
                        }
                    }
                }
                RecoveryAction::RefundGas => {
                    // Gas refund logic would be implemented here
                    // This would typically involve transferring native tokens
                }
                RecoveryAction::RetryBridge => {
                    // Reset request to pending for retry
                    request.status = BridgeOperationStatus::Pending;
                    request.signatures.clear();
                }
                RecoveryAction::CancelBridge => {
                    // Mark as cancelled and unlock token
                    request.status = BridgeOperationStatus::Failed;
                    if let Some(token_owner) = self.token_owner.get(request.token_id) {
                        if token_owner == AccountId::from([0u8; 32]) {
                            self.token_owner.insert(request.token_id, &request.sender);
                            self.balances
                                .insert((&request.sender, &request.token_id), &1u128);
                            self.add_token_to_owner(request.sender, request.token_id)?;
                        }
                    }
                }
            }

            self.bridge_requests.insert(request_id, &request);

            self.env().emit_event(BridgeRecovered {
                request_id,
                recovery_action,
            });

            Ok(())
        }

        /// Gets gas estimation for bridge operation
        #[ink(message)]
        pub fn estimate_bridge_gas(
            &self,
            token_id: TokenId,
            destination_chain: ChainId,
        ) -> Result<u64, Error> {
            if !self
                .bridge_config
                .supported_chains
                .contains(&destination_chain)
            {
                return Err(Error::InvalidChain);
            }

            let base_gas = self.bridge_config.gas_limit_per_bridge;
            let property_info = self
                .token_properties
                .get(token_id)
                .ok_or(Error::TokenNotFound)?;
            let metadata_gas = property_info.metadata.legal_description.len() as u64 * 100;

            Ok(base_gas + metadata_gas)
        }

        /// Monitors bridge status
        #[ink(message)]
        pub fn monitor_bridge_status(&self, request_id: u64) -> Option<BridgeMonitoringInfo> {
            let request = self.bridge_requests.get(request_id)?;

            Some(BridgeMonitoringInfo {
                bridge_request_id: request.request_id,
                token_id: request.token_id,
                source_chain: request.source_chain,
                destination_chain: request.destination_chain,
                status: request.status,
                created_at: request.created_at,
                expires_at: request.expires_at,
                signatures_collected: request.signatures.len() as u8,
                signatures_required: request.required_signatures,
                error_message: None,
            })
        }

        /// Gets bridge history for an account
        #[ink(message)]
        pub fn get_bridge_history(&self, account: AccountId) -> Vec<BridgeTransaction> {
            self.bridge_transactions.get(account).unwrap_or_default()
        }

        /// Verifies bridge transaction hash
        #[ink(message)]
        pub fn verify_bridge_transaction(
            &self,
            _token_id: TokenId,
            transaction_hash: Hash,
            _source_chain: ChainId,
        ) -> bool {
            self.verified_bridge_hashes
                .get(transaction_hash)
                .unwrap_or(false)
        }

        /// Gets bridge status for a token
        #[ink(message)]
        pub fn get_bridge_status(&self, token_id: TokenId) -> Option<BridgeStatus> {
            // Check through all bridged tokens
            for chain_id in &self.bridge_config.supported_chains {
                if let Some(bridged_info) = self.bridged_tokens.get((*chain_id, token_id)) {
                    return Some(BridgeStatus {
                        is_locked: matches!(
                            bridged_info.status,
                            BridgingStatus::Locked | BridgingStatus::InTransit
                        ),
                        source_chain: Some(bridged_info.original_chain),
                        destination_chain: Some(bridged_info.destination_chain),
                        locked_at: Some(bridged_info.bridged_at),
                        bridge_request_id: None,
                        status: match bridged_info.status {
                            BridgingStatus::Locked => BridgeOperationStatus::Locked,
                            BridgingStatus::Pending => BridgeOperationStatus::Pending,
                            BridgingStatus::InTransit => BridgeOperationStatus::InTransit,
                            BridgingStatus::Completed => BridgeOperationStatus::Completed,
                            BridgingStatus::Failed => BridgeOperationStatus::Failed,
                            BridgingStatus::Recovering => BridgeOperationStatus::Recovering,
                            BridgingStatus::Expired => BridgeOperationStatus::Expired,
                        },
                    });
                }
            }
            None
        }

        /// Adds a bridge operator
        #[ink(message)]
        pub fn add_bridge_operator(&mut self, operator: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            if !self.bridge_operators.contains(&operator) {
                self.bridge_operators.push(operator);
            }

            Ok(())
        }

        /// Removes a bridge operator
        #[ink(message)]
        pub fn remove_bridge_operator(&mut self, operator: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            self.bridge_operators.retain(|op| op != &operator);
            Ok(())
        }

        /// Checks if an account is a bridge operator
        #[ink(message)]
        pub fn is_bridge_operator(&self, account: AccountId) -> bool {
            self.bridge_operators.contains(&account)
        }

        /// Gets all bridge operators
        #[ink(message)]
        pub fn get_bridge_operators(&self) -> Vec<AccountId> {
            self.bridge_operators.clone()
        }

        /// Updates bridge configuration (admin only)
        #[ink(message)]
        pub fn update_bridge_config(&mut self, config: BridgeConfig) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            self.bridge_config = config;
            Ok(())
        }

        /// Gets current bridge configuration
        #[ink(message)]
        pub fn get_bridge_config(&self) -> BridgeConfig {
            self.bridge_config.clone()
        }

        /// Pauses or unpauses the bridge (admin or governance_operator only)
        #[ink(message)]
        pub fn set_emergency_pause(&mut self, paused: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            // Admin or GovernanceOperator can pause/unpause
            if caller != self.admin && !self.has_role(caller, Role::GovernanceOperator) {
                return Err(Error::Unauthorized);
            }

            self.bridge_config.emergency_pause = paused;
            self.env().emit_event(EmergencyPauseUpdated {
                paused,
                updated_by: caller,
                updated_at: self.env().block_timestamp(),
            });
            Ok(())
        }

        // =============================================================================
        // Auditor Functions - Claim Review and Compliance Flags
        // =============================================================================

        /// Flag a token for compliance review (auditor only)
        #[ink(message)]
        pub fn flag_for_compliance_review(
            &mut self,
            token_id: TokenId,
            reason: String,
        ) -> Result<(), Error> {
            // Only auditor can flag for compliance review
            self.require_auditor()?;

            // Verify token exists
            if self.token_owner.get(token_id).is_none() {
                return Err(Error::TokenNotFound);
            }

            let current_time = self.env().block_timestamp();
            
            // Update compliance flags
            let mut compliance_info = self.compliance_flags.get(token_id).unwrap_or(ComplianceInfo {
                verified: true,
                verification_date: 0,
                verifier: AccountId::from([0u8; 32]),
                compliance_type: String::from("Standard"),
            });

            compliance_info.verified = false;
            compliance_info.compliance_type = format!("Under Review: {}", reason);
            compliance_info.verification_date = current_time;
            compliance_info.verifier = self.env().caller();

            self.compliance_flags.insert(token_id, &compliance_info);

            self.env().emit_event(ComplianceFlagged {
                token_id,
                flagged_by: self.env().caller(),
                reason,
                flagged_at: current_time,
            });

            Ok(())
        }

        /// Clear compliance flag after review (auditor only)
        #[ink(message)]
        pub fn clear_compliance_flag(
            &mut self,
            token_id: TokenId,
            notes: String,
        ) -> Result<(), Error> {
            // Only auditor can clear compliance flags
            self.require_auditor()?;

            // Verify token exists
            if self.token_owner.get(token_id).is_none() {
                return Err(Error::TokenNotFound);
            }

            let current_time = self.env().block_timestamp();
            
            // Clear compliance flags
            let compliance_info = ComplianceInfo {
                verified: true,
                verification_date: current_time,
                verifier: self.env().caller(),
                compliance_type: format!("Cleared: {}", notes),
            };

            self.compliance_flags.insert(token_id, &compliance_info);

            self.env().emit_event(ComplianceFlagCleared {
                token_id,
                cleared_by: self.env().caller(),
                notes,
                cleared_at: current_time,
            });

            Ok(())
        }

        /// Get compliance flag status for a token
        #[ink(message)]
        pub fn get_compliance_flag_status(&self, token_id: TokenId) -> Option<ComplianceInfo> {
            self.compliance_flags.get(token_id)
        }

        // =============================================================================
        // Liquidity Manager Functions - Pool Parameter Adjustments
        // =============================================================================

        /// Update dividend parameters (liquidity_manager only)
        #[ink(message)]
        pub fn update_dividend_parameters(
            &mut self,
            token_id: TokenId,
            new_dividend_rate: u128,
        ) -> Result<(), Error> {
            // Only liquidity manager can update dividend parameters
            self.require_liquidity_manager()?;

            // Verify token exists
            if self.token_owner.get(token_id).is_none() {
                return Err(Error::TokenNotFound);
            }

            let current_time = self.env().block_timestamp();

            self.env().emit_event(DividendParametersUpdated {
                token_id,
                new_dividend_rate,
                updated_by: self.env().caller(),
                updated_at: current_time,
            });

            Ok(())
        }

        /// Adjust pool risk parameters (liquidity_manager only)
        #[ink(message)]
        pub fn adjust_pool_risk_parameters(
            &mut self,
            token_id: TokenId,
            risk_adjustment: i32,
        ) -> Result<(), Error> {
            // Only liquidity manager can adjust pool parameters
            self.require_liquidity_manager()?;

            // Verify token exists
            if self.token_owner.get(token_id).is_none() {
                return Err(Error::TokenNotFound);
            }

            // Validate risk adjustment range (-100 to 100)
            if risk_adjustment < -100 || risk_adjustment > 100 {
                return Err(Error::InvalidParameters);
            }

            let current_time = self.env().block_timestamp();

            self.env().emit_event(PoolRiskParametersAdjusted {
                token_id,
                risk_adjustment,
                adjusted_by: self.env().caller(),
                adjusted_at: current_time,
            });

            Ok(())
        }

        /// Set liquidity pool fee rate (liquidity_manager only)
        #[ink(message)]
        pub fn set_liquidity_pool_fee(
            &mut self,
            token_id: TokenId,
            fee_rate: u128,
        ) -> Result<(), Error> {
            // Only liquidity manager can set fees
            self.require_liquidity_manager()?;

            // Verify token exists
            if self.token_owner.get(token_id).is_none() {
                return Err(Error::TokenNotFound);
            }

            // Fee rate should be in basis points (0-10000)
            if fee_rate > 10000 {
                return Err(Error::InvalidParameters);
            }

            let current_time = self.env().block_timestamp();

            self.env().emit_event(LiquidityPoolFeeUpdated {
                token_id,
                fee_rate,
                updated_by: self.env().caller(),
                updated_at: current_time,
            });

            Ok(())
        }

        // =============================================================================
        // Governance Operator Functions - Proposal Execution
        // =============================================================================

        /// Execute a governance proposal (governance_operator only)
        #[ink(message)]
        pub fn execute_governance_proposal(
            &mut self,
            token_id: TokenId,
            proposal_id: u64,
        ) -> Result<(), Error> {
            // Only governance operator can execute proposals
            self.require_governance_operator()?;

            // Get proposal
            let mut proposal = self
                .proposals
                .get((token_id, proposal_id))
                .ok_or(Error::ProposalNotFound)?;

            // Check if proposal is open
            if proposal.status != ProposalStatus::Open {
                return Err(Error::ProposalClosed);
            }

            // Check if quorum is met
            if proposal.for_votes < proposal.quorum {
                return Err(Error::ComplianceFailed); // Quorum not met
            }

            // Execute proposal based on outcome
            let current_time = self.env().block_timestamp();
            proposal.status = ProposalStatus::Executed;
            proposal.executed_at = Some(current_time);
            self.proposals.insert((token_id, proposal_id), &proposal);

            self.env().emit_event(GovernanceProposalExecuted {
                token_id,
                proposal_id,
                executed_by: self.env().caller(),
                executed_at: current_time,
            });

            Ok(())
        }

        /// Veto a malicious proposal (admin or governance_operator)
        #[ink(message)]
        pub fn veto_proposal(
            &mut self,
            token_id: TokenId,
            proposal_id: u64,
            reason: String,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            // Admin or GovernanceOperator can veto
            if caller != self.admin && !self.has_role(caller, Role::GovernanceOperator) {
                return Err(Error::Unauthorized);
            }

            // Get proposal
            let mut proposal = self
                .proposals
                .get((token_id, proposal_id))
                .ok_or(Error::ProposalNotFound)?;

            // Check if proposal is still open
            if proposal.status != ProposalStatus::Open {
                return Err(Error::ProposalClosed);
            }

            let current_time = self.env().block_timestamp();
            proposal.status = ProposalStatus::Rejected;
            self.proposals.insert((token_id, proposal_id), &proposal);

            self.env().emit_event(GovernanceProposalVetoed {
                token_id,
                proposal_id,
                vetoed_by: caller,
                reason,
                vetoed_at: current_time,
            });

            Ok(())
        }

        /// Returns the total supply of tokens
        #[ink(message)]
        pub fn total_supply(&self) -> u64 {
            self.total_supply
        }

        /// Returns the current token counter
        #[ink(message)]
        pub fn current_token_id(&self) -> TokenId {
            self.token_counter
        }

        /// Returns the admin account
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        /// Internal helper to add a token to an owner
        fn add_token_to_owner(&mut self, to: AccountId, _token_id: TokenId) -> Result<(), Error> {
            let count = self.owner_token_count.get(to).unwrap_or(0);
            self.owner_token_count.insert(to, &(count + 1));
            Ok(())
        }

        /// Internal helper to remove a token from an owner
        fn remove_token_from_owner(
            &mut self,
            from: AccountId,
            _token_id: TokenId,
        ) -> Result<(), Error> {
            let count = self.owner_token_count.get(from).unwrap_or(0);
            if count == 0 {
                return Err(Error::TokenNotFound);
            }
            self.owner_token_count.insert(from, &(count - 1));
            Ok(())
        }

        /// Internal helper to update ownership history
        fn update_ownership_history(
            &mut self,
            token_id: TokenId,
            from: AccountId,
            to: AccountId,
        ) -> Result<(), Error> {
            let count = self.ownership_history_count.get(token_id).unwrap_or(0);

            let transfer_record = OwnershipTransfer {
                from,
                to,
                timestamp: self.env().block_timestamp(),
                transaction_hash: {
                    use scale::Encode;
                    let data = (&from, &to, token_id);
                    let encoded = data.encode();
                    let mut hash_bytes = [0u8; 32];
                    let len = encoded.len().min(32);
                    hash_bytes[..len].copy_from_slice(&encoded[..len]);
                    Hash::from(hash_bytes)
                },
            };

            self.ownership_history_items
                .insert((token_id, count), &transfer_record);
            self.ownership_history_count.insert(token_id, &(count + 1));

            Ok(())
        }

        /// Helper to check if token has pending bridge request
        fn has_pending_bridge_request(&self, token_id: TokenId) -> bool {
            // This is a simplified check - in a real implementation,
            // you might want to maintain a separate mapping for efficiency
            for i in 1..=self.bridge_request_counter {
                if let Some(request) = self.bridge_requests.get(i) {
                    if request.token_id == token_id
                        && matches!(
                            request.status,
                            BridgeOperationStatus::Pending | BridgeOperationStatus::Locked
                        )
                    {
                        return true;
                    }
                }
            }
            false
        }

        /// Helper to generate bridge transaction hash
        fn generate_bridge_transaction_hash(&self, request: &MultisigBridgeRequest) -> Hash {
            use scale::Encode;
            let data = (
                request.request_id,
                request.token_id,
                request.source_chain,
                request.destination_chain,
                request.sender,
                request.recipient,
                self.env().block_timestamp(),
            );
            let encoded = data.encode();
            // Simple hash: use first 32 bytes of encoded data
            let mut hash_bytes = [0u8; 32];
            let len = encoded.len().min(32);
            hash_bytes[..len].copy_from_slice(&encoded[..len]);
            Hash::from(hash_bytes)
        }

        /// Helper to estimate bridge gas usage
        fn estimate_bridge_gas_usage(&self, request: &MultisigBridgeRequest) -> u64 {
            let base_gas = 100000; // Base gas for bridge operation
            let metadata_gas = request.metadata.legal_description.len() as u64 * 100;
            let signature_gas = request.required_signatures as u64 * 5000; // Gas per signature
            base_gas + metadata_gas + signature_gas
        }

        /// Log an error for monitoring and debugging
        fn log_error(
            &mut self,
            account: AccountId,
            error_code: String,
            message: String,
            context: Vec<(String, String)>,
        ) {
            let timestamp = self.env().block_timestamp();

            // Update error count for this account and error code
            let key = (account, error_code.clone());
            let current_count = self.error_counts.get(&key).unwrap_or(0);
            self.error_counts.insert(&key, &(current_count + 1));

            // Update error rate (1 hour window)
            let window_duration = 3600_000u64; // 1 hour in milliseconds
            let rate_key = error_code.clone();
            let (mut count, window_start) =
                self.error_rates.get(&rate_key).unwrap_or((0, timestamp));

            if timestamp >= window_start + window_duration {
                // Reset window
                count = 1;
                self.error_rates.insert(&rate_key, &(count, timestamp));
            } else {
                count += 1;
                self.error_rates.insert(&rate_key, &(count, window_start));
            }

            // Add to recent errors (keep last 100)
            let log_id = self.error_log_counter;
            self.error_log_counter = self.error_log_counter.wrapping_add(1);

            // Only keep last 100 errors (simple circular buffer)
            if log_id >= 100 {
                let old_id = log_id.wrapping_sub(100);
                self.recent_errors.remove(&old_id);
            }

            let error_entry = ErrorLogEntry {
                error_code: error_code.clone(),
                message,
                account,
                timestamp,
                context,
            };
            self.recent_errors.insert(&log_id, &error_entry);
        }

        /// Get error count for an account and error code
        #[ink(message)]
        pub fn get_error_count(&self, account: AccountId, error_code: String) -> u64 {
            self.error_counts.get(&(account, error_code)).unwrap_or(0)
        }

        /// Get error rate for an error code (errors per hour)
        #[ink(message)]
        pub fn get_error_rate(&self, error_code: String) -> u64 {
            let timestamp = self.env().block_timestamp();
            let window_duration = 3600_000u64; // 1 hour

            if let Some((count, window_start)) = self.error_rates.get(&error_code) {
                if timestamp >= window_start + window_duration {
                    0 // Window expired
                } else {
                    count
                }
            } else {
                0
            }
        }

        /// Get recent error log entries (admin only)
        #[ink(message)]
        pub fn get_recent_errors(&self, limit: u32) -> Vec<ErrorLogEntry> {
            // Only admin can access error logs
            if self.env().caller() != self.admin {
                return Vec::new();
            }

            let mut errors = Vec::new();
            let start_id = if self.error_log_counter > limit as u64 {
                self.error_log_counter - limit as u64
            } else {
                0
            };

            for i in start_id..self.error_log_counter {
                if let Some(entry) = self.recent_errors.get(&i) {
                    errors.push(entry);
                }
            }

            errors
        }

        /// Enterprise-grade API: Get proposal history with pagination
        #[ink(message)]
        pub fn get_proposal_history(
            &self,
            token_id: TokenId,
            params: PaginationParams,
        ) -> PaginatedProposalHistory {
            let total_count = self.proposal_history_count.get(token_id).unwrap_or(0);
            
            // Calculate pagination
            let offset = params.offset.min(total_count);
            let limit = params.limit.min(100).min(total_count - offset); // Cap at 100
            let mut entries = Vec::new();
            
            // Retrieve entries based on sort order
            if params.sort_ascending {
                for i in offset..(offset + limit) {
                    if let Some(entry) = self.proposal_history_items.get((token_id, i)) {
                        entries.push(entry);
                    }
                }
            } else {
                // Descending order (most recent first)
                for i in (0..total_count).rev() {
                    if entries.len() >= limit as usize {
                        break;
                    }
                    if i < offset {
                        continue;
                    }
                    if let Some(entry) = self.proposal_history_items.get((token_id, i)) {
                        entries.push(entry);
                    }
                }
            }
            
            let returned_count = entries.len() as u32;
            let has_more = offset + limit < total_count;
            
            PaginatedProposalHistory {
                entries,
                pagination: PaginationInfo {
                    total_count,
                    returned_count,
                    offset,
                    limit,
                    has_more,
                },
            }
        }

        /// Enterprise-grade API: Get vote history for a proposal with pagination
        #[ink(message)]
        pub fn get_vote_history(
            &self,
            token_id: TokenId,
            proposal_id: u64,
            params: PaginationParams,
        ) -> PaginatedVoteHistory {
            let total_count = self.vote_history_count.get((token_id, proposal_id)).unwrap_or(0);
            
            // Calculate pagination
            let offset = params.offset.min(total_count);
            let limit = params.limit.min(100).min(total_count - offset); // Cap at 100
            let mut entries = Vec::new();
            
            // Retrieve entries based on sort order
            if params.sort_ascending {
                for i in offset..(offset + limit) {
                    if let Some(entry) = self.vote_history_items.get((token_id, proposal_id, i)) {
                        entries.push(entry);
                    }
                }
            } else {
                // Descending order (most recent first)
                for i in (0..total_count).rev() {
                    if entries.len() >= limit as usize {
                        break;
                    }
                    if i < offset {
                        continue;
                    }
                    if let Some(entry) = self.vote_history_items.get((token_id, proposal_id, i)) {
                        entries.push(entry);
                    }
                }
            }
            
            let returned_count = entries.len() as u32;
            let has_more = offset + limit < total_count;
            
            PaginatedVoteHistory {
                entries,
                pagination: PaginationInfo {
                    total_count,
                    returned_count,
                    offset,
                    limit,
                    has_more,
                },
            }
        }

        /// Enterprise-grade API: Get execution history with pagination
        #[ink(message)]
        pub fn get_execution_history(
            &self,
            params: PaginationParams,
        ) -> PaginatedExecutionHistory {
            let total_count = self.execution_history_count;
            
            // Calculate pagination
            let offset = params.offset.min(total_count);
            let limit = params.limit.min(100).min(total_count - offset); // Cap at 100
            let mut entries = Vec::new();
            
            // Retrieve entries based on sort order
            if params.sort_ascending {
                for i in offset..(offset + limit) {
                    if let Some(entry) = self.execution_history_items.get(&i) {
                        entries.push(entry);
                    }
                }
            } else {
                // Descending order (most recent first)
                for i in (0..total_count).rev() {
                    if entries.len() >= limit as usize {
                        break;
                    }
                    if i < offset {
                        continue;
                    }
                    if let Some(entry) = self.execution_history_items.get(&i) {
                        entries.push(entry);
                    }
                }
            }
            
            let returned_count = entries.len() as u32;
            let has_more = offset + limit < total_count;
            
            PaginatedExecutionHistory {
                entries,
                pagination: PaginationInfo {
                    total_count,
                    returned_count,
                    offset,
                    limit,
                    has_more,
                },
            }
        }

        /// Enterprise-grade API: Record slashing event (admin/governance only)
        /// with role-based ACL guard and cooldown protection
        #[ink(message)]
        pub fn record_slashing(
            &mut self,
            target: AccountId,
            role: SlashingRole,
            reason: SlashingReason,
            slashed_amount: u128,
            authority: AccountId,
        ) -> Result<(), Error> {
            // Only admin can record slashing directly
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            
            // Check if target is blacklisted from being slashed
            if self.slashing_blacklist.get(&target).unwrap_or(false) {
                return Err(Error::SlashBlacklisted);
            }
            
            // Convert SlashingRole to u8 for storage key
            let role_id = self.role_to_id(&role);
            
            // Check cooldown period - prevent repeated slashing for same target/role
            let cooldown_key = (target, role_id);
            let last_slash = self.slashing_cooldowns.get(&cooldown_key).unwrap_or(0);
            let now = self.env().block_timestamp();
            if now.saturating_sub(last_slash) < self.slashing_cooldown_period {
                return Err(Error::SlashCooldownActive);
            }
            
            // Count repeat offenses for history
            let mut repeat_count = 0u32;
            for i in 0..self.slashing_history_count {
                if let Some(entry) = self.slashing_history_items.get(&i) {
                    if entry.target == target && entry.role == role {
                        repeat_count += 1;
                    }
                }
            }
            
            // Create slashing history entry
            let tx_hash = Hash::from([0u8; 32]); // In real implementation, get actual tx hash
            let slashing_entry = SlashingHistoryEntry {
                target: target.clone(),
                role,
                reason,
                slashed_amount,
                slashed_at: now,
                transaction_hash: tx_hash,
                authority,
                repeat_offense_count: repeat_count,
            };
            
            // Store in history
            let history_count = self.slashing_history_count;
            self.slashing_history_items.insert(&history_count, &slashing_entry);
            self.slashing_history_count = history_count + 1;
            
            // Update cooldown timestamp
            self.slashing_cooldowns.insert(&cooldown_key, &now);
            
            // Emit event for slashing
            self.env().emit_event(SlashingRecorded {
                target,
                role,
                reason,
                slashed_amount,
                authority,
                cooldown_until: now.saturating_add(self.slashing_cooldown_period),
            });
            
            Ok(())
        }
        
        /// Record slashing with explicit role-scope ACL
        /// Only accounts with the required ACL permission can slash specific roles
        #[ink(message)]
        pub fn record_slashing_with_acl(
            &mut self,
            target: AccountId,
            target_role: SlashingRole,
            reason: SlashingReason,
            slashed_amount: u128,
            caller_role: Role,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Check if caller has the required ACL permission
            let caller_role_id = self.role_to_id_internal(&caller_role);
            let target_role_id = self.role_to_id(&target_role);
            let acl_key = (caller_role_id, target_role_id);
            
            if !self.slashing_acl.get(&acl_key).unwrap_or(false) {
                return Err(Error::SlashingACLRequired);
            }
            
            // Check if target is blacklisted from being slashed
            if self.slashing_blacklist.get(&target).unwrap_or(false) {
                return Err(Error::SlashBlacklisted);
            }
            
            // Check cooldown period
            let cooldown_key = (target, target_role_id);
            let last_slash = self.slashing_cooldowns.get(&cooldown_key).unwrap_or(0);
            let now = self.env().block_timestamp();
            if now.saturating_sub(last_slash) < self.slashing_cooldown_period {
                return Err(Error::SlashCooldownActive);
            }
            
            // Count repeat offenses
            let mut repeat_count = 0u32;
            for i in 0..self.slashing_history_count {
                if let Some(entry) = self.slashing_history_items.get(&i) {
                    if entry.target == target && entry.role == target_role {
                        repeat_count += 1;
                    }
                }
            }
            
            // Create slashing history entry
            let tx_hash = Hash::from([0u8; 32]);
            let slashing_entry = SlashingHistoryEntry {
                target: target.clone(),
                role: target_role.clone(),
                reason,
                slashed_amount,
                slashed_at: now,
                transaction_hash: tx_hash,
                authority: caller,
                repeat_offense_count: repeat_count,
            };
            
            // Store in history
            let history_count = self.slashing_history_count;
            self.slashing_history_items.insert(&history_count, &slashing_entry);
            self.slashing_history_count = history_count + 1;
            
            // Update cooldown timestamp
            self.slashing_cooldowns.insert(&cooldown_key, &now);
            
            // Emit event
            self.env().emit_event(SlashingRecorded {
                target,
                role: target_role,
                reason,
                slashed_amount,
                authority: caller,
                cooldown_until: now.saturating_add(self.slashing_cooldown_period),
            });
            
            Ok(())
        }
        
        /// Set slashing blacklist for an account (admin only)
        #[ink(message)]
        pub fn set_slashing_blacklist(
            &mut self,
            account: AccountId,
            blacklisted: bool,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            
            self.slashing_blacklist.insert(&account, &blacklisted);
            
            self.env().emit_event(SlashingBlacklistUpdated {
                account,
                blacklisted,
                updated_by: caller,
            });
            
            Ok(())
        }
        
        /// Get slashing eligibility for a target account
        #[ink(message)]
        pub fn get_slashing_eligibility(
            &self,
            target: AccountId,
            role: SlashingRole,
        ) -> SlashingEligibility {
            // Check if blacklisted
            let is_blacklisted = self.slashing_blacklist.get(&target).unwrap_or(false);
            
            // Check cooldown status
            let role_id = self.role_to_id(&role);
            let cooldown_key = (target, role_id);
            let last_slash = self.slashing_cooldowns.get(&cooldown_key).unwrap_or(0);
            let now = self.env().block_timestamp();
            let cooldown_remaining = if last_slash == 0 {
                0
            } else {
                let elapsed = now.saturating_sub(last_slash);
                if elapsed >= self.slashing_cooldown_period {
                    0
                } else {
                    self.slashing_cooldown_period.saturating_sub(elapsed)
                }
            };
            
            SlashingEligibility {
                target,
                role,
                is_blacklisted,
                cooldown_remaining,
                cooldown_period: self.slashing_cooldown_period,
                last_slash_timestamp: last_slash,
                can_be_slashed: !is_blacklisted && cooldown_remaining == 0,
            }
        }
        
        /// Set slashing cooldown period (admin only)
        #[ink(message)]
        pub fn set_slashing_cooldown(&mut self, period_seconds: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            
            self.slashing_cooldown_period = period_seconds;
            Ok(())
        }
        
        /// Set slashing ACL for a role (admin only)
        /// slasher_role: the role performing the slashing
        /// target_role: the role being slashed
        /// allowed: whether slasher can slash target
        #[ink(message)]
        pub fn set_slashing_acl(
            &mut self,
            slasher_role: Role,
            target_role: SlashingRole,
            allowed: bool,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }
            
            let slasher_role_id = self.role_to_id_internal(&slasher_role);
            let target_role_id = self.role_to_id(&target_role);
            let acl_key = (slasher_role_id, target_role_id);
            
            self.slashing_acl.insert(&acl_key, &allowed);
            
            self.env().emit_event(SlashingACLUpdated {
                slasher_role,
                target_role,
                allowed,
                updated_by: caller,
            });
            
            Ok(())
        }
        
        /// Check if a role can slash another role
        #[ink(message)]
        pub fn can_role_slash(&self, slasher_role: Role, target_role: SlashingRole) -> bool {
            let slasher_role_id = self.role_to_id_internal(&slasher_role);
            let target_role_id = self.role_to_id(&target_role);
            let acl_key = (slasher_role_id, target_role_id);
            self.slashing_acl.get(&acl_key).unwrap_or(false)
        }
        
        /// Get slashing cooldown period
        #[ink(message)]
        pub fn get_slashing_cooldown_period(&self) -> u64 {
            self.slashing_cooldown_period
        }
        
        /// Get last slash timestamp for a target and role
        #[ink(message)]
        pub fn get_last_slash_timestamp(&self, target: AccountId, role: SlashingRole) -> u64 {
            let role_id = self.role_to_id(&role);
            let cooldown_key = (target, role_id);
            self.slashing_cooldowns.get(&cooldown_key).unwrap_or(0)
        }

        /// Enterprise-grade API: Get slashing history with pagination
        #[ink(message)]
        pub fn get_slashing_history(
            &self,
            target: Option<AccountId>,
            role: Option<SlashingRole>,
            params: PaginationParams,
        ) -> PaginatedSlashingHistory {
            let total_count = self.slashing_history_count;
            
            // Filter entries if filters are provided
            let mut all_entries = Vec::new();
            for i in 0..total_count {
                if let Some(entry) = self.slashing_history_items.get(&i) {
                    let matches_target = target.is_none() || entry.target == target.as_ref().unwrap();
                    let matches_role = role.is_none() || entry.role == *role.as_ref().unwrap();
                    
                    if matches_target && matches_role {
                        all_entries.push((i, entry));
                    }
                }
            }
            
            let filtered_total = all_entries.len() as u32;
            
            // Calculate pagination
            let offset = params.offset.min(filtered_total);
            let limit = params.limit.min(100).min(filtered_total - offset); // Cap at 100
            let mut entries = Vec::new();
            
            // Sort by index (which represents chronological order)
            if params.sort_ascending {
                for (_, entry) in all_entries.iter().skip(offset as usize).take(limit as usize) {
                    entries.push(entry.clone());
                }
            } else {
                // Descending order (most recent first)
                for (_, entry) in all_entries.iter().rev().skip((filtered_total - offset - limit) as usize).take(limit as usize) {
                    entries.push(entry.clone());
                }
            }
            
            let returned_count = entries.len() as u32;
            let has_more = offset + limit < filtered_total;
            
            PaginatedSlashingHistory {
                entries,
                pagination: PaginationInfo {
                    total_count: filtered_total,
                    returned_count,
                    offset,
                    limit,
                    has_more,
                },
            }
        }

        // =============================================================================
        // Multi-Role Identity Management
        // =============================================================================

        /// Grant a role to an account (admin only)
        #[ink(message)]
        pub fn grant_role(
            &mut self,
            account: AccountId,
            role: Role,
            expires_at: Option<u64>,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Only admin can grant roles
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            // Check if account already has this role
            if self.has_role(account, role) {
                return Err(Error::ComplianceFailed); // Reusing error for "already exists"
            }

            let current_time = self.env().block_timestamp();
            let role_info = RoleInfo {
                role,
                granted_at: current_time,
                granted_by: caller,
                expires_at,
                is_active: true,
            };

            // Add role to account's roles
            let mut roles = self.role_assignments.get(account).unwrap_or_default();
            roles.push(role);
            self.role_assignments.insert(account, &roles);

            // Store role info
            self.role_info.insert((account, role), &role_info);

            self.env().emit_event(RoleGranted {
                account,
                role,
                granted_by: caller,
                granted_at: current_time,
                expires_at,
            });

            Ok(())
        }

        /// Revoke a role from an account (admin only)
        #[ink(message)]
        pub fn revoke_role(&mut self, account: AccountId, role: Role) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Only admin can revoke roles
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            // Check if account has this role
            if !self.has_role(account, role) {
                return Err(Error::PropertyNotFound); // Reusing error for "not found"
            }

            // Remove role from account's roles
            let mut roles = self.role_assignments.get(account).unwrap_or_default();
            roles.retain(|&r| r != role);
            self.role_assignments.insert(account, &roles);

            // Deactivate role info
            if let Some(mut role_info) = self.role_info.get((account, role)) {
                role_info.is_active = false;
                self.role_info.insert((account, role), &role_info);
            }

            self.env().emit_event(RoleRevoked {
                account,
                role,
                revoked_by: caller,
                revoked_at: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Check if an account has a specific role
        #[ink(message)]
        pub fn has_role(&self, account: AccountId, role: Role) -> bool {
            if let Some(roles) = self.role_assignments.get(account) {
                for assigned_role in roles.iter() {
                    if *assigned_role == role {
                        // Check if role is active and not expired
                        if let Some(role_info) = self.role_info.get((account, role)) {
                            if !role_info.is_active {
                                return false;
                            }
                            if let Some(expires_at) = role_info.expires_at {
                                if self.env().block_timestamp() > expires_at {
                                    return false;
                                }
                            }
                            return true;
                        }
                    }
                }
            }
            false
        }

        /// Get all roles for an account
        #[ink(message)]
        pub fn get_roles_for_account(&self, account: AccountId) -> Vec<Role> {
            self.role_assignments.get(account).unwrap_or_default()
        }

        /// Get detailed role information
        #[ink(message)]
        pub fn get_role_info(&self, account: AccountId, role: Role) -> Option<RoleInfo> {
            self.role_info.get((account, role))
        }

        /// Request a role transfer with timelock (for Admin role)
        #[ink(message)]
        pub fn request_admin_transfer(
            &mut self,
            new_admin: AccountId,
        ) -> Result<u64, Error> {
            let caller = self.env().caller();
            
            // Only current admin can request transfer
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            let current_time = self.env().block_timestamp();
            let executable_at = current_time + self.role_timelock_seconds;

            let transfer_request = RoleTransferRequest {
                from_role: Role::Admin,
                from_account: caller,
                to_account: new_admin,
                requested_at: current_time,
                executable_at,
                is_executed: false,
            };

            self.role_transfer_counter += 1;
            let request_id = self.role_transfer_counter;
            self.role_transfer_requests.insert(request_id, &transfer_request);

            self.env().emit_event(AdminTransferRequested {
                from: caller,
                to: new_admin,
                request_id,
                requested_at: current_time,
                executable_at,
            });

            Ok(request_id)
        }

        /// Execute a pending admin transfer
        #[ink(message)]
        pub fn execute_admin_transfer(&mut self, request_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            
            let mut transfer_request = self
                .role_transfer_requests
                .get(request_id)
                .ok_or(Error::ProposalNotFound)?;

            // Check if request is executed
            if transfer_request.is_executed {
                return Err(Error::ProposalClosed);
            }

            // Check if timelock has passed
            let current_time = self.env().block_timestamp();
            if current_time < transfer_request.executable_at {
                return Err(Error::ComplianceFailed); // Too early
            }

            // Verify caller is the intended recipient
            if caller != transfer_request.to_account {
                return Err(Error::Unauthorized);
            }

            // Transfer admin role
            self.admin = transfer_request.to_account;
            transfer_request.is_executed = true;
            self.role_transfer_requests.insert(request_id, &transfer_request);

            // Grant Admin role to new admin
            let role_info = RoleInfo {
                role: Role::Admin,
                granted_at: current_time,
                granted_by: transfer_request.from_account,
                expires_at: None,
                is_active: true,
            };

            let mut roles = self.role_assignments.get(transfer_request.to_account).unwrap_or_default();
            roles.push(Role::Admin);
            self.role_assignments.insert(transfer_request.to_account, &roles);
            self.role_info.insert((transfer_request.to_account, Role::Admin), &role_info);

            // Revoke Admin role from old admin
            if let Some(mut old_roles) = self.role_assignments.get(transfer_request.from_account) {
                old_roles.retain(|&r| r != Role::Admin);
                self.role_assignments.insert(transfer_request.from_account, &old_roles);
            }

            self.env().emit_event(AdminTransferExecuted {
                request_id,
                from: transfer_request.from_account,
                to: transfer_request.to_account,
                executed_at: current_time,
            });

            Ok(())
        }

        /// Cancel a pending admin transfer
        #[ink(message)]
        pub fn cancel_admin_transfer(&mut self, request_id: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            
            // Only original requester can cancel
            let transfer_request = self
                .role_transfer_requests
                .get(request_id)
                .ok_or(Error::ProposalNotFound)?;

            if caller != transfer_request.from_account {
                return Err(Error::Unauthorized);
            }

            if transfer_request.is_executed {
                return Err(Error::ProposalClosed);
            }

            self.role_transfer_requests.remove(request_id);

            self.env().emit_event(AdminTransferCancelled {
                request_id,
                cancelled_at: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Set the timelock period for role transfers (admin only)
        #[ink(message)]
        pub fn set_role_timelock_seconds(&mut self, seconds: u64) -> Result<(), Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            self.role_timelock_seconds = seconds;

            self.env().emit_event(RoleTimelockUpdated {
                old_period: self.role_timelock_seconds,
                new_period: seconds,
                updated_at: self.env().block_timestamp(),
            });

            Ok(())
        }

        /// Get the current timelock period
        #[ink(message)]
        pub fn get_role_timelock_seconds(&self) -> u64 {
            self.role_timelock_seconds
        }

        /// Log an annual review for a role holder (admin only)
        #[ink(message)]
        pub fn log_annual_review(
            &mut self,
            account: AccountId,
            role: Role,
            performance_score: u32,
            notes: String,
            is_renewed: bool,
        ) -> Result<u64, Error> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::Unauthorized);
            }

            // Check if account has this role
            if !self.has_role(account, role) {
                return Err(Error::PropertyNotFound);
            }

            let review_log = AnnualReviewLog {
                account,
                role,
                reviewed_at: self.env().block_timestamp(),
                reviewer: caller,
                performance_score,
                notes,
                is_renewed,
            };

            self.annual_review_counter += 1;
            let log_id = self.annual_review_counter;
            self.annual_review_logs.insert(log_id, &review_log);

            // If renewed and role has expiration, extend it
            if is_renewed {
                if let Some(mut role_info) = self.role_info.get((account, role)) {
                    if let Some(expires_at) = role_info.expires_at {
                        // Extend by 1 year (31536000 seconds)
                        role_info.expires_at = Some(expires_at + 31536000);
                        self.role_info.insert((account, role), &role_info);
                    }
                }
            }

            self.env().emit_event(AnnualReviewLogged {
                account,
                role,
                log_id,
                performance_score,
                is_renewed,
                reviewed_at: self.env().block_timestamp(),
            });

            Ok(log_id)
        }

        /// Get annual review logs for an account and role
        #[ink(message)]
        pub fn get_annual_reviews(
            &self,
            account: AccountId,
            role: Role,
            offset: u32,
            limit: u32,
        ) -> Vec<AnnualReviewLog> {
            let mut reviews = Vec::new();
            let total = self.annual_review_counter;
            
            // Iterate through logs and filter by account and role
            for i in 1..=total {
                if let Some(log) = self.annual_review_logs.get(i) {
                    if log.account == account && log.role == role {
                        reviews.push(log);
                    }
                }
            }

            // Apply pagination
            let start = offset as usize;
            let end = core::cmp::min(start + limit as usize, reviews.len());
            
            if start >= reviews.len() {
                return Vec::new();
            }

            reviews[start..end].to_vec()
        }

        /// Check if caller has Admin role
        fn require_admin(&self) -> Result<(), Error> {
            let caller = self.env().caller();
            if self.has_role(caller, Role::Admin) {
                Ok(())
            } else {
                Err(Error::Unauthorized)
            }
        }

        /// Check if caller has Auditor role
        fn require_auditor(&self) -> Result<(), Error> {
            let caller = self.env().caller();
            if self.has_role(caller, Role::Auditor) {
                Ok(())
            } else {
                Err(Error::Unauthorized)
            }
        }

        /// Check if caller has LiquidityManager role
        fn require_liquidity_manager(&self) -> Result<(), Error> {
            let caller = self.env().caller();
            if self.has_role(caller, Role::LiquidityManager) {
                Ok(())
            } else {
                Err(Error::Unauthorized)
            }
        }

        /// Check if caller has GovernanceOperator role
        fn require_governance_operator(&self) -> Result<(), Error> {
            let caller = self.env().caller();
            if self.has_role(caller, Role::GovernanceOperator) {
                Ok(())
            } else {
                Err(Error::Unauthorized)
            }
        }
    }

    // Event definitions for role management
    /// Event emitted when a role is granted
    #[ink(event)]
    pub struct RoleGranted {
        #[indexed]
        account: AccountId,
        #[indexed]
        role: Role,
        granted_by: AccountId,
        granted_at: u64,
        expires_at: Option<u64>,
    }

    /// Event emitted when a role is revoked
    #[ink(event)]
    pub struct RoleRevoked {
        #[indexed]
        account: AccountId,
        #[indexed]
        role: Role,
        revoked_by: AccountId,
        revoked_at: u64,
    }

    /// Event emitted when admin transfer is requested
    #[ink(event)]
    pub struct AdminTransferRequested {
        #[indexed]
        from: AccountId,
        #[indexed]
        to: AccountId,
        request_id: u64,
        requested_at: u64,
        executable_at: u64,
    }

    /// Event emitted when admin transfer is executed
    #[ink(event)]
    pub struct AdminTransferExecuted {
        request_id: u64,
        #[indexed]
        from: AccountId,
        #[indexed]
        to: AccountId,
        executed_at: u64,
    }

    /// Event emitted when admin transfer is cancelled
    #[ink(event)]
    pub struct AdminTransferCancelled {
        request_id: u64,
        cancelled_at: u64,
    }

    /// Event emitted when role timelock is updated
    #[ink(event)]
    pub struct RoleTimelockUpdated {
        old_period: u64,
        new_period: u64,
        updated_at: u64,
    }

    /// Event emitted when annual review is logged
    #[ink(event)]
    pub struct AnnualReviewLogged {
        #[indexed]
        account: AccountId,
        #[indexed]
        role: Role,
        log_id: u64,
        performance_score: u32,
        is_renewed: bool,
        reviewed_at: u64,
    }

    /// Event emitted when emergency pause is updated
    #[ink(event)]
    pub struct EmergencyPauseUpdated {
        paused: bool,
        updated_by: AccountId,
        updated_at: u64,
    }

    /// Event emitted when token is flagged for compliance review
    #[ink(event)]
    pub struct ComplianceFlagged {
        #[indexed]
        token_id: TokenId,
        flagged_by: AccountId,
        reason: String,
        flagged_at: u64,
    }

    /// Event emitted when compliance flag is cleared
    #[ink(event)]
    pub struct ComplianceFlagCleared {
        #[indexed]
        token_id: TokenId,
        cleared_by: AccountId,
        notes: String,
        cleared_at: u64,
    }

    /// Event emitted when dividend parameters are updated
    #[ink(event)]
    pub struct DividendParametersUpdated {
        #[indexed]
        token_id: TokenId,
        new_dividend_rate: u128,
        updated_by: AccountId,
        updated_at: u64,
    }

    /// Event emitted when pool risk parameters are adjusted
    #[ink(event)]
    pub struct PoolRiskParametersAdjusted {
        #[indexed]
        token_id: TokenId,
        risk_adjustment: i32,
        adjusted_by: AccountId,
        adjusted_at: u64,
    }

    /// Event emitted when liquidity pool fee is updated
    #[ink(event)]
    pub struct LiquidityPoolFeeUpdated {
        #[indexed]
        token_id: TokenId,
        fee_rate: u128,
        updated_by: AccountId,
        updated_at: u64,
    }

    /// Event emitted when governance proposal is executed
    #[ink(event)]
    pub struct GovernanceProposalExecuted {
        #[indexed]
        token_id: TokenId,
        proposal_id: u64,
        executed_by: AccountId,
        executed_at: u64,
    }

    /// Event emitted when governance proposal is vetoed
    #[ink(event)]
    pub struct GovernanceProposalVetoed {
        #[indexed]
        token_id: TokenId,
        proposal_id: u64,
        vetoed_by: AccountId,
        reason: String,
        vetoed_at: u64,
    }

    /// Event emitted when a slashing is recorded
    #[ink(event)]
    pub struct SlashingRecorded {
        #[ink(topic)]
        pub target: AccountId,
        #[ink(topic)]
        pub role: SlashingRole,
        pub reason: SlashingReason,
        pub slashed_amount: u128,
        pub authority: AccountId,
        pub cooldown_until: u64,
    }

    /// Event emitted when slashing blacklist is updated
    #[ink(event)]
    pub struct SlashingBlacklistUpdated {
        #[ink(topic)]
        pub account: AccountId,
        pub blacklisted: bool,
        pub updated_by: AccountId,
    }

    /// Event emitted when slashing ACL is updated
    #[ink(event)]
    pub struct SlashingACLUpdated {
        #[ink(topic)]
        pub slasher_role: Role,
        #[ink(topic)]
        pub target_role: SlashingRole,
        pub allowed: bool,
        pub updated_by: AccountId,
    }

    // Unit tests for the PropertyToken contract
    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::{test, DefaultEnvironment};

        fn setup_contract() -> PropertyToken {
            PropertyToken::new()
        }

        #[ink::test]
        fn test_constructor_works() {
            let contract = setup_contract();
            assert_eq!(contract.total_supply(), 0);
            assert_eq!(contract.current_token_id(), 0);
        }

        #[ink::test]
        fn test_register_property_with_token() {
            let mut contract = setup_contract();

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let result = contract.register_property_with_token(metadata.clone());
            assert!(result.is_ok());

            let token_id = result.expect("Token registration should succeed in test");
            assert_eq!(token_id, 1);
            assert_eq!(contract.total_supply(), 1);
        }

        #[ink::test]
        fn test_balance_of() {
            let mut contract = setup_contract();

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let _token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");
            let _caller = AccountId::from([1u8; 32]);

            // Set up mock caller for the test
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            assert_eq!(contract.balance_of(accounts.alice), 1);
        }

        #[ink::test]
        fn test_attach_legal_document() {
            let mut contract = setup_contract();

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let doc_hash = Hash::from([1u8; 32]);
            let doc_type = String::from("Deed");

            let result = contract.attach_legal_document(token_id, doc_hash, doc_type);
            assert!(result.is_ok());
        }

        #[ink::test]
        fn test_verify_compliance() {
            let mut contract = setup_contract();

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            let _accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(contract.admin());

            let result = contract.verify_compliance(token_id, true);
            assert!(result.is_ok());

            let compliance_info = contract
                .compliance_flags
                .get(&token_id)
                .expect("Compliance info should exist after verification");
            assert!(compliance_info.verified);
        }

        #[ink::test]
        fn test_e2e_governance_execution_and_paused_bridge_failure_path() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let metadata = PropertyMetadata {
                location: String::from("Governance House"),
                size: 1800,
                legal_description: String::from("Governance smoke flow"),
                valuation: 900000,
                documents_url: String::from("ipfs://governance-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed");
            assert!(contract.issue_shares(token_id, accounts.alice, 1_000).is_ok());
            assert!(contract.transfer_shares(accounts.alice, accounts.bob, token_id, 400).is_ok());
            assert_eq!(contract.share_balance_of(accounts.alice, token_id), 600);
            assert_eq!(contract.share_balance_of(accounts.bob, token_id), 400);

            let proposal_id = contract
                .create_proposal(token_id, 700, Hash::from([9u8; 32]))
                .expect("Owner should be able to create proposal");
            let proposal_before_vote = contract
                .proposals
                .get((token_id, proposal_id))
                .expect("proposal exists");
            assert_eq!(proposal_before_vote.status, ProposalStatus::Open);
            assert_eq!(proposal_before_vote.for_votes, 0);
            assert_eq!(proposal_before_vote.against_votes, 0);

            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert!(contract.vote(token_id, proposal_id, true).is_ok());
            let proposal_after_bob = contract
                .proposals
                .get((token_id, proposal_id))
                .expect("proposal exists after first vote");
            assert_eq!(proposal_after_bob.for_votes, 400);
            assert_eq!(proposal_after_bob.against_votes, 0);

            test::set_caller::<DefaultEnvironment>(accounts.alice);
            assert!(contract.vote(token_id, proposal_id, true).is_ok());
            let executed = contract
                .execute_proposal(token_id, proposal_id)
                .expect("proposal execution should succeed");
            assert!(executed);

            let proposal_after_execution = contract
                .proposals
                .get((token_id, proposal_id))
                .expect("proposal exists after execution");
            assert_eq!(proposal_after_execution.for_votes, 1_000);
            assert_eq!(proposal_after_execution.against_votes, 0);
            assert_eq!(proposal_after_execution.status, ProposalStatus::Executed);

            let duplicate_vote = contract.vote(token_id, proposal_id, true);
            assert_eq!(duplicate_vote, Err(Error::ProposalClosed));
            let duplicate_execution = contract.execute_proposal(token_id, proposal_id);
            assert_eq!(duplicate_execution, Err(Error::ProposalClosed));

            assert!(contract.set_emergency_pause(true).is_ok());
            let paused_bridge = contract.initiate_bridge_multisig(
                token_id,
                2,
                accounts.charlie,
                2,
                None,
            );
            assert_eq!(paused_bridge, Err(Error::BridgePaused));

            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let unauthorized_pause = contract.set_emergency_pause(false);
            assert_eq!(unauthorized_pause, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_governance_rejects_proposal_without_quorum() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let metadata = PropertyMetadata {
                location: String::from("Rejected Proposal House"),
                size: 900,
                legal_description: String::from("Proposal rejection path"),
                valuation: 250000,
                documents_url: String::from("ipfs://reject-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed");
            assert!(contract.issue_shares(token_id, accounts.alice, 1_000).is_ok());
            assert!(contract.transfer_shares(accounts.alice, accounts.bob, token_id, 300).is_ok());

            let proposal_id = contract
                .create_proposal(token_id, 800, Hash::from([7u8; 32]))
                .expect("Owner should be able to create proposal");

            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert!(contract.vote(token_id, proposal_id, false).is_ok());
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            assert!(contract.vote(token_id, proposal_id, true).is_ok());

            let passed = contract
                .execute_proposal(token_id, proposal_id)
                .expect("execution should settle the proposal");
            assert!(!passed);

            let rejected = contract
                .proposals
                .get((token_id, proposal_id))
                .expect("proposal stored");
            assert_eq!(rejected.for_votes, 700);
            assert_eq!(rejected.against_votes, 300);
            assert_eq!(rejected.status, ProposalStatus::Rejected);
        }

        // ============================================================================
        // EDGE CASE TESTS
        // ============================================================================

        #[ink::test]
        fn test_transfer_from_nonexistent_token() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            let result = contract.transfer_from(accounts.alice, accounts.bob, 999);
            assert_eq!(result, Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn test_transfer_from_unauthorized_caller() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            // Bob tries to transfer Alice's token without approval
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.transfer_from(accounts.alice, accounts.bob, token_id);
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_approve_nonexistent_token() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            let result = contract.approve(accounts.bob, 999);
            assert_eq!(result, Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn test_approve_unauthorized_caller() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            // Bob tries to approve without being owner or operator
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.approve(accounts.charlie, token_id);
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_owner_of_nonexistent_token() {
            let contract = setup_contract();

            assert_eq!(contract.owner_of(0), None);
            assert_eq!(contract.owner_of(1), None);
            assert_eq!(contract.owner_of(u64::MAX), None);
        }

        #[ink::test]
        fn test_balance_of_nonexistent_account() {
            let contract = setup_contract();
            let nonexistent = AccountId::from([0xFF; 32]);

            assert_eq!(contract.balance_of(nonexistent), 0);
        }

        #[ink::test]
        fn test_attach_document_to_nonexistent_token() {
            let mut contract = setup_contract();
            let doc_hash = Hash::from([1u8; 32]);

            let result = contract.attach_legal_document(999, doc_hash, "Deed".to_string());
            assert_eq!(result, Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn test_attach_document_unauthorized() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            // Bob tries to attach document
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let doc_hash = Hash::from([1u8; 32]);
            let result = contract.attach_legal_document(token_id, doc_hash, "Deed".to_string());
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_verify_compliance_nonexistent_token() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let result = contract.verify_compliance(999, true);
            assert_eq!(result, Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn test_initiate_bridge_invalid_chain() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            // Try to bridge to unsupported chain
            let result = contract.initiate_bridge_multisig(
                token_id,
                999, // Invalid chain ID
                accounts.bob,
                2,    // required_signatures
                None, // timeout_blocks
            );

            assert_eq!(result, Err(Error::InvalidChain));
        }

        #[ink::test]
        fn test_initiate_bridge_nonexistent_token() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            let result = contract.initiate_bridge_multisig(
                999,          // nonexistent token_id
                2,            // destination_chain
                accounts.bob, // recipient
                2,            // required_signatures
                None,         // timeout_blocks
            );

            assert_eq!(result, Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn test_sign_bridge_request_nonexistent() {
            let mut contract = setup_contract();
            let _accounts = test::default_accounts::<DefaultEnvironment>();

            let result = contract.sign_bridge_request(999, true);
            assert_eq!(result, Err(Error::InvalidRequest));
        }

        #[ink::test]
        fn test_register_multiple_properties_increments_ids() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            for i in 1..=10 {
                let metadata = PropertyMetadata {
                    location: format!("Property {}", i),
                    size: 1000 + i,
                    legal_description: format!("Description {}", i),
                    valuation: 100_000 + (i as u128 * 1000),
                    documents_url: format!("ipfs://prop{}", i),
                };

                let token_id = contract
                    .register_property_with_token(metadata)
                    .expect("Token registration should succeed in test");
                assert_eq!(token_id, i);
                assert_eq!(contract.total_supply(), i);
            }
        }

        #[ink::test]
        fn test_transfer_preserves_total_supply() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            test::set_caller::<DefaultEnvironment>(accounts.alice);

            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };

            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed in test");

            let initial_supply = contract.total_supply();

            contract
                .transfer_from(accounts.alice, accounts.bob, token_id)
                .expect("Transfer should succeed");

            // Total supply should remain constant
            assert_eq!(contract.total_supply(), initial_supply);
        }

        #[ink::test]
        fn test_balance_of_batch_empty_vectors() {
            let contract = setup_contract();

            let result = contract.balance_of_batch(Vec::new(), Vec::new());
            assert_eq!(result, Vec::<u128>::new());
        }

        #[ink::test]
        fn test_get_error_count_nonexistent() {
            let contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            let count = contract.get_error_count(accounts.alice, "NONEXISTENT".to_string());
            assert_eq!(count, 0);
        }

        #[ink::test]
        fn test_get_error_rate_nonexistent() {
            let contract = setup_contract();

            let rate = contract.get_error_rate("NONEXISTENT".to_string());
            assert_eq!(rate, 0);
        }

        #[ink::test]
        fn test_get_recent_errors_unauthorized() {
            let contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Non-admin tries to get errors
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let errors = contract.get_recent_errors(10);
            assert_eq!(errors, Vec::new());
        }

        // =============================================================================
        // Multi-Role Identity Management Tests
        // =============================================================================

        #[ink::test]
        fn test_grant_role_admin_only() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Admin grants auditor role to Bob
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let result = contract.grant_role(accounts.bob, Role::Auditor, None);
            assert!(result.is_ok());

            // Verify Bob has auditor role
            assert!(contract.has_role(accounts.bob, Role::Auditor));
            assert!(!contract.has_role(accounts.bob, Role::LiquidityManager));

            // Non-admin tries to grant role - should fail
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.grant_role(accounts.charlie, Role::Auditor, None);
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_revoke_role_admin_only() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Admin grants then revokes auditor role
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            assert!(contract.grant_role(accounts.bob, Role::Auditor, None).is_ok());
            assert!(contract.has_role(accounts.bob, Role::Auditor));

            assert!(contract.revoke_role(accounts.bob, Role::Auditor).is_ok());
            assert!(!contract.has_role(accounts.bob, Role::Auditor));

            // Non-admin tries to revoke - should fail
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.revoke_role(accounts.charlie, Role::Auditor);
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_get_roles_for_account() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Grant multiple roles to Bob
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            assert!(contract.grant_role(accounts.bob, Role::Auditor, None).is_ok());
            assert!(contract.grant_role(accounts.bob, Role::LiquidityManager, None).is_ok());

            let roles = contract.get_roles_for_account(accounts.bob);
            assert_eq!(roles.len(), 2);
            assert!(roles.contains(&Role::Auditor));
            assert!(roles.contains(&Role::LiquidityManager));
        }

        #[ink::test]
        fn test_role_with_expiration() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Grant role with expiration (1 year from now)
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let expires_at = 31536000000; // 1 year in milliseconds
            assert!(contract
                .grant_role(accounts.bob, Role::Auditor, Some(expires_at))
                .is_ok());

            // Role should be active
            assert!(contract.has_role(accounts.bob, Role::Auditor));

            // Get role info
            let role_info = contract.get_role_info(accounts.bob, Role::Auditor);
            assert!(role_info.is_some());
            let info = role_info.unwrap();
            assert_eq!(info.role, Role::Auditor);
            assert!(info.is_active);
            assert_eq!(info.expires_at, Some(expires_at));
        }

        #[ink::test]
        fn test_auditor_flag_compliance_review() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Setup: Register a property
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let metadata = PropertyMetadata {
                location: String::from("123 Main St"),
                size: 1000,
                legal_description: String::from("Sample property"),
                valuation: 500000,
                documents_url: String::from("ipfs://sample-docs"),
            };
            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed");

            // Grant auditor role to Bob
            assert!(contract.grant_role(accounts.bob, Role::Auditor, None).is_ok());

            // Auditor flags token for compliance review
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.flag_for_compliance_review(
                token_id,
                String::from("Suspicious activity detected"),
            );
            assert!(result.is_ok());

            // Verify flag is set
            let compliance_info = contract.get_compliance_flag_status(token_id);
            assert!(compliance_info.is_some());
            assert!(!compliance_info.unwrap().verified);

            // Non-auditor tries to flag - should fail
            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            let result =
                contract.flag_for_compliance_review(token_id, String::from("Test flag"));
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_auditor_clear_compliance_flag() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Setup: Register property and flag it
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let metadata = PropertyMetadata {
                location: String::from("456 Oak Ave"),
                size: 1500,
                legal_description: String::from("Another property"),
                valuation: 750000,
                documents_url: String::from("ipfs://docs2"),
            };
            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed");

            assert!(contract.grant_role(accounts.bob, Role::Auditor, None).is_ok());

            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert!(contract
                .flag_for_compliance_review(token_id, String::from("Initial flag"))
                .is_ok());

            // Auditor clears the flag
            let result =
                contract.clear_compliance_flag(token_id, String::from("Issue resolved"));
            assert!(result.is_ok());

            // Verify flag is cleared
            let compliance_info = contract.get_compliance_flag_status(token_id);
            assert!(compliance_info.is_some());
            assert!(compliance_info.unwrap().verified);
        }

        #[ink::test]
        fn test_liquidity_manager_update_parameters() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Setup: Register property
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let metadata = PropertyMetadata {
                location: String::from("789 Pine Rd"),
                size: 2000,
                legal_description: String::from("Large property"),
                valuation: 1000000,
                documents_url: String::from("ipfs://docs3"),
            };
            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed");

            // Grant liquidity manager role to Bob
            assert!(contract.grant_role(accounts.bob, Role::LiquidityManager, None).is_ok());

            // Liquidity manager updates dividend parameters
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.update_dividend_parameters(token_id, 500);
            assert!(result.is_ok());

            // Liquidity manager adjusts pool risk parameters
            let result = contract.adjust_pool_risk_parameters(token_id, 15);
            assert!(result.is_ok());

            // Liquidity manager sets fee rate
            let result = contract.set_liquidity_pool_fee(token_id, 30); // 0.3%
            assert!(result.is_ok());

            // Non-liquidity-manager tries to update - should fail
            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            let result = contract.update_dividend_parameters(token_id, 600);
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_governance_operator_execute_proposal() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Setup: Register property and issue shares
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let metadata = PropertyMetadata {
                location: String::from("321 Elm St"),
                size: 1200,
                legal_description: String::from("Governance property"),
                valuation: 600000,
                documents_url: String::from("ipfs://gov-docs"),
            };
            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Token registration should succeed");

            assert!(contract.issue_shares(token_id, accounts.alice, 1000).is_ok());

            // Grant governance operator role to Bob
            assert!(contract.grant_role(accounts.bob, Role::GovernanceOperator, None).is_ok());

            // Create a proposal
            let proposal_id = contract
                .create_proposal(token_id, 500, Hash::from([9u8; 32]))
                .expect("Should create proposal");

            // Vote on proposal
            assert!(contract.vote(token_id, proposal_id, true).is_ok());

            // Governance operator executes proposal
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.execute_governance_proposal(token_id, proposal_id);
            assert!(result.is_ok());

            // Non-governance-operator tries to execute - should fail
            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            let result = contract.execute_governance_proposal(token_id, proposal_id);
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_admin_transfer_with_timelock() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Current admin (Alice) requests transfer to Bob
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let request_id = contract
                .request_admin_transfer(accounts.bob)
                .expect("Should request admin transfer");

            // Verify transfer request exists
            let timelock = contract.get_role_timelock_seconds();
            assert!(timelock > 0);

            // Try to execute before timelock expires - should fail
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.execute_admin_transfer(request_id);
            assert_eq!(result, Err(Error::ComplianceFailed)); // Too early

            // Note: In a real test, you would advance the blockchain timestamp
            // For now, we verify the request was created successfully
            assert!(request_id > 0);
        }

        #[ink::test]
        fn test_cancel_admin_transfer() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Admin requests transfer
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let request_id = contract
                .request_admin_transfer(accounts.bob)
                .expect("Should request admin transfer");

            // Admin cancels the transfer
            let result = contract.cancel_admin_transfer(request_id);
            assert!(result.is_ok());

            // Bob tries to execute cancelled transfer - should fail
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.execute_admin_transfer(request_id);
            assert_eq!(result, Err(Error::ProposalNotFound));
        }

        #[ink::test]
        fn test_set_role_timelock() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Admin updates timelock
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let new_timelock = 1209600; // 14 days
            let result = contract.set_role_timelock_seconds(new_timelock);
            assert!(result.is_ok());

            assert_eq!(contract.get_role_timelock_seconds(), new_timelock);

            // Non-admin tries to update - should fail
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.set_role_timelock_seconds(604800);
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_annual_review_logging() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Grant auditor role to Bob
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            assert!(contract.grant_role(accounts.bob, Role::Auditor, None).is_ok());

            // Admin logs annual review for Bob
            let log_id = contract
                .log_annual_review(
                    accounts.bob,
                    Role::Auditor,
                    85, // performance score
                    String::from("Excellent performance in claim reviews"),
                    true, // renewed
                )
                .expect("Should log annual review");

            assert!(log_id > 0);

            // Retrieve review logs
            let reviews = contract.get_annual_reviews(accounts.bob, Role::Auditor, 0, 10);
            assert_eq!(reviews.len(), 1);
            assert_eq!(reviews[0].performance_score, 85);
            assert!(reviews[0].is_renewed);

            // Non-admin tries to log review - should fail
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.log_annual_review(
                accounts.charlie,
                Role::Auditor,
                50,
                String::from("Test"),
                false,
            );
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_emergency_pause_governance_operator() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Grant governance operator role to Bob
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            assert!(contract.grant_role(accounts.bob, Role::GovernanceOperator, None).is_ok());

            // Governance operator can pause bridge
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            let result = contract.set_emergency_pause(true);
            assert!(result.is_ok());

            // Verify pause is active
            let config = contract.get_bridge_config();
            assert!(config.emergency_pause);

            // Admin can also unpause
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            assert!(contract.set_emergency_pause(false).is_ok());

            // Regular user cannot pause - should fail
            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            let result = contract.set_emergency_pause(true);
            assert_eq!(result, Err(Error::Unauthorized));
        }

        #[ink::test]
        fn test_multi_role_access_control_matrix() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();

            // Setup property
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            let metadata = PropertyMetadata {
                location: String::from("Multi-role Test"),
                size: 1000,
                legal_description: String::from("Test property"),
                valuation: 500000,
                documents_url: String::from("ipfs://test"),
            };
            let token_id = contract
                .register_property_with_token(metadata)
                .expect("Should register");

            // Grant different roles
            assert!(contract.grant_role(accounts.bob, Role::Auditor, None).is_ok());
            assert!(contract.grant_role(accounts.charlie, Role::LiquidityManager, None).is_ok());
            assert!(contract.grant_role(accounts.david, Role::GovernanceOperator, None).is_ok());

            // Auditor can flag compliance
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert!(contract
                .flag_for_compliance_review(token_id, String::from("Test"))
                .is_ok());

            // Liquidity manager CANNOT flag compliance (wrong role)
            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            assert_eq!(
                contract.flag_for_compliance_review(token_id, String::from("Test")),
                Err(Error::Unauthorized)
            );

            // Liquidity manager CAN update pool parameters
            assert!(contract
                .adjust_pool_risk_parameters(token_id, 10)
                .is_ok());

            // Auditor CANNOT update pool parameters (wrong role)
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(
                contract.adjust_pool_risk_parameters(token_id, 10),
                Err(Error::Unauthorized)
            );

            // Verify role separation works correctly
            assert!(contract.has_role(accounts.bob, Role::Auditor));
            assert!(contract.has_role(accounts.charlie, Role::LiquidityManager));
            assert!(contract.has_role(accounts.david, Role::GovernanceOperator));
        }

        // =============================================================================
        // Slashing ACL and Cooldown Tests
        // =============================================================================

        #[ink::test]
        fn test_record_slashing_with_cooldown() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            
            let target = accounts.bob;
            let role = SlashingRole::GovernanceParticipant;
            let reason = SlashingReason::GovernanceAttack;
            let slashed_amount = 1000;
            
            // First slash should succeed
            test::set_caller::<DefaultEnvironment>(contract.admin());
            assert!(contract.record_slashing(
                target,
                role.clone(),
                reason.clone(),
                slashed_amount,
                contract.admin(),
            ).is_ok());
            
            // Second slash immediately should fail due to cooldown
            assert_eq!(
                contract.record_slashing(
                    target,
                    role.clone(),
                    reason.clone(),
                    slashed_amount,
                    contract.admin(),
                ),
                Err(Error::SlashCooldownActive)
            );
        }

        #[ink::test]
        fn test_slashing_blacklist() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            
            let target = accounts.bob;
            
            // Blacklist the target
            test::set_caller::<DefaultEnvironment>(contract.admin());
            assert!(contract.set_slashing_blacklist(target, true).is_ok());
            
            // Slashing should fail due to blacklist
            assert_eq!(
                contract.record_slashing(
                    target,
                    SlashingRole::GovernanceParticipant,
                    SlashingReason::GovernanceAttack,
                    1000,
                    contract.admin(),
                ),
                Err(Error::SlashBlacklisted)
            );
            
            // Remove from blacklist
            assert!(contract.set_slashing_blacklist(target, false).is_ok());
            
            // Slashing should now succeed
            assert!(contract.record_slashing(
                target,
                SlashingRole::GovernanceParticipant,
                SlashingReason::GovernanceAttack,
                1000,
                contract.admin(),
            ).is_ok());
        }

        #[ink::test]
        fn test_get_slashing_eligibility() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            
            let target = accounts.bob;
            let role = SlashingRole::OracleProvider;
            
            // Initial eligibility - should be eligible
            let eligibility = contract.get_slashing_eligibility(target, role.clone());
            assert!(!eligibility.is_blacklisted);
            assert_eq!(eligibility.cooldown_remaining, 0);
            assert!(eligibility.can_be_slashed);
            
            // Blacklist the target
            test::set_caller::<DefaultEnvironment>(contract.admin());
            contract.set_slashing_blacklist(target, true).unwrap();
            
            // Should not be eligible now
            let eligibility = contract.get_slashing_eligibility(target, role.clone());
            assert!(eligibility.is_blacklisted);
            assert!(!eligibility.can_be_slashed);
        }

        #[ink::test]
        fn test_slashing_acl() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            
            // Set ACL: Auditor can slash GovernanceParticipant
            test::set_caller::<DefaultEnvironment>(contract.admin());
            assert!(contract.set_slashing_acl(
                Role::Auditor,
                SlashingRole::GovernanceParticipant,
                true,
            ).is_ok());
            
            // Verify ACL is set
            assert!(contract.can_role_slash(Role::Auditor, SlashingRole::GovernanceParticipant));
            
            // Verify default ACL is false
            assert!(!contract.can_role_slash(Role::Auditor, SlashingRole::OracleProvider));
            
            // Remove ACL
            assert!(contract.set_slashing_acl(
                Role::Auditor,
                SlashingRole::GovernanceParticipant,
                false,
            ).is_ok());
            assert!(!contract.can_role_slash(Role::Auditor, SlashingRole::GovernanceParticipant));
        }

        #[ink::test]
        fn test_record_slashing_with_acl() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            
            // Grant Auditor role to bob
            test::set_caller::<DefaultEnvironment>(contract.admin());
            contract.grant_role(accounts.bob, Role::Auditor, None).unwrap();
            
            // Set ACL: Auditor can slash GovernanceParticipant
            assert!(contract.set_slashing_acl(
                Role::Auditor,
                SlashingRole::GovernanceParticipant,
                true,
            ).is_ok());
            
            // Bob (Auditor) should be able to slash GovernanceParticipant
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert!(contract.record_slashing_with_acl(
                accounts.charlie,
                SlashingRole::GovernanceParticipant,
                SlashingReason::GovernanceAttack,
                1000,
                Role::Auditor,
            ).is_ok());
            
            // Bob should not be able to slash OracleProvider (no ACL)
            assert_eq!(
                contract.record_slashing_with_acl(
                    accounts.charlie,
                    SlashingRole::OracleProvider,
                    SlashingReason::OracleManipulation,
                    1000,
                    Role::Auditor,
                ),
                Err(Error::SlashingACLRequired)
            );
        }

        #[ink::test]
        fn test_slashing_cooldown_period() {
            let mut contract = setup_contract();
            
            // Default cooldown should be 86400 (24 hours)
            assert_eq!(contract.get_slashing_cooldown_period(), 86400);
            
            // Set new cooldown period
            test::set_caller::<DefaultEnvironment>(contract.admin());
            assert!(contract.set_slashing_cooldown(3600).is_ok()); // 1 hour
            
            // Verify new cooldown period
            assert_eq!(contract.get_slashing_cooldown_period(), 3600);
        }

        #[ink::test]
        fn test_get_last_slash_timestamp() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            
            let target = accounts.bob;
            let role = SlashingRole::RiskPoolProvider;
            
            // Should be 0 initially
            assert_eq!(contract.get_last_slash_timestamp(target, role.clone()), 0);
            
            // Record a slash
            test::set_caller::<DefaultEnvironment>(contract.admin());
            contract.record_slashing(
                target,
                role.clone(),
                SlashingReason::MaliciousBehavior,
                500,
                contract.admin(),
            ).unwrap();
            
            // Should now have a timestamp
            let last_slash = contract.get_last_slash_timestamp(target, role.clone());
            assert!(last_slash > 0);
        }

        #[ink::test]
        fn test_slashing_blacklist_requires_admin() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            
            // Non-admin should not be able to set blacklist
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(
                contract.set_slashing_blacklist(accounts.charlie, true),
                Err(Error::Unauthorized)
            );
        }

        #[ink::test]
        fn test_slashing_cooldown_requires_admin() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            
            // Non-admin should not be able to set cooldown
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(
                contract.set_slashing_cooldown(3600),
                Err(Error::Unauthorized)
            );
        }

        #[ink::test]
        fn test_slashing_acl_requires_admin() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            
            // Non-admin should not be able to set ACL
            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(
                contract.set_slashing_acl(
                    Role::Auditor,
                    SlashingRole::GovernanceParticipant,
                    true,
                ),
                Err(Error::Unauthorized)
            );
        }

        #[ink::test]
        fn test_slashing_role_cooldowns_are_independent() {
            let mut contract = setup_contract();
            let accounts = test::default_accounts::<DefaultEnvironment>();
            
            let target = accounts.bob;
            
            // Slash for GovernanceParticipant role
            test::set_caller::<DefaultEnvironment>(contract.admin());
            contract.record_slashing(
                target,
                SlashingRole::GovernanceParticipant,
                SlashingReason::GovernanceAttack,
                1000,
                contract.admin(),
            ).unwrap();
            
            // Slashing GovernanceParticipant again should fail
            assert_eq!(
                contract.record_slashing(
                    target,
                    SlashingRole::GovernanceParticipant,
                    SlashingReason::GovernanceAttack,
                    1000,
                    contract.admin(),
                ),
                Err(Error::SlashCooldownActive)
            );
            
            // Slashing OracleProvider should succeed (different role = different cooldown)
            assert!(contract.record_slashing(
                target,
                SlashingRole::OracleProvider,
                SlashingReason::OracleManipulation,
                1000,
                contract.admin(),
            ).is_ok());
        }
    }
}
