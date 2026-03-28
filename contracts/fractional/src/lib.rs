#![no_std]
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec};

#[contracterror]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FractionalError {
    ContractPaused = 1,
    Unauthorized = 2,
    TokenNotFound = 3,
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
pub enum FractionalEvent {
    PriceUpdated(u64, i128),
    ContractPaused(Address, Option<Symbol>),
    ContractUnpaused(Address, Option<Symbol>),
}

const ADMIN: Symbol = symbol_short!("ADMIN");
const GUARDIAN: Symbol = symbol_short!("GUARDIAN");
const PAUSE_STATE: Symbol = symbol_short!("PAUSED");
const PRICES: Symbol = symbol_short!("PRICES");

#[contract]
pub struct FractionalContract;

#[contractimpl]
impl FractionalContract {
    pub fn initialize(env: Env, admin: Address, guardian: Address) {
        if env.storage().instance().has(&ADMIN) { panic!("Already initialized"); }
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&GUARDIAN, &guardian);
        env.storage().instance().set(&PAUSE_STATE, &PauseState { is_paused: false, paused_at: None, paused_by: None, reason: None });
    }

    pub fn set_pause_state(env: Env, caller: Address, is_paused: bool, reason: Option<Symbol>) -> Result<(), FractionalError> {
        caller.require_auth();
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        let guardian: Address = env.storage().instance().get(&GUARDIAN).unwrap();

        if caller != admin && caller != guardian { return Err(FractionalError::Unauthorized); }

        let pause_state = PauseState {
            is_paused,
            paused_at: if is_paused { Some(env.ledger().timestamp()) } else { None },
            paused_by: if is_paused { Some(caller.clone()) } else { None },
            reason: reason.clone(),
        };
        env.storage().instance().set(&PAUSE_STATE, &pause_state);

        if is_paused {
            env.events().publish((Symbol::short("PAUSE"), Symbol::short("PAUSED")), FractionalEvent::ContractPaused(caller, reason));
        } else {
            env.events().publish((Symbol::short("PAUSE"), Symbol::short("UNPAUSED")), FractionalEvent::ContractUnpaused(caller, reason));
        }
        Ok(())
    }

    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get::<_, PauseState>(&PAUSE_STATE).map(|s| s.is_paused).unwrap_or(false)
    }

    pub fn set_price(env: Env, token_id: u64, price: i128) -> Result<(), FractionalError> {
        if Self::is_paused(env.clone()) { return Err(FractionalError::ContractPaused); }
        let key = (PRICES, token_id);
        env.storage().persistent().set(&key, &price);
        env.events().publish((PRICES, Symbol::short("UPDATE")), FractionalEvent::PriceUpdated(token_id, price));
        Ok(())
    }

    pub fn get_price(env: Env, token_id: u64) -> Option<i128> {
        let key = (PRICES, token_id);
        env.storage().persistent().get(&key)
    }
}
