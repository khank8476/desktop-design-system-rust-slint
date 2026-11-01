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

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS preferences (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
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

    /// Delete multiple records by IDs in an atomic transaction (Batch Delete)
    pub fn delete_multiple(&self, ids: &[i64]) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("DELETE FROM services WHERE id IN ({})", placeholders);
        let params: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        let affected = self.conn.execute(&query, rusqlite::params_from_iter(params))?;
        Ok(affected)
    }

    /// Update multiple records by IDs with selective field overrides (Batch Update)
    pub fn update_multiple(
        &self,
        ids: &[i64],
        service: Option<&str>,
        runtime: Option<&str>,
        status: Option<&str>,
        latency: Option<&str>,
    ) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut set_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(s) = service {
            set_clauses.push(format!("service = ?{}", params.len() + 1));
            params.push(Box::new(s.to_string()));
        }
        if let Some(r) = runtime {
            set_clauses.push(format!("runtime = ?{}", params.len() + 1));
            params.push(Box::new(r.to_string()));
        }
        if let Some(st) = status {
            set_clauses.push(format!("status = ?{}", params.len() + 1));
            params.push(Box::new(st.to_string()));
        }
        if let Some(l) = latency {
            set_clauses.push(format!("latency = ?{}", params.len() + 1));
            params.push(Box::new(l.to_string()));
        }

        if set_clauses.is_empty() {
            return Ok(0);
        }

        let in_start = params.len() + 1;
        let mut in_clauses = Vec::new();
        for (i, id) in ids.iter().enumerate() {
            in_clauses.push(format!("?{}", in_start + i));
            params.push(Box::new(*id));
        }

        let sql = format!(
            "UPDATE services SET {} WHERE id IN ({})",
            set_clauses.join(", "),
            in_clauses.join(", ")
        );

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let affected = self.conn.execute(&sql, rusqlite::params_from_iter(param_refs))?;
        Ok(affected)
    }

    /// Read a user preference by key
    pub fn get_preference(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM preferences WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Set or update a user preference (Upsert)
    pub fn set_preference(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO preferences (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
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

    #[test]
    fn test_preference_persistence() {
        let db = DatabaseManager::new_in_memory().expect("In-memory DB init failed");

        // Initially empty
        assert_eq!(db.get_preference("sidebar_collapsed").unwrap(), None);

        // Set preference
        db.set_preference("sidebar_collapsed", "true").unwrap();
        assert_eq!(db.get_preference("sidebar_collapsed").unwrap(), Some("true".to_string()));

        // Update preference
        db.set_preference("sidebar_collapsed", "false").unwrap();
        assert_eq!(db.get_preference("sidebar_collapsed").unwrap(), Some("false".to_string()));

        // Display mode (theme) preference persistence
        assert_eq!(db.get_preference("theme_mode").unwrap(), None);
        db.set_preference("theme_mode", "dark").unwrap();
        assert_eq!(db.get_preference("theme_mode").unwrap(), Some("dark".to_string()));
        db.set_preference("theme_mode", "light").unwrap();
        assert_eq!(db.get_preference("theme_mode").unwrap(), Some("light".to_string()));
        db.set_preference("theme_mode", "sepia").unwrap();
        assert_eq!(db.get_preference("theme_mode").unwrap(), Some("sepia".to_string()));
        db.set_preference("theme_mode", "system").unwrap();
        assert_eq!(db.get_preference("theme_mode").unwrap(), Some("system".to_string()));
    }

    #[test]
    fn test_batch_operations() {
        let db = DatabaseManager::new_in_memory().expect("In-memory DB init failed");
        let initial = db.get_all().expect("Query failed");
        assert_eq!(initial.len(), 5);

        let ids: Vec<i64> = initial.iter().take(3).map(|r| r.id).collect();
        assert_eq!(ids.len(), 3);

        // 1. Batch Update Status and Runtime
        let updated_count = db.update_multiple(
            &ids,
            None,
            Some("Batch Modern Runtime"),
            Some("Offline"),
            Some("0.1 ms"),
        ).expect("Batch update failed");
        assert_eq!(updated_count, 3);

        let records = db.get_all().expect("Query failed");
        for r in records.iter().filter(|r| ids.contains(&r.id)) {
            assert_eq!(r.runtime, "Batch Modern Runtime");
            assert_eq!(r.status, "Offline");
            assert_eq!(r.latency, "0.1 ms");
        }

        // 2. Batch Delete
        let deleted_count = db.delete_multiple(&ids).expect("Batch delete failed");
        assert_eq!(deleted_count, 3);

        let remaining = db.get_all().expect("Query failed");
        assert_eq!(remaining.len(), 2);
        for id in ids {
            assert!(remaining.iter().all(|r| r.id != id));
        }
    }
}
