#![no_std]

use soroban_sdk::{contract, contractimpl, token, Address, Env, Symbol};

#[contract]
pub struct WithdrawalContract;

#[contractimpl]
impl WithdrawalContract {
    /// Initialize withdrawal settings
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `beneficiary` - The address authorised to withdraw
    /// * `max_withdrawal` - Maximum amount allowed per withdrawal call
    /// * `token_id` - Address of the XLM token contract (wrapped native)
    pub fn initialize(env: Env, beneficiary: Address, max_withdrawal: i128, token_id: Address) {
        let key = Symbol::new(&env, "settings");
        env.storage()
            .instance()
            .set(&key, &(beneficiary, max_withdrawal, token_id));
    }

    /// Withdraw XLM from the contract to the beneficiary
    ///
    /// Verifies the contract holds sufficient balance before transferring.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `amount` - Amount to withdraw
    ///
    /// # Returns
    /// `true` on success, `false` if amount exceeds `max_withdrawal`
    pub fn withdraw(env: Env, amount: i128) -> bool {
        let key = Symbol::new(&env, "settings");
        let (beneficiary, max_withdrawal, token_id): (Address, i128, Address) = env
            .storage()
            .instance()
            .get(&key)
            .expect("withdrawal not initialized");

        beneficiary.require_auth();

        if amount > max_withdrawal {
            return false;
        }

        // Verify the contract holds enough tokens before transferring
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &token_id);
        let balance = token_client.balance(&contract_address);
        if balance < amount {
            panic!("Insufficient contract balance");
        }

        // Transfer tokens from this contract to the beneficiary
        token_client.transfer(&contract_address, &beneficiary, &amount);

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

#[cfg(test)]
mod test {
    use soroban_sdk::{
        testutils::Address as _,
        token::{Client as TokenClient, StellarAssetClient},
        Address, Env,
    };
    use crate::{WithdrawalContract, WithdrawalContractClient};

    #[test]
    fn test_withdraw_success() {
        let env = Env::default();
        env.mock_all_auths();

        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract(token_admin.clone());

        let contract_id = env.register_contract(None, WithdrawalContract);
        let client = WithdrawalContractClient::new(&env, &contract_id);

        client.initialize(&beneficiary, &500i128, &token_id);

        // Fund the withdrawal contract
        StellarAssetClient::new(&env, &token_id).mint(&contract_id, &500i128);

        let result = client.withdraw(&200i128);
        assert!(result);
        assert_eq!(client.get_total_withdrawn(), 200i128);

        // Verify beneficiary received tokens
        let beneficiary_balance = TokenClient::new(&env, &token_id).balance(&beneficiary);
        assert_eq!(beneficiary_balance, 200i128);

        // Verify contract balance decreased
        let contract_balance = TokenClient::new(&env, &token_id).balance(&contract_id);
        assert_eq!(contract_balance, 300i128);
    }

    #[test]
    fn test_withdraw_exceeds_max() {
        let env = Env::default();
        env.mock_all_auths();

        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract(token_admin.clone());

        let contract_id = env.register_contract(None, WithdrawalContract);
        let client = WithdrawalContractClient::new(&env, &contract_id);

        client.initialize(&beneficiary, &100i128, &token_id);
        StellarAssetClient::new(&env, &token_id).mint(&contract_id, &500i128);

        let result = client.withdraw(&200i128);
        assert!(!result);
        assert_eq!(client.get_total_withdrawn(), 0i128);
    }

    #[test]
    #[should_panic(expected = "Insufficient contract balance")]
    fn test_withdraw_insufficient_balance() {
        let env = Env::default();
        env.mock_all_auths();

        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract(token_admin.clone());

        let contract_id = env.register_contract(None, WithdrawalContract);
        let client = WithdrawalContractClient::new(&env, &contract_id);

        client.initialize(&beneficiary, &1000i128, &token_id);
        // Mint less than what we'll try to withdraw
        StellarAssetClient::new(&env, &token_id).mint(&contract_id, &50i128);

        client.withdraw(&100i128);
    }

    #[test]
    #[should_panic(expected = "withdrawal not initialized")]
    fn test_withdraw_without_initialization() {
        let env = Env::default();
        let contract_id = env.register_contract(None, WithdrawalContract);
        let client = WithdrawalContractClient::new(&env, &contract_id);
        client.withdraw(&100i128);
    }
}
