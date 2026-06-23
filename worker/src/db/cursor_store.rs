use rusqlite::{Connection, params};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CursorStoreError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

#[derive(Debug)]
pub struct CursorRecord {
    pub address: String,
    pub cursor: String,
    pub updated_at: String,
}

pub struct CursorStore {
    conn: Connection,
}

impl CursorStore {
    pub fn new(conn: Connection) -> Result<Self, CursorStoreError> {
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<(), CursorStoreError> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS stellar_cursors (
                address         TEXT    NOT NULL PRIMARY KEY,
                cursor          TEXT    NOT NULL,
                updated_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            )",
            [],
        )?;
        Ok(())
    }

    /// Get the last saved cursor for an address. If the address is not tracked yet,
    /// initialize it with "now" and return that.
    pub fn get_cursor(&self, address: &str) -> Result<String, CursorStoreError> {
        // Try to get existing cursor
        let existing = self.conn.query_row(
            "SELECT cursor FROM stellar_cursors WHERE address = ?1",
            params![address],
            |row| row.get::<_, String>(0),
        );

        match existing {
            Ok(cursor) => Ok(cursor),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // New address - initialize with "now"
                self.save_cursor(address, "now")?;
                Ok("now".to_string())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Save the latest cursor for an address
    pub fn save_cursor(&self, address: &str, cursor: &str) -> Result<(), CursorStoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO stellar_cursors (address, cursor, updated_at)
             VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            params![address, cursor],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn in_memory_store() -> CursorStore {
        CursorStore::new(Connection::open_in_memory().unwrap()).unwrap()
    }

    #[test]
    fn initializes_new_address_with_now() {
        let store = in_memory_store();
        let cursor = store.get_cursor("GA...TEST").unwrap();
        assert_eq!(cursor, "now");
    }

    #[test]
    fn saves_and_retrieves_cursor() {
        let store = in_memory_store();
        let test_address = "GA...TEST123";
        let test_cursor = "123456789";
        
        store.save_cursor(test_address, test_cursor).unwrap();
        let retrieved = store.get_cursor(test_address).unwrap();
        assert_eq!(retrieved, test_cursor);
    }

    #[test]
    fn updates_cursor() {
        let store = in_memory_store();
        let test_address = "GA...UPDATE";
        
        store.save_cursor(test_address, "old_cursor").unwrap();
        store.save_cursor(test_address, "new_cursor").unwrap();
        
        let retrieved = store.get_cursor(test_address).unwrap();
        assert_eq!(retrieved, "new_cursor");
    }
}