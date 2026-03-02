//! StellarAid SDK
//! 
//! This SDK provides utilities and types for interacting with the 
//! StellarAid smart contracts.

use soroban_sdk::{contracttype, Address, String};

/// Represents a campaign in the StellarAid platform
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Campaign {
    pub id: u64,
    pub creator: Address,
    pub title: String,
    pub description: String,
    pub goal_amount: i128,
    pub raised_amount: i128,
    pub status: CampaignStatus,
}

/// Status of a campaign
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CampaignStatus {
    Active,
    Completed,
    Cancelled,
}

/// Represents a donation
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Donation {
    pub donor: Address,
    pub campaign_id: u64,
    pub amount: i128,
    pub asset: Asset,
}

/// Supported asset types
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Asset {
    Native, // XLM
    Token(Address), // Custom token contract address
}

/// Error types for the StellarAid platform
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    NotFound,
    Unauthorized,
    InvalidAmount,
    CampaignNotActive,
    InsufficientFunds,
}
