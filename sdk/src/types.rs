use soroban_sdk::{Address, Symbol};

/// Campaign information structure
#[derive(Clone, Debug)]
pub struct Campaign {
    pub id: Symbol,
    pub title: Symbol,
    pub target_amount: i128,
    pub deadline: u64,
}

/// Donation record structure
#[derive(Clone, Debug)]
pub struct Donation {
    pub donor: Address,
    pub amount: i128,
    pub campaign_id: Symbol,
}
