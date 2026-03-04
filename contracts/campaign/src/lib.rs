#![no_std]

use soroban_sdk::{Address, Env, Symbol, contract, contractimpl};

#[contract]
pub struct CampaignContract;

#[contractimpl]
impl CampaignContract {
    /// Create a new campaign
    pub fn create(env: Env, campaign_id: Symbol, title: Symbol, target: i128, deadline: u64) {
        let key = Symbol::new(&env, "campaign_data");
        env.storage()
            .instance()
            .set(&key, &(campaign_id, title, target, deadline));
    }

    /// Get campaign status
    pub fn get_status(env: Env) -> (Symbol, Symbol, i128, u64) {
        let key = Symbol::new(&env, "campaign_data");
        env.storage().instance().get(&key).unwrap()
    }

    /// Check if campaign is active
    pub fn is_active(env: Env) -> bool {
        let key = Symbol::new(&env, "campaign_data");
        let (_id, _title, _target, deadline): (Symbol, Symbol, i128, u64) =
            env.storage().instance().get(&key).unwrap();

        let current_time = env.ledger().timestamp();
        current_time < deadline
    }
}
