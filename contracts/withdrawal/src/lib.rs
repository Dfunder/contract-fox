#![no_std]

use soroban_sdk::{Address, Env, Symbol, contract, contractimpl};

#[contract]
pub struct WithdrawalContract;

#[contractimpl]
impl WithdrawalContract {
    /// Initialize withdrawal settings
    pub fn initialize(env: Env, beneficiary: Address, max_withdrawal: i128) {
        let key = Symbol::new(&env, "settings");
        env.storage()
            .instance()
            .set(&key, &(beneficiary, max_withdrawal));
    }

    /// Withdraw funds from the contract
    pub fn withdraw(env: Env, amount: i128) -> bool {
        let key = Symbol::new(&env, "settings");
        let (beneficiary, max_withdrawal): (Address, i128) =
            env.storage().instance().get(&key).unwrap();

        beneficiary.require_auth();

        if amount > max_withdrawal {
            return false;
        }

        let withdrawn_key = Symbol::new(&env, "total_withdrawn");
        let total: i128 = env.storage().instance().get(&withdrawn_key).unwrap_or(0);
        env.storage()
            .instance()
            .set(&withdrawn_key, &(total + amount));

        true
    }

    /// Get total withdrawn
    pub fn get_total_withdrawn(env: Env) -> i128 {
        let key = Symbol::new(&env, "total_withdrawn");
        env.storage().instance().get(&key).unwrap_or(0)
    }
}
