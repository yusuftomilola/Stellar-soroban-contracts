#![cfg_attr(not(feature = "std"), no_std)]
#![allow(dead_code)]

#[ink::contract]
mod propchain_proxy {
    use ink::env::DefaultEnvironment;

    /// Unique storage key for the proxy data to avoid collisions.
    /// bytes4(keccak256("proxy.storage")) = 0xc5f3bc7a
    #[allow(dead_code)]
    const PROXY_STORAGE_KEY: u32 = 0xC5F3BC7A;
    const ADMIN_TIMELOCK_SECS: u64 = 3 * 24 * 60 * 60;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        Unauthorized,
        UpgradeFailed,
        InvalidAdmin,
        NoPendingAdmin,
        TimelockActive,
        PendingAdminTransfer,
        NoRenounceScheduled,
    }

    #[ink(storage)]
    pub struct TransparentProxy {
        /// The address of the current implementation contract.
        code_hash: Hash,
        /// The address of the proxy admin.
        admin: AccountId,
        /// Pending admin awaiting timelock expiry and acceptance.
        pending_admin: Option<AccountId>,
        /// Block timestamp at which admin transfer was initiated.
        admin_transfer_requested_at: Option<u64>,
        /// Block timestamp at which renounce was requested.
        renounce_requested_at: Option<u64>,
    }

    #[ink(event)]
    pub struct Upgraded {
        #[ink(topic)]
        new_code_hash: Hash,
    }

    #[ink(event)]
    pub struct AdminChanged {
        #[ink(topic)]
        new_admin: AccountId,
    }

    #[ink(event)]
    pub struct AdminTransferStarted {
        #[ink(topic)]
        current_admin: AccountId,
        #[ink(topic)]
        pending_admin: AccountId,
        accept_after: u64,
    }

    #[ink(event)]
    pub struct AdminRenounceRequested {
        #[ink(topic)]
        admin: AccountId,
        execute_after: u64,
    }

    #[ink(event)]
    pub struct AdminRenounced {
        #[ink(topic)]
        old_admin: AccountId,
    }

    impl TransparentProxy {
        #[ink(constructor)]
        pub fn new(code_hash: Hash) -> Self {
            Self {
                code_hash,
                admin: Self::env().caller(),
                pending_admin: None,
                admin_transfer_requested_at: None,
                renounce_requested_at: None,
            }
        }

        #[ink(message)]
        pub fn upgrade_to(&mut self, new_code_hash: Hash) -> Result<(), Error> {
            self.ensure_admin()?;
            self.code_hash = new_code_hash;
            self.env().emit_event(Upgraded { new_code_hash });
            Ok(())
        }

        #[ink(message)]
        pub fn set_admin(&mut self, new_admin: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            if new_admin == Self::zero_account() || new_admin == self.admin {
                return Err(Error::InvalidAdmin);
            }

            let accept_after = self
                .env()
                .block_timestamp()
                .saturating_add(ADMIN_TIMELOCK_SECS);
            self.pending_admin = Some(new_admin);
            self.admin_transfer_requested_at = Some(self.env().block_timestamp());
            self.renounce_requested_at = None;
            self.env().emit_event(AdminTransferStarted {
                current_admin: self.admin,
                pending_admin: new_admin,
                accept_after,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn accept_admin(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            let pending_admin = self.pending_admin.ok_or(Error::NoPendingAdmin)?;
            if caller != pending_admin {
                return Err(Error::Unauthorized);
            }

            let requested_at = self
                .admin_transfer_requested_at
                .ok_or(Error::NoPendingAdmin)?;
            if self.env().block_timestamp() < requested_at.saturating_add(ADMIN_TIMELOCK_SECS) {
                return Err(Error::TimelockActive);
            }

            self.admin = pending_admin;
            self.pending_admin = None;
            self.admin_transfer_requested_at = None;
            self.renounce_requested_at = None;
            self.env().emit_event(AdminChanged {
                new_admin: pending_admin,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn renounce_admin(&mut self) -> Result<(), Error> {
            self.ensure_admin()?;
            if self.pending_admin.is_some() {
                return Err(Error::PendingAdminTransfer);
            }

            let now = self.env().block_timestamp();
            match self.renounce_requested_at {
                None => {
                    self.renounce_requested_at = Some(now);
                    self.env().emit_event(AdminRenounceRequested {
                        admin: self.admin,
                        execute_after: now.saturating_add(ADMIN_TIMELOCK_SECS),
                    });
                    Ok(())
                }
                Some(requested_at) => {
                    if now < requested_at.saturating_add(ADMIN_TIMELOCK_SECS) {
                        return Err(Error::TimelockActive);
                    }

                    let old_admin = self.admin;
                    self.admin = Self::zero_account();
                    self.renounce_requested_at = None;
                    self.pending_admin = None;
                    self.admin_transfer_requested_at = None;
                    self.env().emit_event(AdminChanged {
                        new_admin: self.admin,
                    });
                    self.env().emit_event(AdminRenounced { old_admin });
                    Ok(())
                }
            }
        }

        #[ink(message)]
        pub fn change_admin(&mut self, new_admin: AccountId) -> Result<(), Error> {
            self.set_admin(new_admin)
        }

        #[ink(message)]
        pub fn code_hash(&self) -> Hash {
            self.code_hash
        }

        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        #[ink(message)]
        pub fn pending_admin(&self) -> Option<AccountId> {
            self.pending_admin
        }

        #[ink(message)]
        pub fn admin_transfer_eta(&self) -> Option<u64> {
            self.admin_transfer_requested_at
                .map(|requested_at| requested_at.saturating_add(ADMIN_TIMELOCK_SECS))
        }

        #[ink(message)]
        pub fn renounce_eta(&self) -> Option<u64> {
            self.renounce_requested_at
                .map(|requested_at| requested_at.saturating_add(ADMIN_TIMELOCK_SECS))
        }

        #[ink(message)]
        pub fn admin_timelock_seconds(&self) -> u64 {
            ADMIN_TIMELOCK_SECS
        }

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                return Err(Error::Unauthorized);
            }
            Ok(())
        }

        fn zero_account() -> AccountId {
            AccountId::from([0u8; 32])
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        fn accounts() -> test::DefaultAccounts<DefaultEnvironment> {
            test::default_accounts::<DefaultEnvironment>()
        }

        fn setup_proxy() -> TransparentProxy {
            let accounts = accounts();
            test::set_caller::<DefaultEnvironment>(accounts.alice);
            test::set_block_timestamp::<DefaultEnvironment>(1_000);
            TransparentProxy::new(Hash::from([1u8; 32]))
        }

        #[ink::test]
        fn test_set_admin_starts_two_step_transfer() {
            let mut proxy = setup_proxy();
            let accounts = accounts();

            assert_eq!(proxy.admin(), accounts.alice);
            assert!(proxy.set_admin(accounts.bob).is_ok());
            assert_eq!(proxy.admin(), accounts.alice);
            assert_eq!(proxy.pending_admin(), Some(accounts.bob));
            assert_eq!(
                proxy.admin_transfer_eta(),
                Some(1_000 + proxy.admin_timelock_seconds())
            );
        }

        #[ink::test]
        fn test_accept_admin_requires_timelock_and_pending_admin() {
            let mut proxy = setup_proxy();
            let accounts = accounts();

            proxy.set_admin(accounts.bob).unwrap();

            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(proxy.accept_admin(), Err(Error::TimelockActive));

            test::set_block_timestamp::<DefaultEnvironment>(1_000 + proxy.admin_timelock_seconds());
            assert!(proxy.accept_admin().is_ok());
            assert_eq!(proxy.admin(), accounts.bob);
            assert_eq!(proxy.pending_admin(), None);

            test::set_caller::<DefaultEnvironment>(accounts.charlie);
            assert_eq!(proxy.accept_admin(), Err(Error::NoPendingAdmin));
        }

        #[ink::test]
        fn test_unauthorized_set_admin_and_invalid_target_fail() {
            let mut proxy = setup_proxy();
            let accounts = accounts();

            test::set_caller::<DefaultEnvironment>(accounts.bob);
            assert_eq!(proxy.set_admin(accounts.charlie), Err(Error::Unauthorized));

            test::set_caller::<DefaultEnvironment>(accounts.alice);
            assert_eq!(proxy.set_admin(accounts.alice), Err(Error::InvalidAdmin));
            assert_eq!(
                proxy.set_admin(AccountId::from([0u8; 32])),
                Err(Error::InvalidAdmin)
            );
        }

        #[ink::test]
        fn test_renounce_admin_is_two_step_and_timelocked() {
            let mut proxy = setup_proxy();
            let accounts = accounts();

            assert!(proxy.renounce_admin().is_ok());
            assert_eq!(
                proxy.renounce_eta(),
                Some(1_000 + proxy.admin_timelock_seconds())
            );
            assert_eq!(proxy.admin(), accounts.alice);

            assert_eq!(proxy.renounce_admin(), Err(Error::TimelockActive));

            test::set_block_timestamp::<DefaultEnvironment>(1_000 + proxy.admin_timelock_seconds());
            assert!(proxy.renounce_admin().is_ok());
            assert_eq!(proxy.admin(), AccountId::from([0u8; 32]));
            assert_eq!(proxy.renounce_eta(), None);
        }

        #[ink::test]
        fn test_pending_transfer_blocks_renounce_until_resolved() {
            let mut proxy = setup_proxy();
            let accounts = accounts();

            proxy.set_admin(accounts.bob).unwrap();
            assert_eq!(proxy.renounce_admin(), Err(Error::PendingAdminTransfer));

            test::set_caller::<DefaultEnvironment>(accounts.bob);
            test::set_block_timestamp::<DefaultEnvironment>(1_000 + proxy.admin_timelock_seconds());
            proxy.accept_admin().unwrap();

            assert!(proxy.renounce_admin().is_ok());
        }
    }
}
