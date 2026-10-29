use rusqlite::{params, Connection, Result};

#[derive(Debug, Clone)]
pub struct ServiceRecord {
    pub id: i64,
    pub service: String,
    pub runtime: String,
    pub status: String,
    pub latency: String,
    pub created_at: String,
}

pub struct DatabaseManager {
    conn: Connection,
}

impl DatabaseManager {
    /// Initialize local database connection with Write-Ahead Logging (WAL) for maximum concurrency
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        // WAL mode enables concurrent reads without blocking writes and sub-millisecond commits
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let manager = Self { conn };
        manager.create_tables()?;
        manager.seed_initial_data_if_empty()?;
        Ok(manager)
    }

    /// In-memory database initialization for isolated fast testing
    #[allow(dead_code)]
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let manager = Self { conn };
        manager.create_tables()?;
        manager.seed_initial_data_if_empty()?;
        Ok(manager)
    }

    fn create_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS services (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                service TEXT NOT NULL,
                runtime TEXT NOT NULL,
                status TEXT NOT NULL,
                latency TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    fn seed_initial_data_if_empty(&self) -> Result<()> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM services", [], |row| row.get(0))?;

        if count == 0 {
            let defaults = [
                ("Direct3D Render Pipeline", "DirectX 12 / HLSL", "Active", "1.1 ms"),
                ("Metal Swapchain Engine", "Apple Metal 3.0", "Active", "0.8 ms"),
                ("Vulkan Surface Compositor", "SPIR-V Cross-Platform", "Standby", "2.4 ms"),
                ("Audio Subsystem Streamer", "WASAPI / CoreAudio", "Warning", "12.8 ms"),
                ("Telemetry Event Ingestion", "Async Native Rust Channel", "Active", "0.4 ms"),
            ];

            let now = chrono_now_str();
            for (service, runtime, status, latency) in defaults {
                self.conn.execute(
                    "INSERT INTO services (service, runtime, status, latency, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![service, runtime, status, latency, now],
                )?;
            }
        }
        Ok(())
    }

    /// Read all services ordered by ID
    pub fn get_all(&self) -> Result<Vec<ServiceRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, service, runtime, status, latency, created_at FROM services ORDER BY id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(ServiceRecord {
                id: row.get(0)?,
                service: row.get(1)?,
                runtime: row.get(2)?,
                status: row.get(3)?,
                latency: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Insert a new service record (Create)
    pub fn insert(
        &self,
        service: &str,
        runtime: &str,
        status: &str,
        latency: &str,
    ) -> Result<ServiceRecord> {
        let now = chrono_now_str();
        self.conn.execute(
            "INSERT INTO services (service, runtime, status, latency, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![service, runtime, status, latency, now],
        )?;

        let id = self.conn.last_insert_rowid();
        Ok(ServiceRecord {
            id,
            service: service.to_string(),
            runtime: runtime.to_string(),
            status: status.to_string(),
            latency: latency.to_string(),
            created_at: now,
        })
    }

    /// Update an existing record (Update)
    pub fn update(
        &self,
        id: i64,
        service: &str,
        runtime: &str,
        status: &str,
        latency: &str,
    ) -> Result<bool> {
        let affected = self.conn.execute(
            "UPDATE services SET service = ?1, runtime = ?2, status = ?3, latency = ?4
             WHERE id = ?5",
            params![service, runtime, status, latency, id],
        )?;
        Ok(affected > 0)
    }

    /// Delete a record by ID (Delete)
    pub fn delete(&self, id: i64) -> Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM services WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}

/// Helper function to generate clean ISO-like timestamps without pulling heavy chrono crate
fn chrono_now_str() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (now / 3600) % 24;
    let mins = (now / 60) % 60;
    let secs = now % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crud_lifecycle() {
        let db = DatabaseManager::new_in_memory().expect("In-memory DB init failed");
        
        // 1. Verify initial seed
        let initial = db.get_all().expect("Query failed");
        assert_eq!(initial.len(), 5);

        // 2. Insert (Create)
        let inserted = db.insert("Test Pipeline", "Rust Tokio", "Active", "0.2 ms").expect("Insert failed");
        assert!(inserted.id > 0);
        assert_eq!(inserted.service, "Test Pipeline");

        let after_insert = db.get_all().expect("Query failed");
        assert_eq!(after_insert.len(), 6);

        // 3. Update (Update)
        let updated = db.update(inserted.id, "Test Pipeline V2", "Rust Rayon", "Standby", "0.5 ms").expect("Update failed");
        assert!(updated);

        let records = db.get_all().expect("Query failed");
        let rec = records.iter().find(|r| r.id == inserted.id).expect("Record not found");
        assert_eq!(rec.service, "Test Pipeline V2");
        assert_eq!(rec.status, "Standby");

        // 4. Delete (Delete)
        let deleted = db.delete(inserted.id).expect("Delete failed");
        assert!(deleted);

        let after_delete = db.get_all().expect("Query failed");
        assert_eq!(after_delete.len(), 5);
        assert!(after_delete.iter().all(|r| r.id != inserted.id));
    }
}
