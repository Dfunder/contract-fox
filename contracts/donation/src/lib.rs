#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

#[contract]
pub struct DonationContract;

#[contractimpl]
impl DonationContract {
    /// Initialize a donation campaign
    pub fn initialize(env: Env, campaign_id: Symbol, target_amount: i128, beneficiary: Address) {
        let key = Symbol::new(&env, "campaign");
        env.storage().instance().set(&key, &(campaign_id, target_amount, beneficiary));
    }

    /// Donate funds to the campaign
    pub fn donate(env: Env, donor: Address, amount: i128) -> i128 {
        donor.require_auth();
        
        let key = Symbol::new(&env, "total_donated");
        let total: i128 = env.storage().instance().get(&key).unwrap_or(0);
        let new_total = total + amount;
        
        env.storage().instance().set(&key, &new_total);
        new_total
    }

    /// Get total donations received
    pub fn get_total_donated(env: Env) -> i128 {
        let key = Symbol::new(&env, "total_donated");
        env.storage().instance().get(&key).unwrap_or(0)
    }
}
