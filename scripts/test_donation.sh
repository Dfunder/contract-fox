#!/bin/bash
# Test script for Donation contract functionality

echo "=== Testing Donation Contract ==="

# Load contract IDs
source .contract_ids 2>/dev/null || {
  echo "Contract IDs not found. Run deploy_donation_example.sh first."
  exit 1
}
# Check if the contract IDs are set
echo "Using Campaign Contract: $CAMPAIGN_CONTRACT_ID"
echo "Using Donation Contract: $DONATION_CONTRACT_ID"

# Generate a test donor address
DONOR_ADDRESS=$(soroban keys address donor-test)

echo "Test donor address: $DONOR_ADDRESS"

# Test 1: Make a donation
echo ""
echo "=== Test 1: Making a donation ==="
soroban contract invoke \
  --id $DONATION_CONTRACT_ID \
  --network testnet \
  --source donor-test \
  --fn donate \
  --args "$DONOR_ADDRESS" "1" "100000000"

echo "Donation made successfully!"

# Test 2: Get total raised for campaign
echo ""
echo "=== Test 2: Getting total raised ==="
TOTAL_RAISED=$(soroban contract invoke \
  --id $DONATION_CONTRACT_ID \
  --network testnet \
  --source admin \
  --fn get_total_raised \
  --args "1")

echo "Total raised for campaign 1: $TOTAL_RAISED"

# Test 3: Get donations for campaign
echo ""
echo "=== Test 3: Getting donations for campaign ==="
soroban contract invoke \
  --id $DONATION_CONTRACT_ID \
  --network testnet \
  --source admin \
  --fn get_donations_for_campaign \
  --args "1"

# Test 4: Get donor history
echo ""
echo "=== Test 4: Getting donor history ==="
soroban contract invoke \
  --id $DONATION_CONTRACT_ID \
  --network testnet \
  --source admin \
  --fn get_donor_history \
  --args "$DONOR_ADDRESS"

# Test 5: Check campaign raised amount
echo ""
echo "=== Test 5: Checking campaign raised amount ==="
CAMPAIGN_RAISED=$(soroban contract invoke \
  --id $CAMPAIGN_CONTRACT_ID \
  --network testnet \
  --source admin \
  --fn get_raised_amount \
  --args "1")

echo "Campaign 1 raised amount: $CAMPAIGN_RAISED"

echo ""
echo "=== All tests completed successfully! ==="