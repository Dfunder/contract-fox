/// Utility functions for contract operations

/// Validate amount is positive
pub fn validate_amount(amount: i128) -> bool {
    amount > 0
}

/// Calculate fee from amount (1% default)
pub fn calculate_fee(amount: i128) -> i128 {
    (amount * 1) / 100
}

/// Calculate net amount after fee
pub fn calculate_net_amount(amount: i128) -> i128 {
    amount - calculate_fee(amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_amount() {
        assert!(validate_amount(100));
        assert!(!validate_amount(0));
        assert!(!validate_amount(-100));
    }

    #[test]
    fn test_calculate_fee() {
        assert_eq!(calculate_fee(100), 1);
        assert_eq!(calculate_fee(1000), 10);
    }

    #[test]
    fn test_calculate_net_amount() {
        assert_eq!(calculate_net_amount(100), 99);
        assert_eq!(calculate_net_amount(1000), 990);
    }
}
