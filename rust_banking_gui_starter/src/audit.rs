use chrono::Utc;
use rusqlite::{params, Connection, Result};
use sha2::{Digest, Sha256};

use crate::models::AuditRecord;

pub fn generate_hash(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

pub fn log_action(conn: &Connection, user_id: Option<i32>, action: &str, details: &str) -> Result<()> {
    let prev_hash: String = conn
        .query_row(
            "SELECT current_hash FROM audit_log ORDER BY log_id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "GENESIS".to_string());

    let created_at = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let raw_data = format!(
        "{}|{}|{}|{}|{}",
        user_id.map(|v| v.to_string()).unwrap_or_else(|| "NULL".to_string()),
        action,
        details,
        created_at,
        prev_hash
    );
    let current_hash = generate_hash(&raw_data);

    conn.execute(
        "INSERT INTO audit_log (user_id, action, details, created_at, prev_hash, current_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![user_id, action, details, created_at, prev_hash, current_hash],
    )?;

    Ok(())
}

pub fn list_audit_logs(conn: &Connection, limit: i32) -> Result<Vec<AuditRecord>> {
    let mut stmt = conn.prepare(
        "SELECT log_id, user_id, action, details, created_at, prev_hash, current_hash
         FROM audit_log
         ORDER BY log_id DESC
         LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit], |row| {
        Ok(AuditRecord {
            log_id: row.get(0)?,
            user_id: row.get(1)?,
            action: row.get(2)?,
            details: row.get(3)?,
            created_at: row.get(4)?,
            prev_hash: row.get(5)?,
            current_hash: row.get(6)?,
        })
    })?;

    rows.collect()
}
