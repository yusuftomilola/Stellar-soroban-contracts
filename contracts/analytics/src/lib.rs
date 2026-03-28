#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketMetrics {
    pub average_price: i128,
    pub total_volume: i128,
    pub properties_listed: u64,
    pub trend_factor: u32, // 100 = 1.0x
}

const METRICS: Symbol = symbol_short!("METRICS");

#[contract]
pub struct AnalyticsContract;

#[contractimpl]
impl AnalyticsContract {
    pub fn update_metrics(env: Env, average_price: i128, total_volume: i128, properties_listed: u64, trend_factor: u32) {
        let metrics = MarketMetrics {
            average_price,
            total_volume,
            properties_listed,
            trend_factor,
        };
        env.storage().instance().set(&METRICS, &metrics);
    }

    pub fn get_metrics(env: Env) -> MarketMetrics {
        env.storage().instance().get(&METRICS).unwrap_or(MarketMetrics {
            average_price: 0,
            total_volume: 0,
            properties_listed: 0,
            trend_factor: 100,
        })
    }
}
