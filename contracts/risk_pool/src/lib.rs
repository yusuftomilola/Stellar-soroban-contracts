#![no_std]
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

#[contracterror]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PoolError {
    ContractPaused = 1,
    Unauthorized = 2,
    InsufficientFunds = 3,
    PoolNotFound = 4,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseState {
    pub is_paused: bool,
    pub paused_at: Option<u64>,
    pub paused_by: Option<Address>,
    pub reason: Option<Symbol>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PoolEvent {
    Deposit(Address, i128),
    Withdraw(Address, i128),
    ContractPaused(Address, Option<Symbol>),
    ContractUnpaused(Address, Option<Symbol>),
}

const ADMIN: Symbol = symbol_short!("ADMIN");
const GUARDIAN: Symbol = symbol_short!("GUARDIAN");
const PAUSE_STATE: Symbol = symbol_short!("PAUSED");
const BALANCE: Symbol = symbol_short!("BALANCE");

#[contract]
pub struct RiskPoolContract;

#[contractimpl]
impl RiskPoolContract {
    pub fn initialize(env: Env, admin: Address, guardian: Address) {
        if env.storage().instance().has(&ADMIN) { panic!("Already initialized"); }
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&GUARDIAN, &guardian);
        env.storage().instance().set(&PAUSE_STATE, &PauseState { is_paused: false, paused_at: None, paused_by: None, reason: None });
        env.storage().instance().set(&BALANCE, &0i128);
    }

    pub fn set_pause_state(env: Env, caller: Address, is_paused: bool, reason: Option<Symbol>) -> Result<(), PoolError> {
        caller.require_auth();
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        let guardian: Address = env.storage().instance().get(&GUARDIAN).unwrap();

        if caller != admin && caller != guardian { return Err(PoolError::Unauthorized); }

        let pause_state = PauseState {
            is_paused,
            paused_at: if is_paused { Some(env.ledger().timestamp()) } else { None },
            paused_by: if is_paused { Some(caller.clone()) } else { None },
            reason: reason.clone(),
        };
        env.storage().instance().set(&PAUSE_STATE, &pause_state);

        if is_paused {
            env.events().publish((Symbol::short("PAUSE"), Symbol::short("PAUSED")), PoolEvent::ContractPaused(caller, reason));
        } else {
            env.events().publish((Symbol::short("PAUSE"), Symbol::short("UNPAUSED")), PoolEvent::ContractUnpaused(caller, reason));
        }
        Ok(())
    }

    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get::<_, PauseState>(&PAUSE_STATE).map(|s| s.is_paused).unwrap_or(false)
    }

    pub fn deposit(env: Env, from: Address, amount: i128) -> Result<(), PoolError> {
        if Self::is_paused(env.clone()) { return Err(PoolError::ContractPaused); }
        from.require_auth();
        let mut balance: i128 = env.storage().instance().get(&BALANCE).unwrap_or(0);
        balance += amount;
        env.storage().instance().set(&BALANCE, &balance);
        env.events().publish((BALANCE, Symbol::short("DEPOSIT")), PoolEvent::Deposit(from, amount));
        Ok(())
    }

    pub fn withdraw(env: Env, to: Address, amount: i128) -> Result<(), PoolError> {
        if Self::is_paused(env.clone()) { return Err(PoolError::ContractPaused); }
        to.require_auth();
        let mut balance: i128 = env.storage().instance().get(&BALANCE).unwrap_or(0);
        if balance < amount { return Err(PoolError::InsufficientFunds); }
        balance -= amount;
        env.storage().instance().set(&BALANCE, &balance);
        env.events().publish((BALANCE, Symbol::short("WITHDRAW")), PoolEvent::Withdraw(to, amount));
        Ok(())
    }

    pub fn get_balance(env: Env) -> i128 {
        env.storage().instance().get(&BALANCE).unwrap_or(0)
    }
}
