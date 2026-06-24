use rusqlite::{Connection, params};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

#[derive(Debug, Clone)]
pub struct NewDonation {
    pub tx_hash: String,
    pub campaign_id: String,
    pub donor_address: String,
    pub donor_user_id: Option<i64>,
    pub amount: u64,
    pub status: String,
    pub memo: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Donation {
    pub id: i64,
    pub tx_hash: String,
    pub campaign_id: String,
    pub donor_address: String,
    pub donor_user_id: Option<i64>,
    pub amount: u64,
    pub status: String,
    pub memo: Option<String>,
    pub created_at: String,
}

/// SQLite-backed donations repository.
///
/// The connection is wrapped in a [`std::sync::Mutex`] because `Connection`
/// is `!Sync` (its internal `StatementCache` uses `RefCell`). Wrapping makes
/// the repo both `Send` and `Sync`, which is what `axum` requires for
/// `AppState`. SQLite operations are brief so contention is acceptable for
/// the donation-submission throughput.
pub struct DonationsRepo {
    conn: std::sync::Mutex<Connection>,
}

impl DonationsRepo {
    pub fn new(conn: Connection) -> Result<Self, DbError> {
        // Initialise schema directly on the freshly created connection
        // (mutex not yet installed).
        conn.execute(
            "CREATE TABLE IF NOT EXISTS donations (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                tx_hash         TEXT    NOT NULL UNIQUE,
                campaign_id     TEXT    NOT NULL,
                donor_address   TEXT    NOT NULL,
                donor_user_id    INTEGER,
                amount          INTEGER NOT NULL,
                status          TEXT    NOT NULL,
                memo            TEXT,
                created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            )",
            [],
        )?;
        Ok(Self {
            conn: std::sync::Mutex::new(conn),
        })
    }

    /// Acquire the connection lock, panicking if it has been poisoned. SQLite
    /// is local and panic-free in the hot path; poisoning would only occur
    /// after another thread panicked while holding the lock.
    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("DonationsRepo lock poisoned")
    }

    /// Save a donation. Returns the existing record if `tx_hash` already exists (idempotent).
    pub fn save_donation(&self, donation: &NewDonation) -> Result<Donation, DbError> {
        let conn = self.lock();
        conn.execute(
            "INSERT OR IGNORE INTO donations
                (tx_hash, campaign_id, donor_address, donor_user_id, amount, status, memo, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            params![
                donation.tx_hash,
                donation.campaign_id,
                donation.donor_address,
                donation.donor_user_id,
                donation.amount,
                donation.status,
                donation.memo,
            ],
        )?;

        let record = conn.query_row(
            "SELECT id, tx_hash, campaign_id, donor_address, donor_user_id, amount, status, memo, created_at
             FROM donations WHERE tx_hash = ?1",
            params![donation.tx_hash],
            |row| {
                Ok(Donation {
                    id: row.get(0)?,
                    tx_hash: row.get(1)?,
                    campaign_id: row.get(2)?,
                    donor_address: row.get(3)?,
                    donor_user_id: row.get(4)?,
                    amount: row.get(5)?,
                    status: row.get(6)?,
                    memo: row.get(7)?,
                    created_at: row.get(8)?,
                })
            },
        )?;

        Ok(record)
    }

    /// Get all donations for a campaign, with anonymous display name logic.
    pub fn get_campaign_donations(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<(String, u64, String)>, DbError> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT donor_address, donor_user_id, amount, created_at
             FROM donations
             WHERE campaign_id = ?1 AND status = 'confirmed'
             ORDER BY created_at DESC",
        )?;

        let donations = stmt.query_map(params![campaign_id], |row| {
            let donor_address: String = row.get(0)?;
            let donor_user_id: Option<i64> = row.get(1)?;
            let amount: u64 = row.get(2)?;
            let created_at: String = row.get(3)?;

            // If no user is linked, display as "Anonymous Donor".
            let display_name = if donor_user_id.is_none() {
                "Anonymous Donor".to_string()
            } else {
                donor_address
            };

            Ok((display_name, amount, created_at))
        })?;

        let mut results = Vec::new();
        for donation in donations {
            results.push(donation?);
        }

        Ok(results)
    }

    /// Mark a donation as refunded in the database.
    ///
    /// Off-chain indexers should call this when they observe a
    /// `DonationRefunded` event from the contract, passing the
    /// `original_tx_hash` field of that event.
    ///
    /// Returns `true` if a donation with the given `tx_hash` was
    /// found and updated to status `refunded`, `false` if no
    /// matching donation existed.
    pub fn mark_refunded(&self, tx_hash: &str) -> Result<bool, DbError> {
        let conn = self.lock();
        let updated = conn.execute(
            "UPDATE donations SET status = 'refunded' WHERE tx_hash = ?1",
            params![tx_hash],
        )?;
        Ok(updated > 0)
    }

    /// Get campaign stats (total raised, donation count)
    /// Get campaign stats (total raised, donation count).
    pub fn get_campaign_stats(&self, campaign_id: &str) -> Result<(u64, u64), DbError> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT COALESCE(SUM(amount), 0) as total_raised, COUNT(*) as donation_count
             FROM donations
             WHERE campaign_id = ?1 AND status = 'confirmed'",
        )?;

        let (total_raised, donation_count) = stmt.query_row(params![campaign_id], |row| {
            Ok((row.get::<_, u64>(0)?, row.get::<_, u64>(1)?))
        })?;

        Ok((total_raised, donation_count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory_repo() -> DonationsRepo {
        DonationsRepo::new(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn sample() -> NewDonation {
        NewDonation {
            tx_hash: "abc123".to_string(),
            campaign_id: "campaign-1".to_string(),
            donor_address: "GABC...".to_string(),
            donor_user_id: Some(1),
            amount: 1000,
            status: "confirmed".to_string(),
            memo: None,
        }
    }

    #[test]
    fn saves_anonymous_donation() {
        let repo = in_memory_repo();
        let anonymous_donation = NewDonation {
            tx_hash: "anonymous456".to_string(),
            campaign_id: "campaign-1".to_string(),
            donor_address: "GANON...".to_string(),
            donor_user_id: None,
            amount: 500,
            status: "confirmed".to_string(),
            memo: None,
        };
        let saved = repo.save_donation(&anonymous_donation).unwrap();
        assert_eq!(saved.tx_hash, "anonymous456");
        assert_eq!(saved.donor_user_id, None);
        assert_eq!(saved.amount, 500);
    }

    #[test]
    fn get_campaign_donations_displays_anonymous() {
        let repo = in_memory_repo();
        repo.save_donation(&sample()).unwrap();
        let anonymous_donation = NewDonation {
            tx_hash: "anonymous456".to_string(),
            campaign_id: "campaign-1".to_string(),
            donor_address: "GANONXYZ...".to_string(),
            donor_user_id: None,
            amount: 500,
            status: "confirmed".to_string(),
            memo: None,
        };
        repo.save_donation(&anonymous_donation).unwrap();

        let donations = repo.get_campaign_donations("campaign-1").unwrap();
        assert_eq!(donations.len(), 2);

        let has_anonymous = donations
            .iter()
            .any(|(name, amount, _)| name == "Anonymous Donor" && *amount == 500);
        let has_registered = donations
            .iter()
            .any(|(name, amount, _)| name == "GABC..." && *amount == 1000);

        assert!(has_anonymous, "Anonymous donation not found");
        assert!(has_registered, "Registered user donation not found");
    }

    #[test]
    fn get_campaign_stats_calculates_correctly() {
        let repo = in_memory_repo();
        repo.save_donation(&sample()).unwrap();
        let anonymous_donation = NewDonation {
            tx_hash: "anonymous456".to_string(),
            campaign_id: "campaign-1".to_string(),
            donor_address: "GANONXYZ...".to_string(),
            donor_user_id: None,
            amount: 500,
            status: "confirmed".to_string(),
            memo: None,
        };
        repo.save_donation(&anonymous_donation).unwrap();

        let (total_raised, donation_count) = repo.get_campaign_stats("campaign-1").unwrap();
        assert_eq!(total_raised, 1500);
        assert_eq!(donation_count, 2);
    }

    #[test]
    fn saves_new_donation() {
        let repo = in_memory_repo();
        let saved = repo.save_donation(&sample()).unwrap();
        assert_eq!(saved.tx_hash, "abc123");
        assert_eq!(saved.amount, 1000);
        assert_eq!(saved.status, "confirmed");
        assert_eq!(saved.memo, None);
    }

    #[test]
    fn saves_donation_with_memo() {
        let repo = in_memory_repo();
        let d = NewDonation {
            memo: Some("for my friend".to_string()),
            ..sample()
        };
        let saved = repo.save_donation(&d).unwrap();
        assert_eq!(saved.memo, Some("for my friend".to_string()));
    }

    #[test]
    fn duplicate_tx_hash_returns_existing() {
        let repo = in_memory_repo();
        let first = repo.save_donation(&sample()).unwrap();
        let second = repo.save_donation(&sample()).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(first.tx_hash, second.tx_hash);
    }

    #[test]
    fn different_tx_hashes_are_independent() {
        let repo = in_memory_repo();
        let a = repo.save_donation(&sample()).unwrap();
        let b = repo
            .save_donation(&NewDonation {
                tx_hash: "xyz789".to_string(),
                ..sample()
            })
            .unwrap();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn mark_refunded_updates_status_and_returns_true() {
        let repo = in_memory_repo();
        let saved = repo.save_donation(&sample()).unwrap();
        assert_eq!(saved.status, "confirmed");

        assert!(repo.mark_refunded(&saved.tx_hash).unwrap());

        // Refetch (via idempotent save_donation) and confirm status changed.
        let refetched = repo
            .save_donation(&NewDonation {
                tx_hash: saved.tx_hash.clone(),
                ..sample()
            })
            .unwrap();
        assert_eq!(refetched.status, "refunded");

        // Stats should now exclude the refunded donation.
        let (total_raised, count) = repo.get_campaign_stats(&saved.campaign_id).unwrap();
        assert_eq!(total_raised, 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn mark_refunded_returns_false_when_donation_missing() {
        let repo = in_memory_repo();
        assert!(!repo.mark_refunded("nonexistent_tx_hash").unwrap());
    }

    #[test]
    fn mark_refunded_only_affects_targeted_donation() {
        let repo = in_memory_repo();
        let a = repo.save_donation(&sample()).unwrap();
        let b = repo
            .save_donation(&NewDonation {
                tx_hash: "tx_b".to_string(),
                ..sample()
            })
            .unwrap();

        repo.mark_refunded(&a.tx_hash).unwrap();

        let a_refetch = repo
            .save_donation(&NewDonation {
                tx_hash: a.tx_hash.clone(),
                ..sample()
            })
            .unwrap();
        let b_refetch = repo
            .save_donation(&NewDonation {
                tx_hash: b.tx_hash.clone(),
                ..sample()
            })
            .unwrap();
        assert_eq!(a_refetch.status, "refunded");
        assert_eq!(b_refetch.status, "confirmed");
    }

    /// The repo must be `Send + Sync` so it can live inside an `AppState`
    /// shared across axum worker tasks.
    #[test]
    fn repo_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DonationsRepo>();
    }
}
