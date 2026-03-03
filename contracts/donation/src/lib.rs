//! Donation Contract
//!
//! This contract handles donation logic for the StellarAid platform.
//! Allows donors to contribute XLM or other Stellar assets to campaigns.

use soroban_sdk::{Env, Symbol, contract, contractimpl};

#[contract]
pub struct DonationContract;

#[contractimpl]
impl DonationContract {
    /// Initialize the donation contract
    pub fn initialize(env: Env) {
        // Store initialization state
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "init"), &true);
    }

    /// Ping the contract to verify it's alive
    pub fn ping(_env: Env) -> u32 {
        1
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_ping() {
        let env = Env::default();
        let contract_id = env.register(DonationContract, ());
        let client = DonationContractClient::new(&env, &contract_id);

        assert_eq!(client.ping(), 1);
    }
}
